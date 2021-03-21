#!/usr/bin/env python3

""" Hddfancontrol unit tests. """

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

    """ Mock hddtemp daemon. """

    outgoing = b""

    def __init__(self, port):
        socketserver.TCPServer.allow_reuse_address = True
        self.server = socketserver.TCPServer(("127.0.0.1", port), FakeHddtempDaemonHandler)
        super().__init__()

    def run(self):
        """ Thread entry point. """
        self.server.serve_forever()


class FakeHddtempDaemonHandler(socketserver.StreamRequestHandler):

    """ Mock hddtemp daemon connection handler. """

    def handle(self):
        """ See socketserver.StreamRequestHandler.handle. """
        self.wfile.write(FakeHddtempDaemon.outgoing)


class TestDrive(unittest.TestCase):

    """ Main tests class. """

    def setUp(self):
        """ Setups test specific stuff. """
        with unittest.mock.patch("hddfancontrol.os.stat") as os_stat_mock, unittest.mock.patch(
            "hddfancontrol.stat"
        ) as stat_mock, unittest.mock.patch(
            "hddfancontrol.subprocess.check_output"
        ) as subprocess_check_output_mock, unittest.mock.patch(
            "hddfancontrol.Drive.getPrettyName"
        ) as drive_getPrettyName:
            os_stat_mock.return_value = os.stat_result
            stat_mock.stat.S_IFBLK.return_value = True
            subprocess_check_output_mock.return_value = ""
            drive_getPrettyName.return_value = "drive_name"
            self.drive = hddfancontrol.Drive("/dev/_sdz", None, 30, 50, False)
        self.hddtemp_daemon = None

    def tearDown(self):
        """ Cleanup test specific stuff. """
        if self.hddtemp_daemon is not None:
            self.hddtemp_daemon.server.shutdown()
            self.hddtemp_daemon.server.server_close()
            self.hddtemp_daemon.join()

    def test_getPrettyName(self):
        """ Test generation of pretty drive name. """
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n\nATA device, with non-removable media\n\tModel Number:       WDC WD4003FZEX-00Z4SA0                  \n\tSerial Number:      WD-WMC5D0D4YY1K\n\tFirmware Revision:  01.01A01\n\tTransport:          Serial, SATA 1.0a, SATA II Extensions, SATA Rev 2.5, SATA Rev 2.6, SATA Rev 3.0\nStandards:\n\tSupported: 9 8 7 6 5 \n\tLikely used: 9\nConfiguration:\n\tLogical\t\tmax\tcurrent\n\tcylinders\t16383\t16383\n\theads\t\t16\t16\n\tsectors/track\t63\t63\n\t--\n\tCHS current addressable sectors:   16514064\n\tLBA    user addressable sectors:  268435455\n\tLBA48  user addressable sectors: 7814037168\n\tLogical  Sector size:                   512 bytes\n\tPhysical Sector size:                  4096 bytes\n\tLogical Sector-0 offset:                  0 bytes\n\tdevice size with M = 1024*1024:     3815447 MBytes\n\tdevice size with M = 1000*1000:     4000787 MBytes (4000 GB)\n\tcache/buffer size  = unknown\n\tNominal Media Rotation Rate: 7200\nCapabilities:\n\tLBA, IORDY(can be disabled)\n\tQueue depth: 32\n\tStandby timer values: spec'd by Standard, with device specific minimum\n\tR/W multiple sector transfer: Max = 16\tCurrent = 0\n\tDMA: mdma0 mdma1 mdma2 udma0 udma1 udma2 udma3 udma4 udma5 *udma6 \n\t     Cycle time: min=120ns recommended=120ns\n\tPIO: pio0 pio1 pio2 pio3 pio4 \n\t     Cycle time: no flow control=120ns  IORDY flow control=120ns\nCommands/features:\n\tEnabled\tSupported:\n\t   *\tSMART feature set\n\t    \tSecurity Mode feature set\n\t   *\tPower Management feature set\n\t   *\tWrite cache\n\t   *\tLook-ahead\n\t   *\tHost Protected Area feature set\n\t   *\tWRITE_BUFFER command\n\t   *\tREAD_BUFFER command\n\t   *\tNOP cmd\n\t   *\tDOWNLOAD_MICROCODE\n\t    \tPower-Up In Standby feature set\n\t   *\tSET_FEATURES required to spinup after power up\n\t    \tSET_MAX security extension\n\t   *\t48-bit Address feature set\n\t   *\tMandatory FLUSH_CACHE\n\t   *\tFLUSH_CACHE_EXT\n\t   *\tSMART error logging\n\t   *\tSMART self-test\n\t   *\tGeneral Purpose Logging feature set\n\t   *\t64-bit World wide name\n\t   *\t{READ,WRITE}_DMA_EXT_GPL commands\n\t   *\tSegmented DOWNLOAD_MICROCODE\n\t   *\tGen1 signaling speed (1.5Gb/s)\n\t   *\tGen2 signaling speed (3.0Gb/s)\n\t   *\tGen3 signaling speed (6.0Gb/s)\n\t   *\tNative Command Queueing (NCQ)\n\t   *\tHost-initiated interface power management\n\t   *\tPhy event counters\n\t   *\tNCQ priority information\n\t   *\tREAD_LOG_DMA_EXT equivalent to READ_LOG_EXT\n\t   *\tDMA Setup Auto-Activate optimization\n\t   *\tSoftware settings preservation\n\t   *\tSMART Command Transport (SCT) feature set\n\t   *\tSCT Write Same (AC2)\n\t   *\tSCT Features Control (AC4)\n\t   *\tSCT Data Tables (AC5)\n\t    \tunknown 206[12] (vendor specific)\n\t    \tunknown 206[13] (vendor specific)\n\t    \tunknown 206[14] (vendor specific)\nSecurity: \n\tMaster password revision code = 65534\n\t\tsupported\n\tnot\tenabled\n\tnot\tlocked\n\tnot\tfrozen\n\tnot\texpired: security count\n\t\tsupported: enhanced erase\n\t424min for SECURITY ERASE UNIT. 424min for ENHANCED SECURITY ERASE UNIT. \nLogical Unit WWN Device Identifier: 50014ee0593d4632\n\tNAA\t\t: 5\n\tIEEE OUI\t: 0014ee\n\tUnique ID\t: 0593d4632\nChecksum: correct\n"  # noqa: E501
            self.assertEqual(self.drive.getPrettyName(), "_sdz WDC WD4003FZEX-00Z4SA0")
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-I", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

    def test_supportsHitachiTempQuery(self):
        """ Test detection for "Hitachi" temp query. """
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = (
                "\n/dev/_sdz:\n drive temperature (celsius) is:  30\n drive temperature in range:  yes"
            )
            self.assertTrue(self.drive.supportsHitachiTempQuery())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-H", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.STDOUT,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\nSG_IO: questionable sense data, results may be incorrect\n drive temperature (celsius) is: -18\n drive temperature in range: yes"  # noqa: E501
            self.assertFalse(self.drive.supportsHitachiTempQuery())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-H", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.STDOUT,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\nSG_IO: missing sense data, results may be incorrect\n drive temperature (celsius) is: -18\n drive temperature in range: yes"  # noqa: E501
            self.assertFalse(self.drive.supportsHitachiTempQuery())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-H", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.STDOUT,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\nSG_IO: bad/missing sense data, sb[]: 70 00 05 00 00 00 00 0a 04 51 40 00 21 04 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00\n drive temperature (celsius) is: -18\n drive temperature in range: yes"  # noqa: E501
            self.assertFalse(self.drive.supportsHitachiTempQuery())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-H", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.STDOUT,
                universal_newlines=True,
            )

    def test_supportsSctTempQuery(self):
        """ Test detection for "SCT" temp query. """
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = """smartctl 7.0 2018-12-30 r4883 [x86_64-linux-4.19.36-1-lts] (local build)
Copyright (C) 2002-18, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF READ SMART DATA SECTION ===
SCT Status Version:                  3
SCT Version (vendor specific):       258 (0x0102)
Device State:                        Stand-by (1)
Current Temperature:                    39 Celsius
Power Cycle Min/Max Temperature:     18/39 Celsius
Lifetime    Min/Max Temperature:      0/56 Celsius
Under/Over Temperature Limit Count:   0/0
Vendor specific:
01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00

"""
            self.assertTrue(self.drive.supportsSctTempQuery())
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-l", "scttempsts", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = """smartctl 7.0 2018-12-30 r4883 [x86_64-linux-4.19.36-1-lts] (local build)
Copyright (C) 2002-18, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF READ SMART DATA SECTION ===
SCT Commands not supported

