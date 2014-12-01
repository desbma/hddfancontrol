#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from setuptools import find_packages, setup

VERSION = "1.0.2"


with open("requirements.txt", "rt") as f:
  requirements = f.read().splitlines()

with open("README.md", "rt") as f:
  readme = f.read()

setup(name="hddfancontrol",
      version=VERSION,
      author="desbma",
      packages=find_packages(),
      entry_points={"console_scripts": ["hddfancontrol = hddfancontrol:cl_main"]},
      install_requires=requirements,
      description="Control system fan speed by monitoring hard drive temperature",
      long_description=readme,
      url="https://github.com/desbma/hddfancontrol",
      download_url="https://github.com/desbma/hddfancontrol/archive/%s.tar.gz" % (VERSION),
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
