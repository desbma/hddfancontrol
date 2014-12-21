#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import re

from setuptools import find_packages, setup


with open(os.path.join("hddfancontrol", "__init__.py"), "rt") as f:
  version = re.search("__version__ = \"([^\"]+)\"", f.read()).group(1)

with open("requirements.txt", "rt") as f:
  requirements = f.read().splitlines()

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
                   "Topic :: System :: Hardware",
                   "Topic :: System :: Monitoring",
                   "Topic :: Utilities"])