"""
            self.assertFalse(self.drive.supportsSctTempQuery())
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-l", "scttempsts", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

    def test_getState(self):
        """ Test drive state identification. """
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n drive state is:  active/idle\n"
            self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.ACTIVE_IDLE)
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n drive state is:  standby\n"
            self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.STANDBY)
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n drive state is:  sleeping\n"
            self.assertEqual(self.drive.getState(), hddfancontrol.Drive.DriveState.SLEEPING)
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
            with self.assertRaises(Exception):
                self.drive.getState()
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "/dev/_sdz: No such file or directory\n"
            with self.assertRaises(Exception):
                self.drive.getState()
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

    def test_isSleeping(self):
        """ Test sleeping device identification. """
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n drive state is:  active/idle\n"
            self.assertFalse(self.drive.isSleeping())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n drive state is:  standby\n"
            self.assertTrue(self.drive.isSleeping())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "\n/dev/_sdz:\n drive state is:  sleeping\n"
            self.assertTrue(self.drive.isSleeping())
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
            with self.assertRaises(Exception):
                self.drive.isSleeping()
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "/dev/_sdz: No such file or directory\n"
            with self.assertRaises(Exception):
                self.drive.isSleeping()
            subprocess_check_output_mock.assert_called_once_with(
                ("hdparm", "-C", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

    def test_getTemperature(self):
        """ Test device temperature probing. """

        #
        # Temperature querying can be done in 5 different ways:
        # * if smartctl use was enabled and SCT is supported => use smartctl -l scttempsts call
        # * if smartctl use was enabled => use smartctl -A call
        # * if drive supports Hitachi-style sensor => use hdparm call
        # * if hddtemp daemon is available => use hddtemp daemon
        # * otherwise use a hddtemp call
        #

        # smartctl -l scttempsts call
        self.drive.supports_hitachi_temp_query = False
        self.drive.hddtemp_daemon_port = None
        self.drive.use_smartctl = True
        self.drive.supports_sct_temp_query = True
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = """smartctl 7.0 2018-12-30 r4883 [x86_64-linux-4.19.36-1-lts] (local build)
Copyright (C) 2002-18, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF READ SMART DATA SECTION ===
SCT Status Version:                  3
SCT Version (vendor specific):       258 (0x0102)
Device State:                        Active (0)
Current Temperature:                    30 Celsius
Power Cycle Min/Max Temperature:     18/40 Celsius
Lifetime    Min/Max Temperature:      0/56 Celsius
Under/Over Temperature Limit Count:   0/0
Vendor specific:
01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00

