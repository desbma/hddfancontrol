# Changelog

## [unreleased]

### 🚀 Features

- Update lints for rust 1.83

## [2.0.0.b2] - 2024-12-12

### 🚀 Features

- Log chosen probing method
- *(pwm-test)* Dynamically resolve rpm file path (WIP)
- *(pwm-test)* Dynamically find RPM sysfs filepath for PWM
- *(ci)* Publish static amd64 binary

### 🚜 Refactor

- Use Option::transpose

### 📚 Documentation

- *(README)* Add note about Debian package

### ⚙️ Miscellaneous Tasks

- Version 2.0.0.b2

## [2.0.0.b1] - 2024-11-10

### 🐛 Bug Fixes

- *(README)* Fedora package URL
- Don't interpret pwm 'enable' values other than 0/1 as they may be driver specific
- Version script beta handling

### 📚 Documentation

- *(README)* Add crates.io badge
- *(README)* Update v1 notice
- *(README)* Add cargo install instructions

### ⚙️ Miscellaneous Tasks

- Update release script
- Add Debian package
- Version 2.0.0.b1

