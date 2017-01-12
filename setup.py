#!/usr/bin/env python3

import os
import re
import sys

from setuptools import find_packages, setup


if sys.hexversion < 0x3030000:
  print("Python version %s is unsupported, >= 3.3.0 is needed" % (".".join(map(str, sys.version_info[:3]))))
  exit(1)

with open(os.path.join("hddfancontrol", "__init__.py"), "rt") as f:
  version = re.search("__version__ = \"([^\"]+)\"", f.read()).group(1)

with open("requirements.txt", "rt") as f:
  requirements = f.read().splitlines()
# require enum34 if enum module is missing (Python 3.3)
try:
  import enum
except ImportError:
  requirements.append("enum34")

try:
  import pypandoc
  readme = pypandoc.convert("README.md", "rst")
except ImportError:
  with open("README.md", "rt") as f:
    readme = f.read()

setup(name="hddfancontrol",
      version=version,
      author="desbma",
      packages=find_packages(),
      entry_points={"console_scripts": ["hddfancontrol = hddfancontrol:cl_main"]},
      test_suite="tests",
      install_requires=requirements,
      description="Control system fan speed by monitoring hard drive temperature",
      long_description=readme,
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
                   "Programming Language :: Python :: 3.3",
                   "Programming Language :: Python :: 3.4",
                   "Programming Language :: Python :: 3.5",
                   "Programming Language :: Python :: 3.6",
                   "Topic :: System :: Hardware",
                   "Topic :: System :: Monitoring",
                   "Topic :: Utilities"])