"""
            self.assertEqual(self.drive.getTemperature(), 30)
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-l", "scttempsts", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

        # smartctl -A call
        self.drive.supports_hitachi_temp_query = False
        self.drive.hddtemp_daemon_port = None
        self.drive.use_smartctl = True
        self.drive.supports_sct_temp_query = False
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = """smartctl 7.0 2018-12-30 r4883 [x86_64-linux-4.19.36-1-lts] (local build)
Copyright (C) 2002-18, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF READ SMART DATA SECTION ===
SMART Attributes Data Structure revision number: 16
Vendor Specific SMART Attributes with Thresholds:
ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  1 Raw_Read_Error_Rate     0x000b   100   100   016    Pre-fail  Always       -       0
  2 Throughput_Performance  0x0005   136   136   054    Pre-fail  Offline      -       80
  3 Spin_Up_Time            0x0007   123   123   024    Pre-fail  Always       -       615 (Average 644)
  4 Start_Stop_Count        0x0012   100   100   000    Old_age   Always       -       540
  5 Reallocated_Sector_Ct   0x0033   100   100   005    Pre-fail  Always       -       0
  7 Seek_Error_Rate         0x000b   100   100   067    Pre-fail  Always       -       0
  8 Seek_Time_Performance   0x0005   124   124   020    Pre-fail  Offline      -       33
  9 Power_On_Hours          0x0012   100   100   000    Old_age   Always       -       1723
 10 Spin_Retry_Count        0x0013   100   100   060    Pre-fail  Always       -       0
 12 Power_Cycle_Count       0x0032   100   100   000    Old_age   Always       -       424
