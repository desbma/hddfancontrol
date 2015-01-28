#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import logging
import os
import socketserver
import subprocess
import threading
import unittest
import unittest.mock

import hddfancontrol


class FakeHddtempDaemon(threading.Thread):

  outgoing = b""

  def __init__(self, port):
    socketserver.TCPServer.allow_reuse_address = True
    self.server = socketserver.TCPServer(("127.0.0.1", port), FakeHddtempDaemonHandler)
    super().__init__()

  def run(self):
    self.server.serve_forever()


class FakeHddtempDaemonHandler(socketserver.StreamRequestHandler):

    def handle(self):
      self.wfile.write(FakeHddtempDaemon.outgoing)


class TestDrive(unittest.TestCase):

  def setUp(self):
    with unittest.mock.patch("hddfancontrol.os.stat") as os_stat_mock, \
         unittest.mock.patch("hddfancontrol.stat") as stat_mock, \
         unittest.mock.patch("hddfancontrol.subprocess") as subprocess_check_call_mock:
      os_stat_mock.return_value = os.stat_result
      stat_mock.stat.S_IFBLK.return_value = True
      subprocess_check_call_mock.side_effect = subprocess.CalledProcessError(0, "")
      self.drive = hddfancontrol.Drive("/dev/sdz", "/dummy", None)
    self.hddtemp_daemon = None

  def tearDown(self):
    if self.hddtemp_daemon is not None:
      self.hddtemp_daemon.server.shutdown()
      self.hddtemp_daemon.server.server_close()
      self.hddtemp_daemon.join()

  def test_getState(self):
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n drive state is:  active/idle\n"
      self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.ACTIVE_IDLE)
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n drive state is:  standby\n"
      self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.STANDBY)
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n drive state is:  sleeping\n"
      self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.SLEEPING)
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
      with self.assertRaises(Exception):
        self.drive.getState()
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "/dev/sdz: No such file or directory\n"
      with self.assertRaises(Exception):
        self.drive.getState()
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)

  def test_getTemperature(self):
    #
    # Temperature querying can be done in 3 different way:
    # * if drive supports Hitachi-style sensor => use hdparm call
    # * if hddtemp daemon is available => use hddtemp daemon
    # * otherwise use a hddtemp call
    #

    # hddtemp call
    self.drive.supports_hitachi_temp_query = False
    self.drive.hddtemp_daemon_port = None
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "30\n"
      self.assertEqual(self.drive.getTemperature(), 30)
      subprocess_check_output_mock.assert_called_once_with(("hddtemp", "-u", "C", "-n", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
      with self.assertRaises(Exception):
        self.drive.getTemperature()
      subprocess_check_output_mock.assert_called_once_with(("hddtemp", "-u", "C", "-n", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "/dev/sdz: open: No such file or directory\n\n"
      with self.assertRaises(Exception):
        self.drive.getTemperature()
      subprocess_check_output_mock.assert_called_once_with(("hddtemp", "-u", "C", "-n", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)

    # hdparm call
    self.drive.supports_hitachi_temp_query = True
    for self.drive.hddtemp_daemon_port in (None, 12345):
      with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
        subprocess_check_output_mock.return_value = "/dev/sdz:\n  drive temperature (celsius) is:  30\n  drive temperature in range:  yes\n"
        self.assertEqual(self.drive.getTemperature(), 30)
        subprocess_check_output_mock.assert_called_once_with(("hdparm", "-H", "/dev/sdz"),
                                                             stdin=subprocess.DEVNULL,
                                                             stderr=subprocess.DEVNULL,
                                                             universal_newlines=True)
      with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
        subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
        with self.assertRaises(Exception):
          self.drive.getTemperature()
        subprocess_check_output_mock.assert_called_once_with(("hdparm", "-H", "/dev/sdz"),
                                                             stdin=subprocess.DEVNULL,
                                                             stderr=subprocess.DEVNULL,
                                                             universal_newlines=True)
      with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
        subprocess_check_output_mock.return_value = "/dev/sdz: No such file or directory\n"
        with self.assertRaises(Exception):
          self.drive.getTemperature()
        subprocess_check_output_mock.assert_called_once_with(("hdparm", "-H", "/dev/sdz"),
                                                             stdin=subprocess.DEVNULL,
                                                             stderr=subprocess.DEVNULL,
                                                             universal_newlines=True)

    # hddtemp daemon
    self.drive.supports_hitachi_temp_query = False
    self.drive.hddtemp_daemon_port = 12345
    with self.assertRaises(Exception):
      self.drive.getTemperature()
    self.hddtemp_daemon = FakeHddtempDaemon(12345)
    self.hddtemp_daemon.start()
    FakeHddtempDaemon.outgoing = b"|/dev/sdz|DriveSDZ|30|C|"
    self.assertEqual(self.drive.getTemperature(), 30)
    FakeHddtempDaemon.outgoing = b"|/dev/sdy|DriveSDY|31|C||/dev/sdz|DriveSDZ|30|C|"
    self.assertEqual(self.drive.getTemperature(), 30)
    FakeHddtempDaemon.outgoing = b"|/dev/sdy|DriveSDY|31|C||/dev/sdz|DriveSDZ|30|C||/dev/sdx|DriveSDX|32|C|"
    self.assertEqual(self.drive.getTemperature(), 30)
    FakeHddtempDaemon.outgoing = b"|/dev/sdx|DriveSDX|31|C||/dev/sdy|DriveSDY|32|C|"
    with self.assertRaises(RuntimeError):
      self.drive.getTemperature()
    FakeHddtempDaemon.outgoing = b"|/dev/sdz|DriveSDZ|30|F|"
    with self.assertRaises(RuntimeError):
      self.drive.getTemperature()
    FakeHddtempDaemon.outgoing = b""
    with self.assertRaises(Exception):
      self.drive.getTemperature()


if __name__ == "__main__":
  # disable logging
  logging.basicConfig(level=logging.CRITICAL + 1)

  # run tests
  unittest.main()
