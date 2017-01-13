#!/usr/bin/env python3

import logging
import os
import socketserver
import subprocess
import tempfile
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
            unittest.mock.patch("hddfancontrol.subprocess") as subprocess_check_call_mock, \
            unittest.mock.patch("hddfancontrol.Drive.getPrettyName") as drive_getPrettyName:
      os_stat_mock.return_value = os.stat_result
      stat_mock.stat.S_IFBLK.return_value = True
      subprocess_check_call_mock.side_effect = subprocess.CalledProcessError(0, "")
      drive_getPrettyName.return_value = "drive_name"
      self.drive = hddfancontrol.Drive("/dev/sdz", None)
    self.hddtemp_daemon = None

  def tearDown(self):
    if self.hddtemp_daemon is not None:
      self.hddtemp_daemon.server.shutdown()
      self.hddtemp_daemon.server.server_close()
      self.hddtemp_daemon.join()

  def test_getPrettyName(self):
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n\nATA device, with non-removable media\n\tModel Number:       WDC WD4003FZEX-00Z4SA0                  \n\tSerial Number:      WD-WMC5D0D4YY1K\n\tFirmware Revision:  01.01A01\n\tTransport:          Serial, SATA 1.0a, SATA II Extensions, SATA Rev 2.5, SATA Rev 2.6, SATA Rev 3.0\nStandards:\n\tSupported: 9 8 7 6 5 \n\tLikely used: 9\nConfiguration:\n\tLogical\t\tmax\tcurrent\n\tcylinders\t16383\t16383\n\theads\t\t16\t16\n\tsectors/track\t63\t63\n\t--\n\tCHS current addressable sectors:   16514064\n\tLBA    user addressable sectors:  268435455\n\tLBA48  user addressable sectors: 7814037168\n\tLogical  Sector size:                   512 bytes\n\tPhysical Sector size:                  4096 bytes\n\tLogical Sector-0 offset:                  0 bytes\n\tdevice size with M = 1024*1024:     3815447 MBytes\n\tdevice size with M = 1000*1000:     4000787 MBytes (4000 GB)\n\tcache/buffer size  = unknown\n\tNominal Media Rotation Rate: 7200\nCapabilities:\n\tLBA, IORDY(can be disabled)\n\tQueue depth: 32\n\tStandby timer values: spec'd by Standard, with device specific minimum\n\tR/W multiple sector transfer: Max = 16\tCurrent = 0\n\tDMA: mdma0 mdma1 mdma2 udma0 udma1 udma2 udma3 udma4 udma5 *udma6 \n\t     Cycle time: min=120ns recommended=120ns\n\tPIO: pio0 pio1 pio2 pio3 pio4 \n\t     Cycle time: no flow control=120ns  IORDY flow control=120ns\nCommands/features:\n\tEnabled\tSupported:\n\t   *\tSMART feature set\n\t    \tSecurity Mode feature set\n\t   *\tPower Management feature set\n\t   *\tWrite cache\n\t   *\tLook-ahead\n\t   *\tHost Protected Area feature set\n\t   *\tWRITE_BUFFER command\n\t   *\tREAD_BUFFER command\n\t   *\tNOP cmd\n\t   *\tDOWNLOAD_MICROCODE\n\t    \tPower-Up In Standby feature set\n\t   *\tSET_FEATURES required to spinup after power up\n\t    \tSET_MAX security extension\n\t   *\t48-bit Address feature set\n\t   *\tMandatory FLUSH_CACHE\n\t   *\tFLUSH_CACHE_EXT\n\t   *\tSMART error logging\n\t   *\tSMART self-test\n\t   *\tGeneral Purpose Logging feature set\n\t   *\t64-bit World wide name\n\t   *\t{READ,WRITE}_DMA_EXT_GPL commands\n\t   *\tSegmented DOWNLOAD_MICROCODE\n\t   *\tGen1 signaling speed (1.5Gb/s)\n\t   *\tGen2 signaling speed (3.0Gb/s)\n\t   *\tGen3 signaling speed (6.0Gb/s)\n\t   *\tNative Command Queueing (NCQ)\n\t   *\tHost-initiated interface power management\n\t   *\tPhy event counters\n\t   *\tNCQ priority information\n\t   *\tREAD_LOG_DMA_EXT equivalent to READ_LOG_EXT\n\t   *\tDMA Setup Auto-Activate optimization\n\t   *\tSoftware settings preservation\n\t   *\tSMART Command Transport (SCT) feature set\n\t   *\tSCT Write Same (AC2)\n\t   *\tSCT Features Control (AC4)\n\t   *\tSCT Data Tables (AC5)\n\t    \tunknown 206[12] (vendor specific)\n\t    \tunknown 206[13] (vendor specific)\n\t    \tunknown 206[14] (vendor specific)\nSecurity: \n\tMaster password revision code = 65534\n\t\tsupported\n\tnot\tenabled\n\tnot\tlocked\n\tnot\tfrozen\n\tnot\texpired: security count\n\t\tsupported: enhanced erase\n\t424min for SECURITY ERASE UNIT. 424min for ENHANCED SECURITY ERASE UNIT. \nLogical Unit WWN Device Identifier: 50014ee0593d4632\n\tNAA\t\t: 5\n\tIEEE OUI\t: 0014ee\n\tUnique ID\t: 0593d4632\nChecksum: correct\n"
      self.assertEqual(self.drive.getPrettyName(), "sdz WDC WD4003FZEX-00Z4SA0")
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-I", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)

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

  def test_isSleeping(self):
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n drive state is:  active/idle\n"
      self.assertFalse(self.drive.isSleeping())
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n drive state is:  standby\n"
      self.assertTrue(self.drive.isSleeping())
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "\n/dev/sdz:\n drive state is:  sleeping\n"
      self.assertTrue(self.drive.isSleeping())
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
      with self.assertRaises(Exception):
        self.drive.isSleeping()
      subprocess_check_output_mock.assert_called_once_with(("hdparm", "-C", "/dev/sdz"),
                                                           stdin=subprocess.DEVNULL,
                                                           stderr=subprocess.DEVNULL,
                                                           universal_newlines=True)
    with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
      subprocess_check_output_mock.return_value = "/dev/sdz: No such file or directory\n"
      with self.assertRaises(Exception):
        self.drive.isSleeping()
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

  def test_spinDown(self):
    with unittest.mock.patch("hddfancontrol.subprocess.check_call") as subprocess_check_call_mock:
      self.drive.spinDown()
      subprocess_check_call_mock.assert_called_once_with(("hdparm", "-y", "/dev/sdz"),
                                                         stdin=subprocess.DEVNULL,
                                                         stdout=subprocess.DEVNULL,
                                                         stderr=subprocess.DEVNULL)

  def test_getActivityStats(self):
    with self.assertRaises(Exception):
      self.drive.getActivityStats()
    with tempfile.NamedTemporaryFile("wt") as stat_file:
      self.drive.stat_filepath = stat_file.name
      with self.assertRaises(Exception):
        self.drive.getActivityStats()
      stat_file.write("   21695     7718  2913268    95136    13986      754   932032    55820        0    19032   150940\n")
      stat_file.flush()
      self.assertEqual(self.drive.getActivityStats(),
                       (21695, 7718, 2913268, 95136, 13986, 754, 932032, 55820, 0, 19032, 150940))


if __name__ == "__main__":
  # disable logging
  logging.basicConfig(level=logging.CRITICAL + 1)

  # run tests
  unittest.main()