192 Power-Off_Retract_Count 0x0032   100   100   000    Old_age   Always       -       571
193 Load_Cycle_Count        0x0012   100   100   000    Old_age   Always       -       571
194 Temperature_Celsius     0x0002   171   171   000    Old_age   Always       -       35 (Min/Max 13/45)
196 Reallocated_Event_Count 0x0032   100   100   000    Old_age   Always       -       0
197 Current_Pending_Sector  0x0022   100   100   000    Old_age   Always       -       0
198 Offline_Uncorrectable   0x0008   100   100   000    Old_age   Offline      -       0
199 UDMA_CRC_Error_Count    0x000a   200   200   000    Old_age   Always       -       0

"""
            self.assertEqual(self.drive.getTemperature(), 35)
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-A", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

        # smartctl -A call, alternate output
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            # https://github.com/smartmontools/smartmontools/blob/28bd62a76e0e81f336bf44809467c7406866d1ea/www/examples/ST910021AS.txt#L64
            subprocess_check_output_mock.return_value = """smartctl version 5.39 [i386-apple-darwin8.11.1] Copyright (C) 2002-8 Bruce Allen
Home page is http://smartmontools.sourceforge.net/

=== START OF READ SMART DATA SECTION ===
SMART Attributes Data Structure revision number: 10
Vendor Specific SMART Attributes with Thresholds:
ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  1 Raw_Read_Error_Rate     0x000e   100   253   006    Old_age   Always       -       0
  3 Spin_Up_Time            0x0003   092   092   000    Pre-fail  Always       -       0
  4 Start_Stop_Count        0x0032   099   099   020    Old_age   Always       -       1987
  5 Reallocated_Sector_Ct   0x0033   001   001   036    Pre-fail  Always   FAILING_NOW 16642
  7 Seek_Error_Rate         0x000f   070   060   030    Pre-fail  Always       -       21531636184
  9 Power_On_Hours          0x0032   095   095   000    Old_age   Always       -       4957
 10 Spin_Retry_Count        0x0013   100   096   034    Pre-fail  Always       -       0
 12 Power_Cycle_Count       0x0032   099   099   020    Old_age   Always       -       1577
187 Reported_Uncorrect      0x0032   001   001   000    Old_age   Always       -       65535
189 High_Fly_Writes         0x003a   001   001   000    Old_age   Always       -       1050
190 Airflow_Temperature_Cel 0x0022   056   044   045    Old_age   Always   In_the_past 44 (0 56 56 12)
192 Power-Off_Retract_Count 0x0032   100   100   000    Old_age   Always       -       1155
193 Load_Cycle_Count        0x0032   001   001   000    Old_age   Always       -       943182
195 Hardware_ECC_Recovered  0x001a   048   048   000    Old_age   Always       -       80662606
197 Current_Pending_Sector  0x0012   070   069   000    Old_age   Always       -       614
198 Offline_Uncorrectable   0x0010   070   069   000    Old_age   Offline      -       614
199 UDMA_CRC_Error_Count    0x003e   200   200   000    Old_age   Always       -       0
200 Multi_Zone_Error_Rate   0x0000   100   253   000    Old_age   Offline      -       0
202 TA_Increase_Count       0x0032   100   253   000    Old_age   Always       -       0

