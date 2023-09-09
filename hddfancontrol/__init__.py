#!/usr/bin/env python3

""" Dynamically control fan speed according to hard drive temperature. """

__version__ = "1.5.1"
__author__ = "desbma"
__license__ = "GPLv3"

import abc
import argparse
import collections
import contextlib
import enum
import errno
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
import sys
import syslog
import threading
import time
from typing import Dict, List, Optional, Sequence, Tuple

import daemon
import daemon.pidfile

from hddfancontrol import bin_dep, colored_logging

MAX_DRIVE_STARTUP_SLEEP_INTERVAL_S = 20
MAX_FAN_STARTUP_TIME_S = 10
DRIVE_STARTUP_TIME_S = 60 * 5


exit_evt = threading.Event()


class DriveAsleepError(Exception):

    """Exception raised when getTemperature fails because drive is asleep."""

    pass


class HddtempDaemonQueryFailed(Exception):

    """Exception raised when hddtemp daemon fails to return temperature for a drive."""

    pass


class LoggingSysLogHandler(logging.Handler):

    """Class similar in goal to logging.handlers.SysLogHandler, but uses the syslog call instead of socket."""

    def __init__(self, facility, options=syslog.LOG_PID):
        syslog.openlog(logoption=options, facility=facility)
        super().__init__()

    def emit(self, record):
        """See logging.Handler.emit."""
        msg = self.format(record)
        h = logging.handlers.SysLogHandler
        level = h.priority_names[h.priority_map[record.levelname]]
        syslog.syslog(level, msg)

    def close(self):
        """See logging.Handler.close."""
        syslog.closelog()
        super().close()


class HotDevice:

    """Base class for devices generating heat."""

    @abc.abstractmethod
    def getTemperature(self) -> float:
        """Get device temperature as int or float."""
        pass

    @abc.abstractmethod
    def getTemperatureRange(self) -> Tuple[float, float]:
        """
        Get min/max target temperatures.

        Fans should be at minimum speed below min, and blow at full speed above max.
        """
        pass


