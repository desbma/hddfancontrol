#!/usr/bin/env python3
# -*- coding: utf-8 -*-

""" Dynamically control fan speed according to hard drive temperature. """

import argparse
import contextlib
import enum
import logging
import operator
import os
import re
import stat
import subprocess
import threading
import time

import daemon
import daemon.pidlockfile

import bin_dep
import colored_logging


class Drive:

  """ Drive represented by a device file like /dev/sdX. """

  DriveState = enum.Enum("DriveState", ("UNKNOWN", "ACTIVE_IDLE", "STANDBY", "SLEEPING"))

  def __init__(self, device_filepath, stat_filepath):
    assert(stat.S_ISBLK(os.stat(device_filepath).st_mode))
    self.device_filepath = device_filepath
    self.stat_filepath = stat_filepath
    self.logger = logging.getLogger(str(self))
    # test if drive supports hdparm -H
    cmd = ("hdparm", "-H", self.device_filepath)
    try:
      subprocess.check_call(cmd,
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
      self.logger.warning("Drive does not allow querying temperature without going out of low power mode.")
      self.supports_hitachi_temp_query = False
    else:
      self.supports_hitachi_temp_query = True

  def __str__(self):
    """ Return a pretty drive name. """
    return os.path.basename(self.device_filepath).rsplit("-", 1)[-1]

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
      temp = int(re.search("drive temperature \(celsius\) is:\s*([0-9]*)", output).group(1))
    self.logger.debug("Drive temperature: %u°C" % (temp))
    return temp

  def spinDown(self):
    """ Spin down a drive, effectively setting it to DriveState.STANDBY state. """
    logging.getLogger().info("Spinning down drive %s" % (self))
    cmd = ("hdparm", "-y", self.device_filepath)
    subprocess.check_call(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

  def getActivityStats(self):
    """ Return drive stats as in /proc/diskstats, as a tuple of integer. """
    with open(self.stat_filepath, "rt") as stat_file:
      stats = stat_file.read()
    stats = filter(None, map(str.strip, stats.strip().split(" ")))
    stats = tuple(map(int, stats))
    return stats


class DriveSpinDownThread(threading.Thread):

  """ Thread responsible for spinning down a drive when it is not active for a certain amount of time. """

  def __init__(self, drive, spin_down_time_s):
    self.drive = drive
    self.spin_down_time_s = spin_down_time_s
    super().__init__(name="%s-%s" % (__class__.__name__, drive))

  def run(self):
    """ Thread loop. """
    try:
      logger = logging.getLogger(self.name)
      previous_stats = None
      while True:
        if self.drive.isSleeping():
          logger.debug("Drive is already sleeping")
          time.sleep(60)
          continue

        if previous_stats is None:
          # get stats
          previous_stats = self.drive.getActivityStats()
          previous_stats_time = time.time()

        # sleep
        time.sleep(min(self.spin_down_time_s, 60))

        # get stats again
        stats = self.drive.getActivityStats()
        now = time.time()
        if stats != previous_stats:
          logger.debug("Drive is active")
          previous_stats = None
        else:
          delta = now - previous_stats_time
          logger.debug("Drive is inactive for %u min" % (int(delta / 60)))
          if delta > self.spin_down_time_s:
            self.drive.spinDown()

    except Exception as e:
      logging.getLogger().error(e)


class Fan:

  """ Represent a fan associated with a PWM file to control its speed. """

  def __init__(self, id, pwm_filepath, start_value, stop_value):
    assert(0 <= start_value <= 255)
    assert(0 <= stop_value <= 255)
    self.id = id
    self.pwm_filepath = pwm_filepath
    self.start_value = start_value
    self.stop_value = stop_value
    self.startup = False
    self.logger = logging.getLogger("Fan #%u" % (self.id))

  def getRpm(self):
    """ Read fan speed in revolutions per minute. """
    pwm_num = int(re.search("[^0-9]*([0-9]*)$", self.pwm_filepath).group(1))
    fan_input_filepath = os.path.join(os.path.dirname(self.pwm_filepath), "fan%u_input" % (pwm_num))
    with open(fan_input_filepath, "rt") as fan_input_file:
      rpm = int(fan_input_file.read().strip())
      self.logger.debug("Rotation speed is currently %u rpm" % (rpm))
    return rpm

  def isRunning(self):
    """ Return True if fan is moving, False instead. """
    return (self.getRpm() > 0)

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
    assert(not stat.S_ISBLK(os.stat(self.pwm_filepath).st_mode))  # check we are not mistakenly writing to a device file
    enabled_filepath = "%s_enable" % (self.pwm_filepath)
    with open(enabled_filepath, "r+t") as enabled_file:
      enabled_val = int(enabled_file.read().strip())
      if enabled_val != 1:
        self.logger.warning("%s was %u, setting it to 1", enabled_filepath, enabled_val)
      enabled_file.seek(0)
      enabled_file.write("1")
    with open(self.pwm_filepath, "wt") as pwm_file:
      self.logger.debug("Setting PWM value to %u" % (value))
      pwm_file.write("%u" % (value))


def main(drive_filepaths, fan_pwm_filepaths, fan_start_values, fan_stop_values, min_fan_speed_prct, min_temp, max_temp,
         interval_s, spin_down_time_s, stat_filepaths):
  try:
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
    drives = [Drive(drive_filepath,
                    stat_filepath) for drive_filepath, stat_filepath in zip(drive_filepaths,
                                                                            stat_filepaths)]

    drives_startup_time = time.time()

    # start spin down threads if needed
    spin_down_threads = []
    if (spin_down_time_s is not None) and (spin_down_time_s > interval_s):
      for drive in drives:
        spin_down_threads.append(DriveSpinDownThread(drive, spin_down_time_s))
      for thread in spin_down_threads:
        thread.start()

    while True:
      now = time.time()

      # calc max drive temperature
      temps = []
      awakes = []
      logger = logging.getLogger("Fan speed control")
      for drive in drives:
        awake = not drive.isSleeping()
        awakes.append(awake)
        if awake or drive.supports_hitachi_temp_query:
          temps.append(drive.getTemperature())
        else:
          logger.debug("Drive %s is in low power state, unable to query temperature" % (drive))
      if temps:
        temp = max(temps)
        logger.info("Maximum drive temperature: %u°C" % (temp))
      else:
        assert(not any(awakes))
        logger.info("All drives are in low power state")

      if not any(awakes):
        drives_startup_time = now

      # calc target percentage speed
      if temps and (temp - min_temp > 0):
        speed_prct = 100 * (temp - min_temp) // (max_temp - min_temp)
        speed_prct = int(min(speed_prct, 100))
      else:
        speed_prct = 0

      # set speed
      for fan in fans:
        fan.setSpeed(speed_prct, min_fan_speed_prct)

      # sleep
      if any(map(operator.attrgetter("startup"), fans)):
        # at least one fan is starting up, quickly cancel startup boost
        current_interval_s = min(10, interval_s)
      elif (now - drives_startup_time) < (60 * 5):
        # if all drives were started or waken up less than 5 min ago, dont' sleep too long because they
        # can heat up quickly
        current_interval_s = min(20, interval_s)
      else:
        current_interval_s = interval_s
      logger.debug("Sleeping for %u seconds" % (current_interval_s))
      time.sleep(current_interval_s)

  except Exception as e:
    logging.getLogger().error(e)


# check deps
bin_dep.check_bin_dependency(("hddtemp", "hdparm"))


if __name__ == "__main__":
  # parse args
  arg_parser = argparse.ArgumentParser(description=__doc__)
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
                          required=True,
                          type=int,
                          nargs="+",
                          dest="fan_start_value",
                          help="""PWM value (0-255), at which the fan starts moving.
                                  Run pwmconfig to find this value.""")
  arg_parser.add_argument("--pwm-stop-value",
                          required=True,
                          type=int,
                          nargs="+",
                          dest="fan_stop_value",
                          help="""PWM value (0-255), at which the fan stop moving.
                                  Run pwmconfig to find this value.
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
  arg_parser.add_argument("--stat-files",
                          required=True,
                          nargs="+",
                          dest="stat_filepaths",
                          help="Filepath of drive stats file (ie. /sys/block/sdX/stat)")
  arg_parser.add_argument("--spin-down-time",
                          type=int,
                          default=None,
                          dest="spin_down_time_s",
                          help="Interval in seconds after which inactive drives will be put to standby state.")
  arg_parser.add_argument("-v",
                          "--verbosity",
                          action="store",
                          choices=("warning", "normal", "debug"),
                          default="normal",
                          dest="verbosity",
                          help="Level of output to display")
  arg_parser.add_argument("-b",
                          "--background",
                          action="store_true",
                          dest="daemonize",
                          help="Daemonize process")
  arg_parser.add_argument("-l",
                          "--log-file",
                          action="store",
                          default=None,
                          dest="log_filepath",
                          help="Filepath for log output when using deamon mode")
  arg_parser.add_argument("--pid-file",
                          action="store",
                          default=None,
                          dest="pid_filepath",
                          help="Filepath for lock file when using deamon mode")
  args = arg_parser.parse_args()
  if not (len(args.fan_pwm_filepath) == len(args.fan_start_value) == len(args.fan_stop_value) ==
          len(args.stat_filepaths)):
    raise ValueError("Invalid parameter count")

  # setup logger
  logging_level = {"warning": logging.WARNING,
                   "normal": logging.INFO,
                   "debug": logging.DEBUG}
  logging.getLogger().setLevel(logging_level[args.verbosity])
  logging_formatter = colored_logging.ColoredFormatter(fmt="%(asctime)s %(levelname)s [%(name)s] %(message)s")
  logging_handler = logging.StreamHandler()
  logging_handler.setFormatter(logging_formatter)
  logging.getLogger().addHandler(logging_handler)

  # check if root
  if os.geteuid() != 0:
    logging.getLogger().error("You need to run this script as root")
    exit(1)

  # main
  with contextlib.ExitStack() as deamon_context:
    if args.daemonize:
      preserved_fds = None
      if args.log_filepath is not None:
        log_output = deamon_context.enter_context(open(args.log_filepath, "at+"))
        preserved_fds = [log_output.fileno()]
      else:
        log_output = None
      if args.pid_filepath is not None:
        pidfile = daemon.pidlockfile.PIDLockFile(args.pid_filepath)
        if pidfile.is_locked():
          logging.getLogger().error("Daemon already running")
          exit(1)
      else:
        pidfile = None
      deamon_context.enter_context(daemon.DaemonContext(stdout=log_output,
                                                        stderr=log_output,
                                                        pidfile=pidfile,
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
         args.stat_filepaths)