"""
            self.assertEqual(self.drive.getTemperature(), 44)
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-A", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

        # smartctl -A call, alternate output
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = """smartctl 6.6 2017-11-05 r4594 [x86_64-linux-4.20.17-gentoo] (local build)
Copyright (C) 2002-17, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF SMART DATA SECTION ===
SMART/Health Information (NVMe Log 0x02, NSID 0xffffffff)
Critical Warning: 0x00
Temperature: 37 Celsius
Available Spare: 100%
Available Spare Threshold: 10%
Percentage Used: 0%
Data Units Read: 419.309 [214 GB]
Data Units Written: 379.116 [194 GB]
Host Read Commands: 1.712.794
Host Write Commands: 1.563.538
Controller Busy Time: 10
Power Cycles: 4
Power On Hours: 2
Unsafe Shutdowns: 4
Media and Data Integrity Errors: 0
Error Information Log Entries: 0
Warning Comp. Temperature Time: 0
Critical Comp. Temperature Time: 0

"""
            self.assertEqual(self.drive.getTemperature(), 37)
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-A", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

        # smartctl -A call, alternate output
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = """smartctl version 5.37 [i686-pc-linux-gnu] Copyright (C) 2002-6 Bruce Allen
Home page is http://smartmontools.sourceforge.net/

Current Drive Temperature:     42 C
Drive Trip Temperature:        68 C
Elements in grown defect list: 0
Vendor (Seagate) cache information
  Blocks sent to initiator = 1666124337
  Blocks received from initiator = 1517744621
  Blocks read from cache and sent to initiator = 384030649
  Number of read and write commands whose size <= segment size = 21193148
  Number of read and write commands whose size > segment size = 1278317
Vendor (Seagate/Hitachi) factory information
  number of hours powered up = 19.86
  number of minutes until next internal SMART test = 108

