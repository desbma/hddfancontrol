#!/usr/bin/env python3

import os
import re
import sys

from setuptools import find_packages, setup


if sys.hexversion < 0x3040000:
  print("Python version %s is unsupported, >= 3.4.0 is needed" % (".".join(map(str, sys.version_info[:3]))))
  exit(1)

with open(os.path.join("hddfancontrol", "__init__.py"), "rt") as f:
  version = re.search("__version__ = \"([^\"]+)\"", f.read()).group(1)

with open("requirements.txt", "rt") as f:
  requirements = f.read().splitlines()

with open("README.md", "rt", encoding="utf-8") as f:
  readme = f.read()

setup(name="hddfancontrol",
      version=version,
      author="desbma",
      packages=find_packages(exclude=("tests",)),
      entry_points={"console_scripts": ["hddfancontrol = hddfancontrol:cl_main"]},
      test_suite="tests",
      install_requires=requirements,
      description="Control system fan speed by monitoring hard drive temperature",
      long_description=readme,
      long_description_content_type="text/markdown",
      url="https://github.com/desbma/hddfancontrol",
      download_url="https://github.com/desbma/hddfancontrol/archive/%s.tar.gz" % (version),
      keywords=["hdd", "drive", "temperature", "fan", "control", "speed"],
      classifiers=["Development Status :: 5 - Production/Stable",
                   "Environment :: Console",
                   "Environment :: No Input/Output (Daemon)",
                   "Intended Audience :: End Users/Desktop",
                   "Intended Audience :: System Administrators",
                   "License :: OSI Approved :: GNU General Public License v3 (GPLv3)",
                   "Natural Language :: English",
                   "Operating System :: POSIX :: Linux",
                   "Programming Language :: Python",
                   "Programming Language :: Python :: 3",
                   "Programming Language :: Python :: 3 :: Only",
                   "Programming Language :: Python :: 3.4",
                   "Programming Language :: Python :: 3.5",
                   "Programming Language :: Python :: 3.6",
                   "Programming Language :: Python :: 3.7",
                   "Topic :: System :: Hardware",
                   "Topic :: System :: Monitoring",
                   "Topic :: Utilities"])
