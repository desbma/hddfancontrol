#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import logging
import os
import unittest
import unittest.mock

import hddfancontrol


class TestDrive(unittest.TestCase):

  def setUp(self):
    with unittest.mock.patch("hddfancontrol.os.stat") as os_stat_mock, \
         unittest.mock.patch("hddfancontrol.stat") as stat_mock, \
         unittest.mock.patch("hddfancontrol.subprocess") as subprocess_mock:
      os_stat_mock.return_value = os.stat_result
      stat_mock.stat.S_IFBLK.return_value = True
      subprocess_mock.check_call.side_effect = subprocess_mock.CalledProcessError(0, "")
      self.drive = hddfancontrol.Drive("/dev/sdz", "/dummy", None)

  def test_getState(self):
    with unittest.mock.patch("hddfancontrol.subprocess") as subprocess_mock:
      subprocess_mock.check_output.return_value = "\n/dev/sdz:\n drive state is:  active/idle\n"
      self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.ACTIVE_IDLE)
    with unittest.mock.patch("hddfancontrol.subprocess") as subprocess_mock:
      subprocess_mock.check_output.return_value = "\n/dev/sdz:\n drive state is:  standby\n"
      self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.STANDBY)
    with unittest.mock.patch("hddfancontrol.subprocess") as subprocess_mock:
      subprocess_mock.check_output.return_value = "\n/dev/sdz:\n drive state is:  sleeping\n"
      self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.SLEEPING)
    with unittest.mock.patch("hddfancontrol.subprocess") as subprocess_mock:
      subprocess_mock.check_output.return_value = "/dev/sdz: No such file or directory\n"
      with self.assertRaises(Exception):
        self.drive.getState()
    with unittest.mock.patch("hddfancontrol.subprocess") as subprocess_mock:
      subprocess_mock.check_output.side_effect = subprocess_mock.CalledProcessError(0, "")
      with self.assertRaises(Exception):
        self.drive.getState()



if __name__ == "__main__":
  # disable logging
  logging.basicConfig(level=logging.CRITICAL + 1)

  # run tests
  unittest.main()
