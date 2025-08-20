# Changelog

## 2.0.5 - 2025-08-20

### <!-- 01 -->💡 Features

- Handle smartctl -A output for NVME ([168ceb1](https://github.com/desbma/hddfancontrol/commit/168ceb1b1666195a3e2d0ae9f834d4efc5c7a032) by desbma)

---

## 2.0.4 - 2025-07-30

### <!-- 01 -->💡 Features

- systemd: Always restart unit ([9b2367a](https://github.com/desbma/hddfancontrol/commit/9b2367a694235fffe7b7489d4c8a84b846081a86) by desbma)
- Parse 'smartctl -A' SCSI temperature output ([3ba1da1](https://github.com/desbma/hddfancontrol/commit/3ba1da1179e47adb0f4ea892070c2f06986b457a) by desbma)

### <!-- 06 -->🚜 Refactor

- Simplify some parsing code ([aa425e9](https://github.com/desbma/hddfancontrol/commit/aa425e9fb5fdb3d0d6bf7de52618182fc6c01890) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Update lints ([9c1c55e](https://github.com/desbma/hddfancontrol/commit/9c1c55e5e84fb6b4750e1f094ca1e94bd3a570d7) by desbma)
- Replace abandoned dependency backoff by backon ([213df14](https://github.com/desbma/hddfancontrol/commit/213df14ab63bfc4f9140230b4d9409762d846410) by desbma)

---

## 2.0.3 - 2025-05-01

### <!-- 01 -->💡 Features

- Set default probe interval to 20s ([bbadf83](https://github.com/desbma/hddfancontrol/commit/bbadf8351b07377c62847c5dac32b4de42df20bd) by desbma)
- Support matching drives by interface type (fixes #69) ([d4076e2](https://github.com/desbma/hddfancontrol/commit/d4076e285243bf6ac9c58fcfd13333680a402ff2) by desbma)

### <!-- 02 -->🐛 Bug fixes

- Minor cliff template fix ([ee327e3](https://github.com/desbma/hddfancontrol/commit/ee327e3842f1ea1186b81585513d7e8d78cc164e) by desbma)

### <!-- 04 -->📗 Documentation

- README: Mention sdparm requirement ([ad353f2](https://github.com/desbma/hddfancontrol/commit/ad353f215e20c65a1dc0f7f812b8cf46fae3be7a) by desbma)

### <!-- 08 -->🏗 Build

- deb: Add optional sdparm dependency ([654a993](https://github.com/desbma/hddfancontrol/commit/654a99326e545a1122931831153eef13645554f4) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Update rust edition & lints ([21da5a9](https://github.com/desbma/hddfancontrol/commit/21da5a9a163564f9a4b934d20484f1a22f7200ef) by desbma)
- Update dependencies ([e6e28d5](https://github.com/desbma/hddfancontrol/commit/e6e28d5e69cf7a36be190aceea57975bf4f63cb3) by desbma)

---

## 2.0.2 - 2025-03-23

### <!-- 01 -->💡 Features

- Support using sdparm to probe for drive state ([fe72479](https://github.com/desbma/hddfancontrol/commit/fe72479b9853e23ee3d4a3affcf38e8c091c4203) by desbma)
- Log state probing method ([ae53c43](https://github.com/desbma/hddfancontrol/commit/ae53c4319d9f7655f38812efcc3cecef7061ffaf) by desbma)

### <!-- 02 -->🐛 Bug fixes

- Detect hdparm -C soft errors ([d5dd2b2](https://github.com/desbma/hddfancontrol/commit/d5dd2b28da822f118a5125600e7e468dcf9ef50b) by desbma)

### <!-- 06 -->🚜 Refactor

- Create separate probe method trait for type erasure ([b85eb25](https://github.com/desbma/hddfancontrol/commit/b85eb25ca713349c426ed11d234ceae717f4f07d) by desbma)

---

## 2.0.1 - 2025-02-15

### <!-- 01 -->💡 Features

- Add error reporting contexts ([aeb47ed](https://github.com/desbma/hddfancontrol/commit/aeb47ed55cb39c8e1849c5c1d8321ab6445af641) by desbma)
- Add more error reporting contexts ([60d533e](https://github.com/desbma/hddfancontrol/commit/60d533e23feb8c2d487b84678f66690d3d3ef9fd) by desbma)
- Support drivers with missing 'pwmX_enable' file ([1879b70](https://github.com/desbma/hddfancontrol/commit/1879b7070d62785e3dda8aaf78a9e208abed3acc) by desbma)

### <!-- 04 -->📗 Documentation

- README: Split crates.io installation instructions ([4bed6bd](https://github.com/desbma/hddfancontrol/commit/4bed6bd183fe4c16e73f743f1367859ac1d5c577) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Update lints for rust 1.84 ([e061ec7](https://github.com/desbma/hddfancontrol/commit/e061ec7e59314119f6e28055302b50ad8ec1d994) by desbma)
- Update dependencies ([0b4630f](https://github.com/desbma/hddfancontrol/commit/0b4630ff236002e568f844f055b1ef8d530fbe27) by desbma)

---

## 2.0.0 - 2025-01-18

### <!-- 02 -->🐛 Bug fixes

- MSRV ([b521f08](https://github.com/desbma/hddfancontrol/commit/b521f08e63d4c59c7f9c51729302e4c0d035e810) by desbma)

### <!-- 04 -->📗 Documentation

- Update changelog template ([1305ee8](https://github.com/desbma/hddfancontrol/commit/1305ee84e291ce13ff87f02e428dd4c17548e674) by desbma)
- README: Reorder badges ([99e668b](https://github.com/desbma/hddfancontrol/commit/99e668b7066c55263b5080c6d05b3d0b8d29c5ba) by desbma)

### <!-- 05 -->🧪 Testing

- Minor cosmetic changes ([10d891a](https://github.com/desbma/hddfancontrol/commit/10d891a2d0dd36bf5c579efec303be1311e41526) by desbma)

### <!-- 09 -->🤖 Continuous integration

- Fix possible incorrect release changelog reference version ([626b38b](https://github.com/desbma/hddfancontrol/commit/626b38bd13be679a7007db0e7a85fcc0242fad7a) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Lint ([ed1f49e](https://github.com/desbma/hddfancontrol/commit/ed1f49ec8d735835921edc5e3ae555c74f6158bc) by desbma)

---

## 2.0.0.b4 - 2025-01-02

### <!-- 01 -->💡 Features

- deb: Compress man pages ([a8b0462](https://github.com/desbma/hddfancontrol/commit/a8b0462fcc903f14db8faf07d041f34a601be779) by desbma)
- Allow changing log level from configuration file ([a847e9d](https://github.com/desbma/hddfancontrol/commit/a847e9d039637f756cd4e34782aa37abe0fb6aac) by desbma)

### <!-- 02 -->🐛 Bug fixes

- Hdparm prober error handling (fix #64) ([9a73731](https://github.com/desbma/hddfancontrol/commit/9a73731953ad85175e91e2e30facdfded5db409a) by desbma)
- Always check subprocess return codes ([376291a](https://github.com/desbma/hddfancontrol/commit/376291afda1ececccd32fd67c53c02e4b6b9e3b7) by desbma)
- Hdparm stderr soft errors ([d1fbe90](https://github.com/desbma/hddfancontrol/commit/d1fbe9090aa95748a2dde2f1ea933a5dd6d1ad50) by desbma)
- Smartctl attribute 190 parsing ([e13a510](https://github.com/desbma/hddfancontrol/commit/e13a510c81903008d4011218960f8f4d2c2b6348) by desbma)

### <!-- 04 -->📗 Documentation

- Minor comment typo ([3f447a3](https://github.com/desbma/hddfancontrol/commit/3f447a3aba84eb453f99bd1a96b6bc319086a541) by desbma)

### <!-- 05 -->🧪 Testing

- Check hdparm prober errors if drive is missing ([8d287d1](https://github.com/desbma/hddfancontrol/commit/8d287d1ecb29e3aca57272fd0686a7d62c24367a) by desbma)
- Add hddtemp prober test for sleeping drive ([ccb44b5](https://github.com/desbma/hddfancontrol/commit/ccb44b5f4de45515ac709654daee4553aa4cd61d) by desbma)

### <!-- 06 -->🚜 Refactor

- Homogeneize Command::output usage ([a29b542](https://github.com/desbma/hddfancontrol/commit/a29b542d36b57c79598ae836fefacc097b41606b) by desbma)

### <!-- 09 -->🤖 Continuous integration

- Build debian package with man pages ([77bac80](https://github.com/desbma/hddfancontrol/commit/77bac80e8cfb3ab992c5675188858c5186a25090) by desbma)

---

## 2.0.0.b3 - 2024-12-24

### <!-- 01 -->💡 Features

- Update lints for rust 1.83 ([7c45bb2](https://github.com/desbma/hddfancontrol/commit/7c45bb290045c06547274b3ee1b31b8b626c8024) by desbma)
- Man page generation ([a29dfdc](https://github.com/desbma/hddfancontrol/commit/a29dfdc3989ea5c6ff40375c3b47e50615f5d0b3) by desbma)

### <!-- 04 -->📗 Documentation

- Add changelog ([d346329](https://github.com/desbma/hddfancontrol/commit/d346329cbd7254490825f1ee1ae7a86bc1751ebb) by desbma)
- README: Add changelog reference ([4e901fb](https://github.com/desbma/hddfancontrol/commit/4e901fb84aa22b13109aa95ada2b1735bf1f7d9f) by desbma)

### <!-- 09 -->🤖 Continuous integration

- Publish changelog with each release ([575bccb](https://github.com/desbma/hddfancontrol/commit/575bccb04473b929dbc579fb7dc1e445e93b01bc) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Update pre-commit hooks ([07ef8dc](https://github.com/desbma/hddfancontrol/commit/07ef8dca2ef7b09ecf047374affc586060f9c3a7) by desbma)
- Update git cliff template ([a69849a](https://github.com/desbma/hddfancontrol/commit/a69849a2898552134ecadc8cdc300bf21f98e9e7) by desbma)

---

## 2.0.0.b2 - 2024-12-12

### <!-- 01 -->💡 Features

- Log chosen probing method ([3c744ef](https://github.com/desbma/hddfancontrol/commit/3c744efb8b5c1e8823257242fedcdd9225fa4187) by desbma)
- pwm-test: Dynamically resolve rpm file path (WIP) ([fa4f283](https://github.com/desbma/hddfancontrol/commit/fa4f283dc24040da1c629a2a8b179aec2e6f44f1) by desbma)
- pwm-test: Dynamically find RPM sysfs filepath for PWM ([fd2c1b9](https://github.com/desbma/hddfancontrol/commit/fd2c1b91ec1bf24a838141234a723c34f86c6e90) by desbma)
- ci: Publish static amd64 binary ([f1349bc](https://github.com/desbma/hddfancontrol/commit/f1349bcc73db1561c9254718fec3c8e4400292ac) by desbma)

### <!-- 04 -->📗 Documentation

- README: Add note about Debian package ([c129e2a](https://github.com/desbma/hddfancontrol/commit/c129e2ad91aab562ba58832490835729809b14d7) by desbma)

### <!-- 06 -->🚜 Refactor

- Use Option::transpose ([86ae9af](https://github.com/desbma/hddfancontrol/commit/86ae9afe017a359b12e28bf9e7bd87f47d35f9ad) by desbma)

---

## 2.0.0.b1 - 2024-11-10

### <!-- 02 -->🐛 Bug fixes

- README: Fedora package URL ([bb14417](https://github.com/desbma/hddfancontrol/commit/bb1441796665ceb16691c09e14c1167ad02e71e8) by desbma)
- Don't interpret pwm 'enable' values other than 0/1 as they may be driver specific ([890a98f](https://github.com/desbma/hddfancontrol/commit/890a98f35694c4be25bbd8de2e0572ba3160fa3c) by desbma)
- Version script beta handling ([4b0f0ad](https://github.com/desbma/hddfancontrol/commit/4b0f0ad1646b18f7f6e9b840493d1b6f0e88d65f) by desbma)

### <!-- 04 -->📗 Documentation

- README: Add crates.io badge ([9036a54](https://github.com/desbma/hddfancontrol/commit/9036a54d05d849b6c9863a724d380c40090ce16c) by desbma)
- README: Update v1 notice ([bb624c8](https://github.com/desbma/hddfancontrol/commit/bb624c884a01c39231b7e100f64c5a3b7540e8ca) by desbma)
- README: Add cargo install instructions ([1e7643f](https://github.com/desbma/hddfancontrol/commit/1e7643fcb394d465fe65e0586820a31b903488c8) by desbma)

### <!-- 09 -->🤖 Continuous integration

- Add Debian package ([bd52d96](https://github.com/desbma/hddfancontrol/commit/bd52d96ac44aa7888dc672aed426ab9664b774c9) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Update release script ([f5d282b](https://github.com/desbma/hddfancontrol/commit/f5d282bf674a8b83c5f2cd31098e9f74bb8756e2) by desbma)