class Drive(HotDevice):

    """Drive represented by a device file like /dev/sdX."""

    DriveState = enum.Enum("DriveState", ("UNKNOWN", "ACTIVE_IDLE", "STANDBY", "SLEEPING"))

    HDPARM_GET_TEMP_HITACHI_REGEX = re.compile(r"drive temperature \(celsius\) is:\s*([0-9]*)")
    HDPARM_GET_TEMP_HITACHI_ERROR_REGEX = re.compile("^SG_IO: .* sense data", re.MULTILINE)
    HDPARM_GET_MODEL_REGEX = re.compile(r"Model Number:\s*(.*)")
    HDDTEMP_SLEEPING_SUFFIX = ": drive is sleeping\n"

    class TempProbingMethod(enum.Enum):

        """Method used to query drive temperature, depending on what the drive/system support."""

        HDDTEMP_INVOCATION = enum.auto()
        HDDTEMP_DAEMON = enum.auto()
        HDPARM_INVOCATION = enum.auto()
        SMARTCTL_ATTRIB_INVOCATION = enum.auto()
        SMARTCTL_SCT_INVOCATION = enum.auto()
        DRIVETEMP = enum.auto()

    def __init__(
        self,
        device_filepath: str,
        hddtemp_daemon_port: Optional[int],
        min_temp: float,
        max_temp: float,
        use_smartctl: bool,
    ):
        assert stat.S_ISBLK(os.stat(device_filepath).st_mode)
        device_filepath = self.__class__.normalizeDrivePath(device_filepath)
        if not self.__class__.isPartition(device_filepath):
            self.device_filepath = device_filepath
            self.stat_filepath = f"/sys/block/{os.path.basename(self.device_filepath)}/stat"
        else:
            self.device_filepath = self.__class__.getParentDevice(device_filepath)
            self.stat_filepath = (
                f"/sys/block/{os.path.basename(self.device_filepath)}/{os.path.basename(device_filepath)}/stat"
            )
        self.hddtemp_daemon_port = hddtemp_daemon_port

        self.min_temp = min_temp
        self.max_temp = max_temp

        self.pretty_name = self.getPrettyName()
        self.logger = logging.getLogger(str(self))

        self.drivetemp_input_filepath = self.getDrivetempInputFilepath()
        if use_smartctl:
            if self.supportsSctTempQuery():
                self.temp_query_method = Drive.TempProbingMethod.SMARTCTL_SCT_INVOCATION
            else:
                self.temp_query_method = Drive.TempProbingMethod.SMARTCTL_ATTRIB_INVOCATION
        elif self.drivetemp_input_filepath is not None:
            self.temp_query_method = Drive.TempProbingMethod.DRIVETEMP
        elif self.supportsHitachiTempQuery():
            self.temp_query_method = Drive.TempProbingMethod.HDPARM_INVOCATION
        elif self.hddtemp_daemon_port is not None:
            self.temp_query_method = Drive.TempProbingMethod.HDDTEMP_DAEMON
        else:
            self.temp_query_method = Drive.TempProbingMethod.HDDTEMP_INVOCATION
        self.logger.info(f"Will probe temperature with method {self.temp_query_method.name}")

        self.probe_lock = threading.Lock()
        self.probe_count = 0
        self.get_state_lock = threading.Lock()
        self.get_state_count = 0

        self.warned_smartctl_attrib_counters = False

        if self.__class__.isPartition(device_filepath):
            self.logger.warning(
                f"{device_filepath!r} is a partition, "
                f"parent device {self.device_filepath!r} will be used except for activity stats"
            )

    def __str__(self):
        """Return a pretty drive name."""
        return self.pretty_name

    def getPrettyName(self) -> str:
        """Return a pretty drive name."""
        # get device metadata to grab model string
        cmd = ("hdparm", "-I", self.device_filepath)
        output = subprocess.check_output(
            cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, universal_newlines=True
        )
        model_match = self.__class__.HDPARM_GET_MODEL_REGEX.search(output)
        assert model_match is not None
        model = model_match.group(1).strip()
        return f"{os.path.basename(self.device_filepath)} {model}"

    def getDrivetempInputFilepath(self) -> Optional[str]:
        """Return sysfs drivetemp input filepath if supported, None instead."""
        try:
            sysfs_bus_dir = f"/sys/block/{os.path.basename(self.device_filepath)}"
            sysfs_bus_dir = os.path.normpath(
                os.path.join(os.path.dirname(sysfs_bus_dir), os.readlink(sysfs_bus_dir), "..", "..")
            )
            with os.scandir(f"{sysfs_bus_dir}/hwmon") as dir_it:
                for entry in filter(operator.methodcaller("is_dir"), dir_it):
                    hwmon_dir = entry.path
                    with open(f"{hwmon_dir}/name", "rt") as f:
                        name = f.read().rstrip()
                    if name == "drivetemp":
                        return f"{hwmon_dir}/temp1_input"
        except FileNotFoundError:
            pass
        self.logger.info("Drive does not support native drivetemp temp query")
        return None

    def supportsHitachiTempQuery(self) -> bool:
        """Test if drive supports hdparm -H."""
        supported = True
        cmd = ("hdparm", "-H", self.device_filepath)
        try:
            output = subprocess.check_output(
                cmd, stdin=subprocess.DEVNULL, stderr=subprocess.STDOUT, universal_newlines=True
            )
        except subprocess.CalledProcessError:
            supported = False
        else:
            # catch non fatal errors
            # see: https://github.com/Distrotech/hdparm/blob/4517550db29a91420fb2b020349523b1b4512df2/sgio.c#L308-L315
            if self.__class__.HDPARM_GET_TEMP_HITACHI_ERROR_REGEX.search(output) is not None:
                supported = False
        if not supported:
            self.logger.warning("Drive does not support HGST temp query")
        return supported

    def supportsSctTempQuery(self) -> bool:
        """Test if drive supports smartctl -l scttempsts."""
        supported = True
        cmd = ("smartctl", "-l", "scttempsts", self.device_filepath)
        output = subprocess.check_output(
            cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, universal_newlines=True
        )
        try:
            temp_line = next(filter(lambda x: x.lstrip().startswith("Current Temperature: "), output.splitlines()))
            int(temp_line.split()[2])
        except Exception:
            supported = False
        else:
            supported = True
        if not supported:
            self.logger.warning("Drive does not support SCT temp query")
        return supported

    def supportsProbingWhileAsleep(self) -> bool:
        """Return True if drive can be probed while asleep, without waking up, False instead."""
        return self.temp_query_method in (Drive.TempProbingMethod.HDPARM_INVOCATION, Drive.TempProbingMethod.DRIVETEMP)

    def getState(self) -> DriveState:
        """Get drive power state, as a DriveState enum."""
        states = {
            "unknown": self.__class__.DriveState.UNKNOWN,
            "active": self.__class__.DriveState.ACTIVE_IDLE,
            "idle": self.__class__.DriveState.ACTIVE_IDLE,
            "active/idle": self.__class__.DriveState.ACTIVE_IDLE,
            "standby": self.__class__.DriveState.STANDBY,
            "sleeping": self.__class__.DriveState.SLEEPING,
        }
        cmd = ("hdparm", "-C", self.device_filepath)
        with self.get_state_lock:
            output = subprocess.check_output(
                cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, universal_newlines=True
            )
            self.get_state_count += 1
        str_state = output.rsplit(" ", 1)[-1].strip()
        state = states[str_state]
        self.logger.debug(f"Drive state: {state.name}")
        return state

    def isSleeping(self) -> bool:
        """Return True if drive is in low power state, False otherwise."""
        return self.getState() in (Drive.DriveState.STANDBY, Drive.DriveState.SLEEPING)

    def getTemperatureRange(self) -> Tuple[float, float]:
        """See HotDevice.getTemperatureRange."""
        return self.min_temp, self.max_temp

    def getTemperature(self) -> float:
        """Get drive temperature in Celcius."""
        methods = {
            Drive.TempProbingMethod.HDDTEMP_INVOCATION: self.getTemperatureWithHddtempInvocation,
            Drive.TempProbingMethod.HDDTEMP_DAEMON: self.getTemperatureWithHddtempDaemon,
            Drive.TempProbingMethod.HDPARM_INVOCATION: self.getTemperatureWithHdparmInvocation,
            Drive.TempProbingMethod.SMARTCTL_ATTRIB_INVOCATION: self.getTemperatureWithSmartctlAttribInvocation,
            Drive.TempProbingMethod.SMARTCTL_SCT_INVOCATION: self.getTemperatureWithSmartctlSctInvocation,
            Drive.TempProbingMethod.DRIVETEMP: self.getTemperatureWithDrivetemp,
        }
        with self.probe_lock:
            try:
                temp = methods[self.temp_query_method]()
            except HddtempDaemonQueryFailed:
                self.logger.warning(
                    "Hddtemp daemon returned an error when querying temperature for this drive, "
                    "falling back to hddtemp process invocation. "
                    "If that happens often, you may need to raise the hddtemp daemon priority "
                    "(see: https://github.com/desbma/hddfancontrol/issues/15#issuecomment-461405402)."
                )
                temp = self.getTemperatureWithHddtempInvocation()

            self.probe_count += 1

        self.logger.debug(f"Drive temperature: {temp} °C")
        return temp

    def getTemperatureWithHddtempDaemon(self) -> int:
        """Get drive temperature in Celcius using hddtemp daemon."""
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
        daemon_data_str = daemon_data.decode("utf-8")
        drives_data = iter(daemon_data_str.split("|")[:-1])
        found = False
        while True:
            drive_data = tuple(itertools.islice(drives_data, 0, 5))
            if not drive_data:
                break
            drive_path = drive_data[1]
            if self.__class__.normalizeDrivePath(drive_path) == self.device_filepath:
                if drive_data[3] == "ERR":
                    raise HddtempDaemonQueryFailed
                if drive_data[3] == "SLP":
                    raise DriveAsleepError
                temp_unit = drive_data[4]
                if temp_unit != "C":
                    raise RuntimeError("hddtemp daemon is not returning temp as Celsius")
                temp = int(drive_data[3])
                found = True
                break

        if not found:
            raise RuntimeError(f"Unable to get temperature from hddtemp daemon for drive {self}")
        return temp

    def getTemperatureWithHddtempInvocation(self) -> int:
        """Get drive temperature in Celcius using a one shot hddtemp invocation."""
        cmd = ("hddtemp", "-u", "C", "-n", self.device_filepath)
        cmd_env = dict(os.environ)
        cmd_env["LANG"] = "C"
        output = subprocess.check_output(
            cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, env=cmd_env, universal_newlines=True
        )
        if output.endswith(self.__class__.HDDTEMP_SLEEPING_SUFFIX):
            raise DriveAsleepError
        return int(output.strip())

    def getTemperatureWithHdparmInvocation(self) -> int:
        """Get drive temperature in Celcius using hdparm."""
        cmd = ("hdparm", "-H", self.device_filepath)
        output = subprocess.check_output(
            cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, universal_newlines=True
        )
        temp_match = self.__class__.HDPARM_GET_TEMP_HITACHI_REGEX.search(output)
        assert temp_match is not None
        return int(temp_match.group(1))

    def getTemperatureWithSmartctlAttribInvocation(self) -> int:
        """Get drive temperature in Celcius using smartctl SMART attribute."""
        cmd = ("smartctl", "-A", self.device_filepath)
        output = subprocess.check_output(
            cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, universal_newlines=True
        )
        output_lines = output.splitlines()

        prefixes = collections.OrderedDict(
            (
                ("194 Temperature_Celsius", 9),
                ("190 Airflow_Temperature_Cel", 9),
                # https://github.com/smartmontools/smartmontools/blob/1f3ff52f06c2c281f7531a6c4bd7dc32eac00201/smartmontools/nvmeprint.cpp#L343
                (
                    "Temperature: ",
                    1,
                ),
                # https://github.com/smartmontools/smartmontools/blob/1f3ff52f06c2c281f7531a6c4bd7dc32eac00201/smartmontools/scsiprint.cpp#L330
                ("Current Drive Temperature: ", 3),
            )
        )

        for line_prefix, token_index in prefixes.items():
            try:
                temp_line = next(filter(lambda x: x.lstrip().startswith(line_prefix), output_lines))
            except StopIteration:
                continue
            break

        return int(temp_line.split()[token_index])

    def getTemperatureWithSmartctlSctInvocation(self) -> int:
        """Get drive temperature in Celcius using smartctl SCT reading."""
        cmd = ("smartctl", "-l", "scttempsts", self.device_filepath)
        output = subprocess.check_output(
            cmd, stdin=subprocess.DEVNULL, stderr=subprocess.DEVNULL, universal_newlines=True
        )
        temp_line = next(filter(lambda x: x.lstrip().startswith("Current Temperature: "), output.splitlines()))
        return int(temp_line.split()[2])

    def getTemperatureWithDrivetemp(self) -> int:
        """Get drive temperature in Celcius using drivetemp sysfs."""
        assert self.drivetemp_input_filepath is not None
        with open(self.drivetemp_input_filepath, "rt") as f:
            return int(f.read().rstrip()) // 1000

    def spinDown(self) -> None:
        """Spin down a drive, effectively setting it to DriveState.STANDBY state."""
        self.logger.info(f"Spinning down drive {self}")
        cmd = ("hdparm", "-y", self.device_filepath)
        subprocess.check_call(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def getActivityStats(self) -> Tuple[int, ...]:
        """Return drive stats as in /proc/diskstats, as a tuple of integer."""
        with open(self.stat_filepath, "rt") as stat_file:
            stat_data = stat_file.read()
        stats = tuple(map(int, filter(None, map(str.strip, stat_data.strip().split(" ")))))
        if not stats:
            raise RuntimeError(f"Unable to get stats for drive {self}")
        return stats

    def compareActivityStats(
        self, prev: Tuple[int, ...], current: Tuple[int, ...], temp_probe_count: int, state_probe_count: int
    ) -> bool:
        """
        Compare two samples of activity stats and return True if real activity occured.

        Unfortunaly some Linux kernel change between 5.4 and 5.10 makes temperature or state probing increase the read
        activity counters, so we can not just compare the counters.
        Empirical evidence seems to indicate that a temp or state probe shows in the counters as a fixed number
        of completed reads operations, and "sector" reads. The actual numbers vary depending on the probing method.
        So try to detect that and see if it matches our probe count.
        See https://www.kernel.org/doc/Documentation/iostats.txt
        """
        if prev == current:
            # no activity whatsoever
            # self.logger.debug("No activity")
            return False

        if prev[4:9] != current[4:9]:
            # write occured
            # self.logger.debug("Write activity")
            return True

        if prev[1] != current[1]:
            # actual read occured
            # self.logger.debug("Read activity")
            return True

        # only reads have been registered, it can be data reads or just our own temp/state probes showing up in
        # the counters

        if self.temp_query_method is Drive.TempProbingMethod.SMARTCTL_SCT_INVOCATION:
            expected_read_io_delta = temp_probe_count * 4
            expected_read_sectors_delta = temp_probe_count * 3

        elif self.temp_query_method is Drive.TempProbingMethod.SMARTCTL_ATTRIB_INVOCATION:
            if not self.warned_smartctl_attrib_counters:
                self.logger.warning(
                    "Your kernel version and the current method of temperature probing "
                    "does not allow reliably keeping track of drive activity. "
                    "Auto spin down will not work."
                )
                self.warned_smartctl_attrib_counters = True
            return True

        elif self.temp_query_method is Drive.TempProbingMethod.DRIVETEMP:
            expected_read_io_delta = 0
            expected_read_sectors_delta = 0

        elif self.temp_query_method is Drive.TempProbingMethod.HDPARM_INVOCATION:
            expected_read_io_delta = temp_probe_count
            expected_read_sectors_delta = 0

        else:
            # hddtemp
            assert self.temp_query_method in (
                Drive.TempProbingMethod.HDDTEMP_INVOCATION,
                Drive.TempProbingMethod.HDDTEMP_DAEMON,
            )
            expected_read_io_delta = temp_probe_count * 5
            expected_read_sectors_delta = temp_probe_count * 3

        expected_read_io_delta += state_probe_count

        # self.logger.debug(
        #     f"prev={prev!r} current={current!r} "
        #     f"expected_delta=({expected_read_io_delta}, {expected_read_sectors_delta})"
        # )

        return not (
            (current[0] - prev[0] == expected_read_io_delta) and (current[2] - prev[2] == expected_read_sectors_delta)
        )

    def getTempProbeLock(self) -> threading.Lock:
        """Return the mutex to protect the temp probe count."""
        return self.probe_lock

    def getTempProbeCount(self) -> int:
        """
        Return current temp probe count.

        The caller must hold the lock returned by getTempProbeLock.
        """
        return self.probe_count

    def getStateProbeLock(self) -> threading.Lock:
        """Return the mutex to protect the state probe count."""
        return self.get_state_lock

    def getStateProbeCount(self) -> int:
        """
        Return current state probe count.

        The caller must hold the lock returned by getStateProbeLock.
        """
        return self.get_state_count

    @staticmethod
    def normalizeDrivePath(path: str) -> str:
        """Normalize filepath by following symbolic links, and making it absolute."""
        if os.path.islink(path):
            r = os.readlink(path)
            if not os.path.isabs(r):
                r = os.path.join(os.path.dirname(path), r)
        else:
            r = path
        return os.path.abspath(r)

    @staticmethod
    def isPartition(device_filepath: str) -> bool:
        """Return True if device is a partition, False otherwise."""
        # https://github.com/python/mypy/issues/4177
        assert __class__.normalizeDrivePath(device_filepath) == device_filepath  # type: ignore
        return os.path.isfile(f"/sys/class/block/{os.path.basename(device_filepath)}/partition")

    @staticmethod
    def getParentDevice(device_filepath: str) -> str:
        """Get parent device from a partition device filepath."""
        # https://github.com/python/mypy/issues/4177
        assert __class__.normalizeDrivePath(device_filepath) == device_filepath  # type: ignore
        sysfs_dev = os.readlink(f"/sys/class/block/{os.path.basename(device_filepath)}")
        parent_dev = os.path.basename(os.path.dirname(sysfs_dev))
        return f"/dev/{parent_dev}"


class CPU(HotDevice):

    """CPU device with a sysfs temp sensor."""

    SENSOR_DIGITS_REGEX = re.compile("temp([0-9])*_input$")
    DEFAULT_MIN_TEMP = 30

    def __init__(self, cpu_sensor: str, temp_range: Tuple[float, float]):
        self.cpu_sensor_input_filepath = cpu_sensor
        self.logger = logging.getLogger(self.__class__.__name__)
        self.min_temp, self.max_temp = temp_range
        if self.min_temp is None:
            self.min_temp = self.__class__.DEFAULT_MIN_TEMP
        if self.max_temp is None:
            self.max_temp = self.getDefaultMaxTemp()
        assert 0 < self.min_temp < self.max_temp

    def getSysfsTempValue(self, filepath: str) -> float:
        """Get temperature value from a sysfs file as a float."""
        with open(filepath, "rt") as f:
            return int(f.read()) / 1000

    def getTemperature(self) -> float:
        """See HotDevice.getTemperature."""
        r = self.getSysfsTempValue(self.cpu_sensor_input_filepath)
        self.logger.debug(f"CPU temperature: {r} °C")
        return r

    def getDefaultMaxTemp(self) -> float:
        """Compute maximum temperature."""
        # first compute filepath for max and crit sysfs files
        sensor_match = self.__class__.SENSOR_DIGITS_REGEX.search(self.cpu_sensor_input_filepath)
        assert sensor_match is not None
        sensor_num = int(sensor_match.group(1))
        max_filepath = os.path.join(os.path.dirname(self.cpu_sensor_input_filepath), f"temp{sensor_num}_max")
        crit_filepath = os.path.join(os.path.dirname(self.cpu_sensor_input_filepath), f"temp{sensor_num}_crit")

        # get values
        crit_temp = self.getSysfsTempValue(crit_filepath)
        self.logger.debug(f"Critical CPU temperature: {crit_temp} °C")
        try:
            max_temp = self.getSysfsTempValue(max_filepath)
            self.logger.debug(f"Maximum CPU temperature: {max_temp} °C")
        except FileNotFoundError:
            max_temp = crit_temp - 20

        # some sensors have it reversed...
        max_temp, crit_temp = min(max_temp, crit_temp), max(max_temp, crit_temp)

        # keep a safety margin
        r = max_temp - (crit_temp - max_temp)
        return r

    def getTemperatureRange(self) -> Tuple[float, float]:
        """See HotDevice.getTemperatureRange."""
        return self.min_temp, self.max_temp


class DriveSpinDownThread(threading.Thread):

    """Thread responsible for spinning down a drive when it is not active for a certain amount of time."""

    LOOP_SLEEP_DELAY_S = 60

    def __init__(self, drive: Drive, spin_down_time_s: int):
        super().__init__(name=f"{self.__class__.__name__}-{drive}")
        self.drive = drive
        self.spin_down_time_s = spin_down_time_s
        self.logger = logging.getLogger(self.name)

    def run(self):
        """Thread loop."""
        try:
            previous_stats = None
            while not exit_evt.is_set():
                if self.drive.isSleeping():
                    self.logger.debug("Drive is already sleeping")
                    self.sleep(__class__.LOOP_SLEEP_DELAY_S)
                    continue

                if previous_stats is None:
                    # get stats
                    with self.drive.getTempProbeLock(), self.drive.getStateProbeLock():
                        previous_stats = self.drive.getActivityStats()
                        previous_temp_probe_count = self.drive.getTempProbeCount()
                        previous_state_probe_count = self.drive.getStateProbeCount()
                    previous_stats_time = time.monotonic()

                # sleep
                self.sleep(min(self.spin_down_time_s, __class__.LOOP_SLEEP_DELAY_S))
                if exit_evt.is_set():
                    break

                # get stats again
                with self.drive.getTempProbeLock(), self.drive.getStateProbeLock():
                    stats = self.drive.getActivityStats()
                    temp_probe_count = self.drive.getTempProbeCount()
                    state_probe_count = self.drive.getStateProbeCount()
                now = time.monotonic()
                temp_probe_count_delta = temp_probe_count - previous_temp_probe_count
                state_probe_count_delta = state_probe_count - previous_state_probe_count

                # spin down if needed
                if self.drive.compareActivityStats(
                    previous_stats, stats, temp_probe_count_delta, state_probe_count_delta
                ):
                    self.logger.debug("Drive is active")
                    previous_stats = None
                else:
                    delta = now - previous_stats_time
                    self.logger.debug(f"Drive is inactive for {int(delta / 60)} min")
                    if delta > self.spin_down_time_s:
                        self.drive.spinDown()

            self.logger.info("Exiting")

        except Exception as e:
            self.logger.error(f"{e.__class__.__qualname__}: {e}")

    def sleep(self, s: int) -> None:
        """Sleep for s seconds, or less if exit event occurs."""
        self.logger.debug(f"Sleeping for {s} seconds")
        interrupted = exit_evt.wait(timeout=s)
        if interrupted:
            self.logger.debug("Sleep interrupted")


class Fan:

    """Represent a fan associated with a PWM file to control its speed."""

    LAST_DIGITS_REGEX = re.compile("[^0-9]*([0-9]*)$")

    def __init__(self, id: int, pwm_filepath: str, start_value: int, stop_value: int):
        assert 0 <= start_value <= 255
        assert 0 <= stop_value <= 255
        self.id = id
        self.pwm_filepath = pwm_filepath
        if stat.S_ISBLK(os.stat(self.pwm_filepath).st_mode):
            # we don't want to write to a block device in setPwmValue
            # command line parameters have probably been mixed up
            raise RuntimeError(f"{self.pwm_filepath!r} is a block device, PWM /sys file expected")
        pwm_num_match = self.__class__.LAST_DIGITS_REGEX.search(self.pwm_filepath)
        assert pwm_num_match is not None
        pwm_num = int(pwm_num_match.group(1))
        self.fan_input_filepath = os.path.join(os.path.dirname(self.pwm_filepath), f"fan{pwm_num}_input")
        self.enable_filepath: Optional[str] = f"{self.pwm_filepath}_enable"
        self.start_value = start_value
        self.stop_value = stop_value
        self.startup = False
        self.logger = logging.getLogger(f"Fan #{self.id}")
        self.enable_restore = self.getEnabledValue()
        self.pwm_restore = self.getPwmValue()

    def getRpm(self) -> int:
        """Read fan speed in revolutions per minute."""
        with open(self.fan_input_filepath, "rt") as fan_input_file:
            rpm = int(fan_input_file.read().strip())
        self.logger.debug(f"Rotation speed is currently {rpm} RPM")
        return rpm

    def isRunning(self) -> bool:
        """Return True if fan is moving, False instead."""
        return self.getRpm() > 0

    def isStartingUp(self) -> bool:
        """Return True if fan is starting up, False instead."""
        return self.startup

    def waitStabilize(self) -> None:
        """
        Wait for the fan to have a stable rotational speed.

        The algorithm only works if the fan is either slowing down, accelerating, or steady during the test, not if its
        speed changes quickly ie. going up and down.
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

    def setSpeed(self, target_prct: int) -> None:
        """Set fan speed to a percentage of its maximum speed."""
        # preconditions
        assert 0 <= target_prct <= 100

        self.logger.info(f"Setting fan speed to {target_prct}%")

        # calculate target PWM value
        if target_prct == 0:
            target_value = 0
        else:
            target_value = self.stop_value + ((255 - self.stop_value) * target_prct) // 100

        if (0 < target_value < self.start_value) and (not self.isRunning()):
            self.logger.debug("Applying startup boost")
            self.startup = True
            target_value = max(self.start_value, target_value)
        else:
            self.startup = False

        # set speed
        self.setPwmValue(target_value)

    def setPwmValue(self, value: int) -> None:
        """Set fan PWM value."""
        assert 0 <= value <= 255
        self.setEnabledValue(1)
        self.logger.debug(f"Setting PWM value to {value}")
        with open(self.pwm_filepath, "wt") as pwm_file:
            pwm_file.write(str(value))

    def getEnabledValue(self) -> Optional[int]:
        """Read the enabled value of the fan."""
        if self.enable_filepath is not None:
            try:
                with open(self.enable_filepath, "rt") as enable_file:
                    enabled_val = int(enable_file.read().strip())
                    self.logger.debug(f"Fan is in enabled state {enabled_val}")
                    return enabled_val
            except FileNotFoundError:
                self.enable_filepath = None
        return None

    def setEnabledValue(self, value: int) -> None:
        """
        Set fan enabled value.

        0: no fan speed control (i.e. fan at full speed)
        1: manual fan speed control enabled
        2+: automatic fan speed control enabled
        """
        assert 0 <= value
        if self.enable_filepath is not None:
            try:
                with open(self.enable_filepath, "r+t") as enable_file:
                    enabled_val = int(enable_file.read().strip())
                    if enabled_val != value:
                        self.logger.warning(f"{self.enable_filepath} was {enabled_val}, setting it to {value}")
                        enable_file.seek(0)
                        enable_file.write(str(value))
            except FileNotFoundError:
                self.enable_filepath = None

    def getPwmValue(self) -> int:
        """Read pwm value of the fan."""
        with open(self.pwm_filepath, "rt") as pwm_file:
            pwm = int(pwm_file.read().strip())
        self.logger.debug(f"PWM value is currently {pwm}")
        return pwm

    def restoreFanSettings(self) -> None:
        """Restore fan settings."""
        self.logger.debug(f"Fan PWM is restored to {self.pwm_restore}")
        self.setPwmValue(self.pwm_restore)
        if self.enable_restore is not None:
            self.setEnabledValue(self.enable_restore)
            self.logger.debug(f"Fan enabled state is restored to {self.enable_restore}")


class TestHardware:

    """Run basic drive tests, and analyze fan start/stop behaviour."""

    def __init__(self, drives: Sequence[Drive], fans: Sequence[Fan]):
        self.drives = drives
        self.fans = fans
        self.ok_count = 0
        self.ko_count = 0
        self.logger = logging.getLogger(self.__class__.__name__)

    def run(self) -> None:
        """Run tests on drives and PWMs to recommend parameter values."""
        self.logger.info("Running hardware tests, this may take a few minutes")
        self.testDrives()
        start_stop_values = self.testPwms()
        if self.ko_count > 0:
            print(f"{self.ko_count}/{self.ko_count + self.ok_count} tests failed!")
        else:
            print(f"{self.ok_count}/{self.ok_count} tests OK, all good :)")
        self.logger.info(
            "Recommended parameters: --pwm-start-value %s --pwm-stop-value %s"
            % (
                " ".join(str(min(255, x[0] + 32)) for x in start_stop_values),
                " ".join(str(x[1]) for x in start_stop_values),
            )
        )

    def testDrives(self) -> None:
        """Run tests on hard disc drives."""
        for drive in self.drives:
            self.reportTestGroupStart(f"Test of drive {drive}")

            test_desc = "Getting drive power state"
            self.reportTestStart(test_desc)
            try:
                state = drive.getState()
                test_ok = state in Drive.DriveState
            except Exception:
                test_ok = False
            self.reportTestResult(test_desc, test_ok)

            test_desc = "Getting drive temperature"
            self.reportTestStart(test_desc)
            try:
                temp = drive.getTemperature()
                test_ok = isinstance(temp, int)
            except Exception:
                test_ok = False
            self.reportTestResult(test_desc, test_ok)

            test_desc = "Getting drive activity statistics"
            self.reportTestStart(test_desc)
            try:
                stats = drive.getActivityStats()
                test_ok = isinstance(stats, tuple)
            except Exception:
                test_ok = False
            self.reportTestResult(test_desc, test_ok)

    def testPwms(self) -> Sequence[Tuple[int, int]]:
        """Run tests on PWMs."""
        start_stop_values = []
        pwm_vals = (255,) + tuple(range(240, -1, -16))
        for fan in self.fans:
            self.reportTestGroupStart(f"Test of fan #{fan.id}")
            start_value, stop_value = 255, 0

            test_desc = "Stopping fan"
            self.reportTestStart(test_desc)
            try:
                fan.setPwmValue(0)
                fan.waitStabilize()
                test_ok = not fan.isRunning()
            except Exception:
                test_ok = False
            self.reportTestResult(test_desc, test_ok)

            test_desc = "Starting fan"
            self.reportTestStart(test_desc)
            try:
                fan.setPwmValue(255)
                fan.waitStabilize()
                test_ok = fan.isRunning()
            except Exception:
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
            except Exception:
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
            except Exception:
                pass
            self.reportTestResult(test_desc, test_ok)

            start_stop_values.append((start_value, stop_value))

        return start_stop_values

    def reportTestGroupStart(self, desc: str) -> None:
        """Output text when starting a group of tests."""
        print("%s %s" % (desc, "-" * (shutil.get_terminal_size()[0] - len(desc) - 1)))

    def reportTestStart(self, desc: str) -> None:
        """Output text when starting a test."""
        print(desc, end=" ", flush=True)

    def reportTestResult(self, desc: str, ok: bool) -> None:
        """Output text when ending a test."""
        if ok:
            self.ok_count += 1
        else:
            self.ko_count += 1
        print(("[ %s ]" % ("OK" if ok else "KO")).rjust(shutil.get_terminal_size()[0] - len(desc) - 1))


def test(drive_filepaths: Sequence[str], fan_pwm_filepaths: Sequence[str], hddtemp_daemon_port: int):
    """Entry point to run hardware tests."""
    fans = [Fan(i, fan_pwm_filepath, 0, 0) for i, fan_pwm_filepath in enumerate(fan_pwm_filepaths, 1)]
    drives = [Drive(drive_filepath, hddtemp_daemon_port, 0, 0, False) for drive_filepath in drive_filepaths]

    tester = TestHardware(drives, fans)
    tester.run()


def signal_handler(sig, frame):
    """Signal handler for clean exit."""
    logging.getLogger("Signal handler").info(f"Catched signal {sig}")
    global exit_evt
    exit_evt.set()


def set_high_priority(logger: logging.Logger) -> None:
    """Change process scheduler and priority."""
    # use "real time" scheduler
    done = False
    sched = os.SCHED_RR
    try:
        current_sched = os.sched_getscheduler(0)
    except OSError as e:
        if e.errno != errno.ENOSYS:
            raise
        logger.warning("sched_getscheduler is not supported on this system, leaving scheduler as is")
    else:
        if current_sched == sched:
            # already running with RR scheduler, likely set from init system, don't touch priority
            done = True
        else:
            prio = (os.sched_get_priority_max(sched) - os.sched_get_priority_min(sched)) // 2
            param = os.sched_param(prio)  # type: ignore
            try:
                os.sched_setscheduler(0, sched, param)
            except OSError:
                logger.warning(f"Failed to set real time process scheduler to {sched}, priority {prio}")
            else:
                done = True
                logger.info(f"Process real time scheduler set to {sched}, priority {prio}")

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
            logger.warning(f"Unable to renice process to {target_niceness}, current niceness is {new_niceness}")
        else:
            logger.info(f"Process reniced from {previous_niceness} to {new_niceness}")


def main(  # noqa: C901
    drive_filepaths: Sequence[str],
    cpu_probe_filepath: str,
    fan_pwm_filepaths: Sequence[str],
    fan_start_values: Sequence[int],
    fan_stop_values: Sequence[int],
    min_fan_speed_prct: int,
    min_drive_temp: int,
    max_drive_temp: int,
    cpu_temp_range: Tuple[float, float],
    interval_s: int,
    spin_down_time_s: int,
    hddtemp_daemon_port: Optional[int],
    use_smartctl: bool,
    restore_fan_settings: bool,
):
    """Run main program logic, after handling command line specific stuff."""
    logger = logging.getLogger("Main")
    fans = []
    try:
        # change process priority
        set_high_priority(logger)

        # register signal handler
        signal.signal(signal.SIGINT, signal_handler)
        signal.signal(signal.SIGTERM, signal_handler)

        # init fans
        fans = [
            Fan(i, fan_pwm_filepath, fan_start_value, fan_stop_value)
            for i, (fan_pwm_filepath, fan_start_value, fan_stop_value) in enumerate(
                zip(fan_pwm_filepaths, fan_start_values, fan_stop_values), 1
            )
        ]
        current_fan_speeds: List[Optional[float]] = [None] * len(fans)

        # init devices
        drives = [
            Drive(drive_filepath, hddtemp_daemon_port, min_drive_temp, max_drive_temp, use_smartctl)
            for drive_filepath in drive_filepaths
        ]
        cpus = []
        if cpu_probe_filepath is not None:
            cpus.append(CPU(cpu_probe_filepath, cpu_temp_range))
        drives_startup_time = time.monotonic()

        # start spin down threads if needed
        spin_down_threads = []
        if (spin_down_time_s is not None) and (spin_down_time_s >= interval_s):
            for drive in drives:
                spin_down_threads.append(DriveSpinDownThread(drive, spin_down_time_s))
            for thread in spin_down_threads:
                thread.start()

        while not exit_evt.is_set():
            now = time.monotonic()
            device_temps: Dict[HotDevice, float] = dict()

            # get drive temperatures
            drive_awakes = []
            for drive in drives:
                drive_awake = not drive.isSleeping()
                if drive_awake or drive.supportsProbingWhileAsleep():
                    try:
                        device_temps[drive] = drive.getTemperature()
                    except DriveAsleepError:
                        assert not drive.supportsProbingWhileAsleep()
                        drive_awake = False
                if (not drive_awake) and (not drive.supportsProbingWhileAsleep()):
                    logger.debug(f"Drive {drive} is in low power state, unable to query temperature")
                drive_awakes.append(drive_awake)
            if not device_temps:
                assert not any(drive_awakes)
                logger.info("All drives are in low power state")

            if not any(drive_awakes):
                drives_startup_time = now

            if cpus:
                # get cpu temp
                assert len(cpus) == 1
                device_temps[cpus[0]] = cpus[0].getTemperature()

            if device_temps:
                logger.info(f"Maximum device temperature: {max(device_temps.values())} °C")

            # calc target percentage fan speed
            fan_speed_prct: float = 0
            for device, device_temp in device_temps.items():
                device_temp_range = device.getTemperatureRange()
                if device_temp > device_temp_range[0]:
                    fan_speed_prct = max(
                        fan_speed_prct,
                        100 * (device_temp - device_temp_range[0]) // (device_temp_range[1] - device_temp_range[0]),
                    )
            # cap at 100%
            fan_speed_prct = int(min(fan_speed_prct, 100))
            # enforce min fan speed
            fan_speed_prct = max(fan_speed_prct, min_fan_speed_prct)

            # set fan speed if needed
            for i, fan in enumerate(fans):
                if current_fan_speeds[i] != fan_speed_prct:
                    fan.setSpeed(fan_speed_prct)
                    current_fan_speeds[i] = fan_speed_prct

            # sleep
            if any(map(operator.methodcaller("isStartingUp"), fans)):
                # at least one fan is starting up, quickly cancel startup boost
                current_interval_s = min(MAX_FAN_STARTUP_TIME_S, interval_s)
            elif any(drive_awakes) and ((now - drives_startup_time) < DRIVE_STARTUP_TIME_S):
                # if at least a drive was started or waken up less than 5 min ago, dont' sleep too long because it
                # can heat up quickly
                current_interval_s = min(MAX_DRIVE_STARTUP_SLEEP_INTERVAL_S, interval_s)
            else:
                current_interval_s = interval_s
            logger.debug(f"Sleeping for {current_interval_s} seconds")
            exit_evt.wait(current_interval_s)

        logger.info("Exiting")

        for thread in spin_down_threads:
            thread.join()

    except Exception as e:
        logger.error(f"{e.__class__.__qualname__}: {e}")
        exit_evt.set()

    # restore fan settings or run fans at full speed at exit
    for fan in fans:
        if restore_fan_settings:
            fan.restoreFanSettings()
        else:
            fan.setSpeed(100)


def cl_main():  # noqa: C901
    """Command line entry point."""
    # parse args
    arg_parser = argparse.ArgumentParser(
        description=f"HDD Fan Control v{__version__}.{__doc__}",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    arg_parser.add_argument(
        "-d",
        "--drives",
        required=True,
        nargs="+",
        dest="drive_filepaths",
        help="Drive(s) to get temperature from (ie. /dev/sdX)",
    )
    arg_parser.add_argument(
        "-p",
        "--pwm",
        required=True,
        nargs="+",
        dest="fan_pwm_filepath",
        help="PWM filepath(s) to control fan speed (ie. /sys/class/hwmon/hwmonX/device/pwmY)",
    )
    arg_parser.add_argument(
        "--pwm-start-value",
        type=int,
        default=None,
        nargs="+",
        dest="fan_start_value",
        help="""PWM value (0-255), at which the fan starts moving.
                Use the -t parameter, or run pwmconfig to find this value.""",
    )
    arg_parser.add_argument(
        "--pwm-stop-value",
        type=int,
        default=None,
        nargs="+",
        dest="fan_stop_value",
        help="""PWM value (0-255), at which the fan stop moving.
                Use the -t parameter, or run pwmconfig to find this value.
                Often 20-40 lower than start speed.""",
    )
    arg_parser.add_argument(
        "--min-temp",
        type=int,
        default=30,
        dest="min_temp",
        help="Temperature in Celcius at which the fan(s) will be set to minimum speed.",
    )
    arg_parser.add_argument(
        "--max-temp",
        type=int,
        default=50,
        dest="max_temp",
        help="Temperature in Celcius at which the fan(s) will be set to maximum speed.",
    )
    arg_parser.add_argument(
        "--min-fan-speed-prct",
        type=int,
        default=20,
        dest="min_fan_speed_prct",
        help="""Minimum percentage of full fan speed to set the fan to.
                Never set to 0 unless you have other fans to cool down your system,
                or a case specially designed for passive cooling.""",
    )
    arg_parser.add_argument(
        "-i",
        "--interval",
        type=int,
        default=None,
        dest="interval_s",
        help="Interval in seconds to check temperature and adjust fan speed.",
    )
    arg_parser.add_argument(
        "-c",
        "--cpu-sensor",
        default=None,
        dest="cpu_probe_filepath",
        help="""Also control fan speed according to this CPU temperature probe.
               (ie. /sys/devices/platform/coretemp.0/hwmon/hwmonX/tempY_input).
                WARNING: This is experimental, only use for low TDP CPUs. You
                may need to set a low value for -i/--interval parameter to react quickly to
                sudden CPU temperature increase.""",
    )
    arg_parser.add_argument(
        "--cpu-temp-range",
        type=int,
        nargs=2,
        default=(None, None),
        help="""CPU temperature range, if CPU temp monitoring is enabled.
                If missing, will be autodetected or use a default value.""",
    )
    arg_parser.add_argument(
        "--spin-down-time",
        type=int,
        default=None,
        dest="spin_down_time_s",
        help="Interval in seconds after which inactive drives will be put to standby state.",
    )
    arg_parser.add_argument(
        "-v",
        "--verbosity",
        choices=("warning", "normal", "debug"),
        default="normal",
        dest="verbosity",
        help="Level of logging output",
    )
    arg_parser.add_argument("-b", "--background", action="store_true", dest="daemonize", help="Daemonize process")
    arg_parser.add_argument(
        "-l",
        "--log-file",
        default=None,
        dest="log_filepath",
        help="Filepath for log output when using daemon mode, if omitted, logging goes to syslog.",
    )
    arg_parser.add_argument(
        "--pid-file",
        default="/var/run/hddfancontrol.pid",
        dest="pid_filepath",
        help="Filepath for lock file when using daemon mode",
    )
    arg_parser.add_argument(
        "-t", "--test", action="store_true", default=False, dest="test_mode", help="Run some tests and exit"
    )
    arg_parser.add_argument(
        "--hddtemp-daemon",
        action="store_true",
        default=False,
        dest="hddtemp_daemon",
        help="""Get drive temperature from hddtemp daemon instead of spawning
                a new process each time temperature is probed""",
    )
    arg_parser.add_argument(
        "--hddtemp-daemon-port",
        type=int,
        default=7634,
        dest="hddtemp_daemon_port",
        help="hddtemp daemon port if option --hddtemp-daemon is used",
    )
    arg_parser.add_argument(
        "--smartctl",
        action="store_true",
        default=False,
        help="""Probe temperature using smartctl instead of hddtemp/hdparm/drivetemp (EXPERIMENTAL)""",
    )
    arg_parser.add_argument(
        "--restore-fan-settings",
        action="store_true",
        default=False,
        help="""Restore fan settings on exit, otherwise the fans are run with full speed on exit""",
    )
    args = arg_parser.parse_args()
    if ((args.fan_start_value is not None) and (len(args.fan_pwm_filepath) != len(args.fan_start_value))) or (
        (args.fan_stop_value is not None) and (len(args.fan_pwm_filepath) != len(args.fan_stop_value))
    ):
        print("Invalid parameter count")
        exit(os.EX_USAGE)
    if args.interval_s is None:
        args.interval_s = 60 if (args.cpu_probe_filepath is None) else 10
    if (args.spin_down_time_s is not None) and (args.spin_down_time_s < args.interval_s):
        print(
            f"Spin down time {args.spin_down_time_s}s is lower than temperature check interval {args.interval_s}s.\n"
            "Please set a higher spin down time, or use hdparm's -S switch "
            "to set SATA spin down time."
        )
        exit(os.EX_USAGE)

    # setup logger
    logging_level = {"warning": logging.WARNING, "normal": logging.INFO, "debug": logging.DEBUG}
    logging.getLogger().setLevel(logging_level[args.verbosity])
    logging_fmt_long = "%(asctime)s %(levelname)s [%(name)s] %(message)s"
    logging_fmt_short = "%(levelname)s [%(name)s] %(message)s"
    if args.daemonize:
        if args.log_filepath is not None:
            # log to file
            logging_fmt = logging_fmt_long
            logging_handler = logging.handlers.WatchedFileHandler(args.log_filepath)
        else:
            # log to syslog
            logging_fmt = logging_fmt_short
            logging_handler = LoggingSysLogHandler(syslog.LOG_DAEMON)
        logging_formatter = logging.Formatter(fmt=logging_fmt)
    else:
        # log to stderr
        if sys.stderr.isatty():
            logging_formatter = colored_logging.ColoredFormatter(fmt=logging_fmt_long)
        else:
            # assume systemd service in 'simple' mode
            logging_formatter = logging.Formatter(fmt=logging_fmt_short)
        logging_handler = logging.StreamHandler()
    logging_handler.setFormatter(logging_formatter)
    logging.getLogger().addHandler(logging_handler)

    # check if root
    if os.geteuid() != 0:
        logging.getLogger("Startup").error("You need to run this script as root")
        exit(1)

    # check mandatory deps
    bin_dep.check_bin_dependency(("hdparm",))

    if args.test_mode or (args.fan_start_value is None) or (args.fan_stop_value is None):
        if (args.fan_start_value is None) or (args.fan_stop_value is None):
            logging.getLogger("Startup").warning(
                "Missing --pwm-start-value or --pwm-stop-value argument, running hardware test to find values"
            )
        test(args.drive_filepaths, args.fan_pwm_filepath, args.hddtemp_daemon_port if args.hddtemp_daemon else None)

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
                daemon_context.enter_context(daemon.DaemonContext(pidfile=pidfile, files_preserve=preserved_fds))
            main(
                args.drive_filepaths,
                args.cpu_probe_filepath,
                args.fan_pwm_filepath,
                args.fan_start_value,
                args.fan_stop_value,
                args.min_fan_speed_prct,
                args.min_temp,
                args.max_temp,
                args.cpu_temp_range,
                args.interval_s,
                args.spin_down_time_s,
                args.hddtemp_daemon_port if args.hddtemp_daemon else None,
                args.smartctl,
                args.restore_fan_settings,
            )


if __name__ == "__main__":
    cl_main()
