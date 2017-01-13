#!/usr/bin/env python3

""" Dynamically control fan speed according to hard drive temperature. """

__version__ = "1.2.5"
__author__ = "desbma"
__license__ = "GPLv3"

import argparse
import contextlib
import enum
import itertools
import logging
import logging.handlers
import operator
import os
import re
import shutil
import signal
import socket
import stat
import subprocess
import syslog
import threading
import time

import daemon
import daemon.pidfile

from hddfancontrol import bin_dep
from hddfancontrol import colored_logging


exit_evt = threading.Event()


class LoggingSysLogHandler(logging.Handler):

  """ Class similar in goal to logging.handlers.SysLogHandler, but uses the syslog call instead of socket. """

  def __init__(self, facility, options=syslog.LOG_PID):
    syslog.openlog(logoption=options, facility=facility)
    super().__init__()

  def emit(self, record):
    """ See logging.Handler.emit. """
    msg = self.format(record)
    h = logging.handlers.SysLogHandler
    level = h.priority_names[h.priority_map[record.levelname]]
    syslog.syslog(level, msg)

  def close(self):
    """ See logging.Handler.close. """
    syslog.closelog()
    super().close()


class Drive:

  """ Drive represented by a device file like /dev/sdX. """

  DriveState = enum.Enum("DriveState", ("UNKNOWN", "ACTIVE_IDLE", "STANDBY", "SLEEPING"))

  HDPARM_GET_TEMP_HITACHI_REGEX = re.compile("drive temperature \(celsius\) is:\s*([0-9]*)")
  HDPARM_GET_MODEL_REGEX = re.compile("Model Number:\s*(.*)")

  def __init__(self, device_filepath, hddtemp_daemon_port):
    assert(stat.S_ISBLK(os.stat(device_filepath).st_mode))
    self.device_filepath = __class__.normalizeDrivePath(device_filepath)
    self.stat_filepath = "/sys/block/%s/stat" % (os.path.basename(self.device_filepath))
    self.hddtemp_daemon_port = hddtemp_daemon_port
    self.pretty_name = self.getPrettyName()
    self.logger = logging.getLogger(str(self))
    self.supports_hitachi_temp_query = self.supportsHitachiTempQuery()

  def __str__(self):
    """ Return a pretty drive name. """
    return self.pretty_name

  def getPrettyName(self):
    """ Return a pretty drive name. """
    # get device metadata to grab model string
    cmd = ("hdparm", "-I", self.device_filepath)
    output = subprocess.check_output(cmd,
                                     stdin=subprocess.DEVNULL,
                                     stderr=subprocess.DEVNULL,
                                     universal_newlines=True)
    model = __class__.HDPARM_GET_MODEL_REGEX.search(output).group(1).strip()
    return "%s %s" % (os.path.basename(self.device_filepath), model)

  def supportsHitachiTempQuery(self):
    # test if drive supports hdparm -H
    cmd = ("hdparm", "-H", self.device_filepath)
    try:
      subprocess.check_call(cmd,
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
      self.logger.warning("Drive does not allow querying temperature without going out of low power mode.")
      return False
    return True

  def getState(self):
    """ Get drive power state, as a DriveState enum. """
    states = {"unknown": __class__.DriveState.UNKNOWN,
              "active/idle": __class__.DriveState.ACTIVE_IDLE,
              "standby": __class__.DriveState.STANDBY,
              "sleeping": __class__.DriveState.SLEEPING}
    cmd = ("hdparm", "-C", self.device_filepath)
    output = subprocess.check_output(cmd,
                                     stdin=subprocess.DEVNULL,
                                     stderr=subprocess.DEVNULL,
                                     universal_newlines=True)
    str_state = output.rsplit(" ", 1)[-1].strip()
    state = states[str_state]
    self.logger.debug("Drive state: %s" % (state.name))
    return state

  def isSleeping(self):
    """ Return True if drive is in low power state, False otherwise. """
    return (self.getState() in (Drive.DriveState.STANDBY, Drive.DriveState.SLEEPING))

  def getTemperature(self):
    """ Get drive temperature in Celcius using either hddtemp or hdparm. """
    if not self.supports_hitachi_temp_query:
      if self.hddtemp_daemon_port is not None:
        # get temp from daemon
        daemon_data = bytearray()
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sckt:
          sckt.connect(("127.0.0.1", self.hddtemp_daemon_port))
          while True:
            new_daemon_data = sckt.recv(4096)
            if not new_daemon_data:
              break
            daemon_data.extend(new_daemon_data)
        # parse it
        daemon_data = daemon_data.decode("utf-8")
        drives_data = iter(daemon_data.split("|")[:-1])
        found = False
        while True:
          drive_data = tuple(itertools.islice(drives_data, 0, 5))
          if not drive_data:
            break
          drive_path = drive_data[1]
          if __class__.normalizeDrivePath(drive_path) == self.device_filepath:
            temp_unit = drive_data[4]
            if temp_unit != "C":
              raise RuntimeError("hddtemp daemon is not returning temp as Celsius")
            temp = int(drive_data[3])
            found = True
            break
        if not found:
          raise RuntimeError("Unable to get temperature from hddtemp daemon for drive %s" % (self))
      else:
        cmd = ("hddtemp", "-u", "C", "-n", self.device_filepath)
        output = subprocess.check_output(cmd,
                                         stdin=subprocess.DEVNULL,
                                         stderr=subprocess.DEVNULL,
                                         universal_newlines=True)
        temp = int(output.strip())
    else:
      cmd = ("hdparm", "-H", self.device_filepath)
      output = subprocess.check_output(cmd,
                                       stdin=subprocess.DEVNULL,
                                       stderr=subprocess.DEVNULL,
                                       universal_newlines=True)
      temp = int(__class__.HDPARM_GET_TEMP_HITACHI_REGEX.search(output).group(1))
    self.logger.debug("Drive temperature: %u C" % (temp))
    return temp

  def spinDown(self):
    """ Spin down a drive, effectively setting it to DriveState.STANDBY state. """
    self.logger.info("Spinning down drive %s" % (self))
    cmd = ("hdparm", "-y", self.device_filepath)
    subprocess.check_call(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

  def getActivityStats(self):
    """ Return drive stats as in /proc/diskstats, as a tuple of integer. """
    with open(self.stat_filepath, "rt") as stat_file:
      stats = stat_file.read()
    stats = filter(None, map(str.strip, stats.strip().split(" ")))
    stats = tuple(map(int, stats))
    if not stats:
      raise RuntimeError("Unable to get stats for drive %s" % (self))
    return stats

  @staticmethod
  def normalizeDrivePath(path):
    """ Normalize filepath by following symbolic links, and making it absolute. """
    if os.path.islink(path):
      r = os.readlink(path)
      if not os.path.isabs(r):
        r = os.path.join(os.path.dirname(path), r)
    else:
      r = path
    return os.path.abspath(r)


class DriveSpinDownThread(threading.Thread):

  """ Thread responsible for spinning down a drive when it is not active for a certain amount of time. """

  def __init__(self, drive, spin_down_time_s):
    super().__init__(name="%s-%s" % (__class__.__name__, drive))
    self.drive = drive
    self.spin_down_time_s = spin_down_time_s
    self.logger = logging.getLogger(self.name)

  def run(self):
    """ Thread loop. """
    try:
      previous_stats = None
      while not exit_evt.is_set():
        if self.drive.isSleeping():
          self.logger.debug("Drive is already sleeping")
          self.sleep(60)
          continue

        if previous_stats is None:
          # get stats
          previous_stats = self.drive.getActivityStats()
          previous_stats_time = time.monotonic()

        # sleep
        self.sleep(min(self.spin_down_time_s, 60))
        if exit_evt.is_set():
          break

        # get stats again
        stats = self.drive.getActivityStats()
        now = time.monotonic()
        if stats != previous_stats:
          self.logger.debug("Drive is active")
          previous_stats = None
        else:
          delta = now - previous_stats_time
          self.logger.debug("Drive is inactive for %u min" % (int(delta / 60)))
          if delta > self.spin_down_time_s:
            self.drive.spinDown()

      self.logger.info("Exiting")

    except Exception as e:
      self.logger.error("%s: %s" % (e.__class__.__qualname__, e))

  def sleep(self, s):
    """ Sleep for s seconds, or less if exit event occurs. """
    self.logger.debug("Sleeping for %u seconds" % (s))
    interrupted = exit_evt.wait(timeout=s)
    if interrupted:
      self.logger.debug("Sleep interrupted")


class Fan:

  """ Represent a fan associated with a PWM file to control its speed. """

  LAST_DIGITS_REGEX = re.compile("[^0-9]*([0-9]*)$")

  def __init__(self, id, pwm_filepath, start_value, stop_value):
    assert(0 <= start_value <= 255)
    assert(0 <= stop_value <= 255)
    self.id = id
    self.pwm_filepath = pwm_filepath
    if stat.S_ISBLK(os.stat(self.pwm_filepath).st_mode):
      # we don't want to write to a block device in setPwmValue
      # command line parameters have probably been mixed up
      raise RuntimeError("%s is a block device, PWM /sys file expected" % (self.pwm_filepath))
    pwm_num = int(__class__.LAST_DIGITS_REGEX.search(self.pwm_filepath).group(1))
    self.fan_input_filepath = os.path.join(os.path.dirname(self.pwm_filepath),
                                           "fan%u_input" % (pwm_num))
    self.enable_filepath = "%s_enable" % (self.pwm_filepath)
    self.start_value = start_value
    self.stop_value = stop_value
    self.startup = False
    self.logger = logging.getLogger("Fan #%u" % (self.id))

  def getRpm(self):
    """ Read fan speed in revolutions per minute. """
    with open(self.fan_input_filepath, "rt") as fan_input_file:
      rpm = int(fan_input_file.read().strip())
    self.logger.debug("Rotation speed is currently %u rpm" % (rpm))
    return rpm

  def isRunning(self):
    """ Return True if fan is moving, False instead. """
    return (self.getRpm() > 0)

  def waitStabilize(self):
    """
    Wait for the fan to have a stable rotational speed.

    The algorithm only works if the fan is either slowing down, accelerating, or steady during the test, not if its speed
    changes quickly ie. going up and down.
    """
    rpm = self.getRpm()
    min_rpm, max_rpm = rpm, rpm
    while True:
      time.sleep(2)
      rpm = self.getRpm()
      if min_rpm <= rpm <= max_rpm:
        break
      min_rpm = min(min_rpm, rpm)
      max_rpm = max(max_rpm, rpm)

  def setSpeed(self, speed_prct, min_prct):
    """ Set fan speed to a percentage of its maximum speed. """
    # preconditions
    assert(0 <= speed_prct <= 100)
    assert(0 <= min_prct <= 100)

    self.logger.info("Setting fan speed to %u%%" % (speed_prct))

    # calculate target PWM value
    if speed_prct == 0:
      target_value = 0
    else:
      target_value = self.stop_value + ((255 - self.stop_value) * speed_prct) // 100
    if min_prct > 0:
      min_value = self.stop_value + ((255 - self.stop_value) * min_prct) // 100
      target_value = max(min_value, target_value)

    if (0 < target_value < self.start_value) and (not self.isRunning()):
      self.startup = True
    else:
      self.startup = False

    # set speed
    if self.startup:
      # fan startup boost
      self.logger.debug("Applying startup boost")
      self.setPwmValue(self.start_value)
    else:
      self.setPwmValue(target_value)

  def setPwmValue(self, value):
    """ Set fan PWM value. """
    assert(0 <= value <= 255)
    with open(self.enable_filepath, "r+t") as enable_file:
      enabled_val = int(enable_file.read().strip())
      if enabled_val != 1:
        self.logger.warning("%s was %u, setting it to 1", self.enable_filepath, enabled_val)
        enable_file.seek(0)
        enable_file.write("1")
    self.logger.debug("Setting PWM value to %u" % (value))
    with open(self.pwm_filepath, "wt") as pwm_file:
      pwm_file.write("%u" % (value))


class TestHardware:

  """ Run basic drive tests, and analyze fan start/stop behaviour. """

  def __init__(self, drives, fans):
    self.drives = drives
    self.fans = fans
    self.ok_count = 0
    self.ko_count = 0
    self.logger = logging.getLogger(__class__.__name__)

  def run(self):
    self.logger.info("Running hardware tests, this may take a few minutes")
    self.testDrives()
    start_stop_values = self.testPwms()
    if self.ko_count > 0:
      print("%u/%u tests failed!" % (self.ko_count, self.ko_count + self.ok_count))
    else:
      print("%u/%u tests OK, all good :)" % (self.ok_count, self.ok_count))
    self.logger.info("Recommended parameters: --pwm-start-value %s --pwm-stop-value %s"
                     % (" ".join(str(min(255, x[0] + 32)) for x in start_stop_values),
                        " ".join(str(x[1]) for x in start_stop_values)))

  def testDrives(self):
    for drive in self.drives:
      self.reportTestGroupStart("Test of drive %s" % (drive))

      test_desc = "Getting drive power state"
      self.reportTestStart(test_desc)
      try:
        state = drive.getState()
        test_ok = state in Drive.DriveState
      except:
        test_ok = False
      self.reportTestResult(test_desc, test_ok)

      test_desc = "Getting drive temperature"
      self.reportTestStart(test_desc)
      try:
        temp = drive.getTemperature()
        test_ok = isinstance(temp, int)
      except:
        test_ok = False
      self.reportTestResult(test_desc, test_ok)

      test_desc = "Getting drive activity statistics"
      self.reportTestStart(test_desc)
      try:
        stats = drive.getActivityStats()
        test_ok = isinstance(stats, tuple)
      except:
        test_ok = False
      self.reportTestResult(test_desc, test_ok)

  def testPwms(self):
    start_stop_values = []
    pwm_vals = (255,) + tuple(range(240, -1, -16))
    for fan in self.fans:
      self.reportTestGroupStart("Test of fan #%u" % (fan.id))
      start_value, stop_value = 255, 0

      test_desc = "Stopping fan"
      self.reportTestStart(test_desc)
      try:
        fan.setPwmValue(0)
        fan.waitStabilize()
        test_ok = not fan.isRunning()
      except:
        test_ok = False
      self.reportTestResult(test_desc, test_ok)

      test_desc = "Starting fan"
      self.reportTestStart(test_desc)
      try:
        fan.setPwmValue(255)
        fan.waitStabilize()
        test_ok = fan.isRunning()
      except:
        test_ok = False
      self.reportTestResult(test_desc, test_ok)

      test_desc = "Finding exact start value of fan"
      self.reportTestStart(test_desc)
      test_ok = False
      try:
        for v in reversed(pwm_vals):
          fan.setPwmValue(v)
          fan.waitStabilize()
          test_ok = fan.isRunning()
          if test_ok:
            start_value = v
            break
      except:
        pass
      self.reportTestResult(test_desc, test_ok)

      test_desc = "Finding exact stop value of fan"
      self.reportTestStart(test_desc)
      test_ok = False
      try:
        for v in pwm_vals:
          fan.setPwmValue(v)
          fan.waitStabilize()
          test_ok = not fan.isRunning()
          if test_ok:
            stop_value = v
            break
      except:
        pass
      self.reportTestResult(test_desc, test_ok)

      start_stop_values.append((start_value, stop_value))

    return start_stop_values

  def reportTestGroupStart(self, desc):
    print("%s %s" % (desc, "-" * (shutil.get_terminal_size()[0] - len(desc) - 1)))

  def reportTestStart(self, desc):
    print(desc, end=" ", flush=True)

  def reportTestResult(self, desc, ok):
    if ok:
      self.ok_count += 1
    else:
      self.ko_count += 1
    print(("[ %s ]" % ("OK" if ok else "KO")).rjust(shutil.get_terminal_size()[0] - len(desc) - 1))


def test(drive_filepaths, fan_pwm_filepaths, hddtemp_daemon_port):
  fans = [Fan(i, fan_pwm_filepath, 0, 0) for i, fan_pwm_filepath in enumerate(fan_pwm_filepaths, 1)]
  drives = [Drive(drive_filepath, hddtemp_daemon_port) for drive_filepath in drive_filepaths]

  tester = TestHardware(drives, fans)
  tester.run()


def signal_handler(sig, frame):
  logging.getLogger("Signal handler").info("Catched signal %u" % (sig))
  global exit_evt
  exit_evt.set()


def set_high_priority(logger):
  """ Change process priority to the highest possible. """
  # use "real time" scheduler
  done = False
  sched = os.SCHED_RR
  prio = os.sched_get_priority_max(sched)
  param = os.sched_param(prio)
  try:
    os.sched_setscheduler(0, sched, param)
  except OSError:
    logger.warning("Failed to set real time process scheduler to %u, priority %u" % (sched, prio))
  else:
    done = True
    logger.info("Process real time scheduler set to %u, priority %u" % (sched, prio))

  if not done:
    # renice to highest priority
    target_niceness = -19
    previous_niceness = os.nice(0)
    delta_niceness = target_niceness - previous_niceness
    try:
      new_niceness = os.nice(delta_niceness)
    except OSError:
      new_niceness = previous_niceness
    if new_niceness != target_niceness:
      logger.warning("Unable to renice process to %d, current niceness is %d" % (target_niceness, new_niceness))
    else:
      logger.info("Process reniced from %d to %d" % (previous_niceness, new_niceness))


def main(drive_filepaths, fan_pwm_filepaths, fan_start_values, fan_stop_values, min_fan_speed_prct, min_temp, max_temp,
         interval_s, spin_down_time_s, hddtemp_daemon_port):
  logger = logging.getLogger("Main")
  fans = []
  try:
    # change process priority
    set_high_priority(logger)

    # register signal handler
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    # init
    fans = [Fan(i,
                fan_pwm_filepath,
                fan_start_value,
                fan_stop_value) for i,
                                    (fan_pwm_filepath,
                                     fan_start_value,
                                     fan_stop_value) in enumerate(zip(fan_pwm_filepaths,
                                                                      fan_start_values,
                                                                      fan_stop_values),
                                                                  1)]
    drives = [Drive(drive_filepath, hddtemp_daemon_port) for drive_filepath in drive_filepaths]
    drives_startup_time = time.monotonic()
    current_fan_speeds = [None] * len(fans)

    # start spin down threads if needed
    spin_down_threads = []
    if (spin_down_time_s is not None) and (spin_down_time_s > interval_s):
      for drive in drives:
        spin_down_threads.append(DriveSpinDownThread(drive, spin_down_time_s))
      for thread in spin_down_threads:
        thread.start()

    while not exit_evt.is_set():
      now = time.monotonic()

      # calc max drive temperature
      temps = []
      awakes = []
      for drive in drives:
        awake = not drive.isSleeping()
        awakes.append(awake)
        if awake or drive.supports_hitachi_temp_query:
          temps.append(drive.getTemperature())
        else:
          logger.debug("Drive %s is in low power state, unable to query temperature" % (drive))
      if temps:
        temp = max(temps)
        logger.info("Maximum drive temperature: %u C" % (temp))
      else:
        assert(not any(awakes))
        logger.info("All drives are in low power state")

      if not any(awakes):
        drives_startup_time = now

      # calc target percentage fan speed
      if temps and (temp - min_temp > 0):
        speed_prct = 100 * (temp - min_temp) // (max_temp - min_temp)
        speed_prct = int(min(speed_prct, 100))
      else:
        speed_prct = 0

      # set fan speed if needed
      for i, fan in enumerate(fans):
        if current_fan_speeds[i] != speed_prct:
          fan.setSpeed(speed_prct, min_fan_speed_prct)
          current_fan_speeds[i] = speed_prct

      # sleep
      if any(map(operator.attrgetter("startup"), fans)):
        # at least one fan is starting up, quickly cancel startup boost
        current_interval_s = min(10, interval_s)
      elif any(awakes) and ((now - drives_startup_time) < (60 * 5)):
        # if at least a drive was started or waken up less than 5 min ago, dont' sleep too long because it
        # can heat up quickly
        current_interval_s = min(20, interval_s)
      else:
        current_interval_s = interval_s
      logger.debug("Sleeping for %u seconds" % (current_interval_s))
      exit_evt.wait(current_interval_s)

    logger.info("Exiting")

    for thread in spin_down_threads:
      thread.join()

  except Exception as e:
    logger.error("%s: %s" % (e.__class__.__qualname__, e))

  # run fans at full speed at exit
  for fan in fans:
    fan.setSpeed(100, 100)


def cl_main():
  # parse args
  arg_parser = argparse.ArgumentParser(description="HDD Fan Control v%s.%s" % (__version__, __doc__),
                                       formatter_class=argparse.ArgumentDefaultsHelpFormatter)
  arg_parser.add_argument("-d",
                          "--drives",
                          required=True,
                          nargs="+",
                          dest="drive_filepaths",
                          help="Drive(s) to get temperature from (ie. /dev/sdX)")
  arg_parser.add_argument("-p",
                          "--pwm",
                          required=True,
                          nargs="+",
                          dest="fan_pwm_filepath",
                          help="PWM filepath(s) to control fan speed (under /sys)")
  arg_parser.add_argument("--pwm-start-value",
                          type=int,
                          default=None,
                          nargs="+",
                          dest="fan_start_value",
                          help="""PWM value (0-255), at which the fan starts moving.
                                  Use the -t parameter, or run pwmconfig to find this value.""")
  arg_parser.add_argument("--pwm-stop-value",
                          type=int,
                          default=None,
                          nargs="+",
                          dest="fan_stop_value",
                          help="""PWM value (0-255), at which the fan stop moving.
                                  Use the -t parameter, or run pwmconfig to find this value.
                                  Often 20-40 lower than start speed.""")
  arg_parser.add_argument("--min-temp",
                          type=int,
                          default=30,
                          dest="min_temp",
                          help="Temperature in Celcius at which the fan(s) will be set to minimum speed.")
  arg_parser.add_argument("--max-temp",
                          type=int,
                          default=50,
                          dest="max_temp",
                          help="Temperature in Celcius at which the fan(s) will be set to maximum speed.")
  arg_parser.add_argument("--min-fan-speed-prct",
                          type=int,
                          default=20,
                          dest="min_fan_speed_prct",
                          help="""Minimum percentage of full fan speed to set the fan to.
                                  Never set to 0 unless you have other fans to cool down your system,
                                  or a case specially designed for passive cooling.""")
  arg_parser.add_argument("-i",
                          "--interval",
                          type=int,
                          default=60,
                          dest="interval_s",
                          help="Interval in seconds to check temperature and adjust fan speed.")
  arg_parser.add_argument("--spin-down-time",
                          type=int,
                          default=None,
                          dest="spin_down_time_s",
                          help="Interval in seconds after which inactive drives will be put to standby state.")
  arg_parser.add_argument("-v",
                          "--verbosity",
                          choices=("warning", "normal", "debug"),
                          default="normal",
                          dest="verbosity",
                          help="Level of logging output")
  arg_parser.add_argument("-b",
                          "--background",
                          action="store_true",
                          dest="daemonize",
                          help="Daemonize process")
  arg_parser.add_argument("-l",
                          "--log-file",
                          default=None,
                          dest="log_filepath",
                          help="Filepath for log output when using daemon mode, if omitted, logging goes to syslog.")
  arg_parser.add_argument("--pid-file",
                          default=None,
                          dest="pid_filepath",
                          help="Filepath for lock file when using daemon mode")
  arg_parser.add_argument("-t",
                          "--test",
                          action="store_true",
                          default=False,
                          dest="test_mode",
                          help="Run some tests and exit")
  arg_parser.add_argument("--hddtemp-daemon",
                          action="store_true",
                          default=False,
                          dest="hddtemp_daemon",
                          help="""Get drive temperature from hddtemp daemon instead of spawning
                                  a new process each time temperature is probed""")
  arg_parser.add_argument("--hddtemp-daemon-port",
                          type=int,
                          default=7634,
                          dest="hddtemp_daemon_port",
                          help="hddtemp daemon port if option --hddtemp-daemon is used")
  args = arg_parser.parse_args()
  if (((args.fan_start_value is not None) and (len(args.fan_pwm_filepath) != len(args.fan_start_value))) or
          ((args.fan_stop_value is not None) and (len(args.fan_pwm_filepath) != len(args.fan_stop_value)))):
    print("Invalid parameter count")
    exit(1)

  # setup logger
  logging_level = {"warning": logging.WARNING,
                   "normal": logging.INFO,
                   "debug": logging.DEBUG}
  logging.getLogger().setLevel(logging_level[args.verbosity])
  logging_fmt = "%(asctime)s %(levelname)s [%(name)s] %(message)s"
  if args.daemonize:
    if args.log_filepath is not None:
      # log to file
      logging_handler = logging.handlers.WatchedFileHandler(args.log_filepath)
    else:
      # log to syslog
      logging_fmt = "%(levelname)s [%(name)s] %(message)s"
      logging_handler = LoggingSysLogHandler(syslog.LOG_DAEMON)
    logging_formatter = logging.Formatter(fmt=logging_fmt)
  else:
    # log to stderr
    logging_formatter = colored_logging.ColoredFormatter(fmt=logging_fmt)
    logging_handler = logging.StreamHandler()
  logging_handler.setFormatter(logging_formatter)
  logging.getLogger().addHandler(logging_handler)

  # check if root
  if os.geteuid() != 0:
    logging.getLogger("Startup").error("You need to run this script as root")
    exit(1)

  if args.test_mode or (args.fan_start_value is None) or (args.fan_stop_value is None):
    if (args.fan_start_value is None) or (args.fan_stop_value is None):
      logging.getLogger("Startup").warning("Missing --pwm-start-value or --pwm-stop-value argument, running hardware test to find values")
    test(args.drive_filepaths,
         args.fan_pwm_filepath,
         args.hddtemp_daemon_port if args.hddtemp_daemon else None)

  else:
    # main
    with contextlib.ExitStack() as daemon_context:
      if args.daemonize:
        preserved_fds = None
        if args.log_filepath is not None:
          preserved_fds = [logging_handler.stream.fileno()]
        if args.pid_filepath is not None:
          pidfile = daemon.pidfile.PIDLockFile(args.pid_filepath)
          if pidfile.is_locked():
            logging.getLogger("Startup").error("Daemon already running")
            exit(1)
        else:
          pidfile = None
        daemon_context.enter_context(daemon.DaemonContext(pidfile=pidfile,
                                                          files_preserve=preserved_fds))
      main(args.drive_filepaths,
           args.fan_pwm_filepath,
           args.fan_start_value,
           args.fan_stop_value,
           args.min_fan_speed_prct,
           args.min_temp,
           args.max_temp,
           args.interval_s,
           args.spin_down_time_s,
           args.hddtemp_daemon_port if args.hddtemp_daemon else None)


# check deps
bin_dep.check_bin_dependency(("hddtemp", "hdparm"))


if __name__ == "__main__":
  cl_main()