"""
            self.assertEqual(self.drive.getTemperature(), 42)
            subprocess_check_output_mock.assert_called_once_with(
                ("smartctl", "-A", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                universal_newlines=True,
            )

        # hddtemp call
        hddtemp_env = dict(os.environ)
        hddtemp_env["LANG"] = "C"
        self.drive.supports_hitachi_temp_query = False
        self.drive.hddtemp_daemon_port = None
        self.drive.use_smartctl = False
        self.drive.supports_sct_temp_query = False
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "30\n"
            self.assertEqual(self.drive.getTemperature(), 30)
            subprocess_check_output_mock.assert_called_once_with(
                ("hddtemp", "-u", "C", "-n", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=hddtemp_env,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
            with self.assertRaises(Exception):
                self.drive.getTemperature()
            subprocess_check_output_mock.assert_called_once_with(
                ("hddtemp", "-u", "C", "-n", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=hddtemp_env,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "/dev/_sdz: drive_name: drive is sleeping\n"
            with self.assertRaises(hddfancontrol.DriveAsleepError):
                self.drive.getTemperature()
            subprocess_check_output_mock.assert_called_once_with(
                ("hddtemp", "-u", "C", "-n", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=hddtemp_env,
                universal_newlines=True,
            )
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "/dev/_sdz: open: No such file or directory\n\n"
            with self.assertRaises(Exception):
                self.drive.getTemperature()
            subprocess_check_output_mock.assert_called_once_with(
                ("hddtemp", "-u", "C", "-n", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=hddtemp_env,
                universal_newlines=True,
            )

        # hdparm call
        self.drive.supports_hitachi_temp_query = True
        self.drive.use_smartctl = False
        self.drive.supports_sct_temp_query = False
        for self.drive.hddtemp_daemon_port in (None, 12345):
            with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
                subprocess_check_output_mock.return_value = (
                    "/dev/_sdz:\n  drive temperature (celsius) is:  30\n  drive temperature in range:  yes\n"
                )
                self.assertEqual(self.drive.getTemperature(), 30)
                subprocess_check_output_mock.assert_called_once_with(
                    ("hdparm", "-H", "/dev/_sdz"),
                    stdin=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    universal_newlines=True,
                )
            with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
                subprocess_check_output_mock.side_effect = subprocess.CalledProcessError(0, "")
                with self.assertRaises(Exception):
                    self.drive.getTemperature()
                subprocess_check_output_mock.assert_called_once_with(
                    ("hdparm", "-H", "/dev/_sdz"),
                    stdin=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    universal_newlines=True,
                )
            with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
                subprocess_check_output_mock.return_value = "/dev/_sdz: No such file or directory\n"
                with self.assertRaises(Exception):
                    self.drive.getTemperature()
                subprocess_check_output_mock.assert_called_once_with(
                    ("hdparm", "-H", "/dev/_sdz"),
                    stdin=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    universal_newlines=True,
                )

        # hddtemp daemon
        self.drive.supports_hitachi_temp_query = False
        self.drive.hddtemp_daemon_port = 12345
        self.drive.use_smartctl = False
        self.drive.supports_sct_temp_query = False
        with self.assertRaises(Exception):
            self.drive.getTemperature()
        self.hddtemp_daemon = FakeHddtempDaemon(12345)
        self.hddtemp_daemon.start()
        FakeHddtempDaemon.outgoing = b"|/dev/_sdz|DriveSDZ|30|C|"
        self.assertEqual(self.drive.getTemperature(), 30)
        FakeHddtempDaemon.outgoing = b"|/dev_/sdy|DriveSDY|31|C||/dev/_sdz|DriveSDZ|30|C|"
        self.assertEqual(self.drive.getTemperature(), 30)
        FakeHddtempDaemon.outgoing = b"|/dev_/sdy|DriveSDY|31|C||/dev/_sdz|DriveSDZ|30|C||/dev_/sdx|DriveSDX|32|C|"
        self.assertEqual(self.drive.getTemperature(), 30)
        FakeHddtempDaemon.outgoing = b"|/dev/_sdz|DriveSDZ|SLP|*|"
        with self.assertRaises(hddfancontrol.DriveAsleepError):
            self.drive.getTemperature()
        FakeHddtempDaemon.outgoing = b"|/dev/_sdz|DriveSDZ|ERR|*|"
        with self.assertRaises(hddfancontrol.HddtempDaemonQueryFailed):
            self.drive.getTemperatureWithHddtempDaemon()
        with unittest.mock.patch("hddfancontrol.subprocess.check_output") as subprocess_check_output_mock:
            subprocess_check_output_mock.return_value = "30\n"
            self.assertEqual(self.drive.getTemperature(), 30)
            subprocess_check_output_mock.assert_called_once_with(
                ("hddtemp", "-u", "C", "-n", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                env=hddtemp_env,
                universal_newlines=True,
            )
        FakeHddtempDaemon.outgoing = b"|/dev_/sdx|DriveSDX|31|C||/dev_/sdy|DriveSDY|32|C|"
        with self.assertRaises(RuntimeError):
            self.drive.getTemperature()
        FakeHddtempDaemon.outgoing = b"|/dev/_sdz|DriveSDZ|30|F|"
        with self.assertRaises(RuntimeError):
            self.drive.getTemperature()
        FakeHddtempDaemon.outgoing = b""
        with self.assertRaises(Exception):
            self.drive.getTemperature()

    def test_spinDown(self):
        """ Test HDD spin down. """
        with unittest.mock.patch("hddfancontrol.subprocess.check_call") as subprocess_check_call_mock:
            self.drive.spinDown()
            subprocess_check_call_mock.assert_called_once_with(
                ("hdparm", "-y", "/dev/_sdz"),
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

    def test_getActivityStats(self):
        """ Test drive stats fetching. """
        with self.assertRaises(Exception):
            self.drive.getActivityStats()
        with tempfile.NamedTemporaryFile("wt") as stat_file:
            self.drive.stat_filepath = stat_file.name
            with self.assertRaises(Exception):
                self.drive.getActivityStats()
            stat_file.write(
                "   21695     7718  2913268    95136    13986      754   932032    55820        0    19032   150940\n"
            )
            stat_file.flush()
            self.assertEqual(
                self.drive.getActivityStats(),
                (21695, 7718, 2913268, 95136, 13986, 754, 932032, 55820, 0, 19032, 150940),
            )

    def test_compareActivityStats(self):
        """ Test drive stat analysis to detect activity. """
        self.drive.supports_hitachi_temp_query = False
        self.drive.supports_sct_temp_query = False
        self.drive.use_smartctl = False
        only_hddtemp_probe_stats = (
            (
                (
                    3368700,
                    14189,
                    1667115340,
                    13385382,
                    115556,
                    13373,
                    138957136,
                    4019257,
                    0,
                    8704136,
                    17488349,
                    0,
                    0,
                    0,
                    0,
                    3009,
                    83709,
                ),
                (
                    3368705,
                    14189,
                    1667115343,
                    13385427,
                    115556,
                    13373,
                    138957136,
                    4019257,
                    0,
                    8704188,
                    17488394,
                    0,
                    0,
                    0,
                    0,
                    3009,
                    83709,
                ),
            ),
            (
                (49113, 60, 904613, 39145, 17736, 2687, 1083896, 46424, 0, 234520, 86522, 0, 0, 0, 0, 36, 952),
                (49118, 60, 904616, 39553, 17736, 2687, 1083896, 46424, 0, 234940, 86930, 0, 0, 0, 0, 36, 952),
            ),
        )
        for prev_stat, current_stat in only_hddtemp_probe_stats:
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 0, 0), True)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 1, 0), False)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 2, 0), True)

        self.drive.supports_hitachi_temp_query = False
        self.drive.supports_sct_temp_query = True
        self.drive.use_smartctl = True
        only_smartctl_sct_probe_stats = (
            (
                (49675, 60, 904922, 42379, 17736, 2687, 1083896, 46424, 0, 241350, 89756, 0, 0, 0, 0, 36, 952),
                (49679, 60, 904925, 42383, 17736, 2687, 1083896, 46424, 0, 241370, 89760, 0, 0, 0, 0, 36, 952),
            ),
        )
        for prev_stat, current_stat in only_smartctl_sct_probe_stats:
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 0, 0), True)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 1, 0), False)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 2, 0), True)

        self.drive.supports_hitachi_temp_query = True
        self.drive.supports_sct_temp_query = False
        self.drive.use_smartctl = False
        only_hdparm_probe_stats = (
            (
                (49690, 60, 904931, 42390, 17736, 2687, 1083896, 46424, 0, 241440, 89767, 0, 0, 0, 0, 36, 952),
                (49691, 60, 904931, 42390, 17736, 2687, 1083896, 46424, 0, 241450, 89767, 0, 0, 0, 0, 36, 952),
            ),
        )
        for prev_stat, current_stat in only_hdparm_probe_stats:
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 0, 0), True)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 1, 0), False)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 2, 0), True)

        idle_stats = (
            (
                (49113, 60, 904613, 39145, 17736, 2687, 1083896, 46424, 0, 234520, 86522, 0, 0, 0, 0, 36, 952),
                (49113, 60, 904613, 39145, 17736, 2687, 1083896, 46424, 0, 234520, 86522, 0, 0, 0, 0, 36, 952),
            ),
        )
        for prev_stat, current_stat in idle_stats:
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 0, 0), False)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 1, 0), False)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 2, 0), False)

        busy_stats = (
            (
                (49203, 60, 904662, 40016, 17736, 2687, 1083896, 46424, 0, 235960, 87393, 0, 0, 0, 0, 36, 952),
                (49214, 60, 904668, 40023, 17736, 2687, 1083896, 46424, 0, 236050, 87400, 0, 0, 0, 0, 36, 952),
            ),
        )
        for prev_stat, current_stat in busy_stats:
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 0, 0), True)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 1, 0), True)
            self.assertEqual(self.drive.compareActivityStats(prev_stat, current_stat, 2, 0), True)


if __name__ == "__main__":
    # disable logging
    logging.basicConfig(level=logging.CRITICAL + 1)

    # run tests
    unittest.main()
