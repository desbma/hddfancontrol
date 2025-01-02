# Changelog

## 2.0.0.b4 - 2025-01-02

### <!-- 1 -->🚀 Features

- deb: Compress man pages ([a8b0462](https://github.com/desbma/hddfancontrol/commit/a8b0462fcc903f14db8faf07d041f34a601be779) by desbma)
- Allow changing log level from configuration file ([a847e9d](https://github.com/desbma/hddfancontrol/commit/a847e9d039637f756cd4e34782aa37abe0fb6aac) by desbma)

### <!-- 2 -->🐛 Bug fixes

- Hdparm prober error handling (fix #64) ([9a73731](https://github.com/desbma/hddfancontrol/commit/9a73731953ad85175e91e2e30facdfded5db409a) by desbma)
- Always check subprocess return codes ([376291a](https://github.com/desbma/hddfancontrol/commit/376291afda1ececccd32fd67c53c02e4b6b9e3b7) by desbma)
- Hdparm stderr soft errors ([d1fbe90](https://github.com/desbma/hddfancontrol/commit/d1fbe9090aa95748a2dde2f1ea933a5dd6d1ad50) by desbma)
- Smartctl attribute 190 parsing ([e13a510](https://github.com/desbma/hddfancontrol/commit/e13a510c81903008d4011218960f8f4d2c2b6348) by desbma)

### <!-- 4 -->📚 Documentation

- Minor comment typo ([3f447a3](https://github.com/desbma/hddfancontrol/commit/3f447a3aba84eb453f99bd1a96b6bc319086a541) by desbma)

### <!-- 5 -->🧪 Testing

- Check hdparm prober errors if drive is missing ([8d287d1](https://github.com/desbma/hddfancontrol/commit/8d287d1ecb29e3aca57272fd0686a7d62c24367a) by desbma)
- Add hddtemp prober test for sleeping drive ([ccb44b5](https://github.com/desbma/hddfancontrol/commit/ccb44b5f4de45515ac709654daee4553aa4cd61d) by desbma)

### <!-- 6 -->🚜 Refactor

- Homogeneize Command::output usage ([a29b542](https://github.com/desbma/hddfancontrol/commit/a29b542d36b57c79598ae836fefacc097b41606b) by desbma)

### <!-- 8 -->⚙️ Continuous integration

- Build debian package with man pages ([77bac80](https://github.com/desbma/hddfancontrol/commit/77bac80e8cfb3ab992c5675188858c5186a25090) by desbma)

---

## 2.0.0.b3 - 2024-12-24

### <!-- 1 -->🚀 Features

- Update lints for rust 1.83 ([7c45bb2](https://github.com/desbma/hddfancontrol/commit/7c45bb290045c06547274b3ee1b31b8b626c8024) by desbma)
- Man page generation ([a29dfdc](https://github.com/desbma/hddfancontrol/commit/a29dfdc3989ea5c6ff40375c3b47e50615f5d0b3) by desbma)

### <!-- 4 -->📚 Documentation

- Add changelog ([d346329](https://github.com/desbma/hddfancontrol/commit/d346329cbd7254490825f1ee1ae7a86bc1751ebb) by desbma)
- README: Add changelog reference ([4e901fb](https://github.com/desbma/hddfancontrol/commit/4e901fb84aa22b13109aa95ada2b1735bf1f7d9f) by desbma)

### <!-- 8 -->⚙️ Continuous integration

- Publish changelog with each release ([575bccb](https://github.com/desbma/hddfancontrol/commit/575bccb04473b929dbc579fb7dc1e445e93b01bc) by desbma)

### <!-- 9 -->💼 Miscellaneous tasks

- Update pre-commit hooks ([07ef8dc](https://github.com/desbma/hddfancontrol/commit/07ef8dca2ef7b09ecf047374affc586060f9c3a7) by desbma)
- Update git cliff template ([a69849a](https://github.com/desbma/hddfancontrol/commit/a69849a2898552134ecadc8cdc300bf21f98e9e7) by desbma)

---

## 2.0.0.b2 - 2024-12-12

### <!-- 1 -->🚀 Features

- Log chosen probing method ([3c744ef](https://github.com/desbma/hddfancontrol/commit/3c744efb8b5c1e8823257242fedcdd9225fa4187) by desbma)
- pwm-test: Dynamically resolve rpm file path (WIP) ([fa4f283](https://github.com/desbma/hddfancontrol/commit/fa4f283dc24040da1c629a2a8b179aec2e6f44f1) by desbma)
- pwm-test: Dynamically find RPM sysfs filepath for PWM ([fd2c1b9](https://github.com/desbma/hddfancontrol/commit/fd2c1b91ec1bf24a838141234a723c34f86c6e90) by desbma)
- ci: Publish static amd64 binary ([f1349bc](https://github.com/desbma/hddfancontrol/commit/f1349bcc73db1561c9254718fec3c8e4400292ac) by desbma)

### <!-- 4 -->📚 Documentation

- README: Add note about Debian package ([c129e2a](https://github.com/desbma/hddfancontrol/commit/c129e2ad91aab562ba58832490835729809b14d7) by desbma)

### <!-- 6 -->🚜 Refactor

- Use Option::transpose ([86ae9af](https://github.com/desbma/hddfancontrol/commit/86ae9afe017a359b12e28bf9e7bd87f47d35f9ad) by desbma)

---

## 2.0.0.b1 - 2024-11-10

### <!-- 2 -->🐛 Bug fixes

- README: Fedora package URL ([bb14417](https://github.com/desbma/hddfancontrol/commit/bb1441796665ceb16691c09e14c1167ad02e71e8) by desbma)
- Don't interpret pwm 'enable' values other than 0/1 as they may be driver specific ([890a98f](https://github.com/desbma/hddfancontrol/commit/890a98f35694c4be25bbd8de2e0572ba3160fa3c) by desbma)
- Version script beta handling ([4b0f0ad](https://github.com/desbma/hddfancontrol/commit/4b0f0ad1646b18f7f6e9b840493d1b6f0e88d65f) by desbma)

### <!-- 4 -->📚 Documentation

- README: Add crates.io badge ([9036a54](https://github.com/desbma/hddfancontrol/commit/9036a54d05d849b6c9863a724d380c40090ce16c) by desbma)
- README: Update v1 notice ([bb624c8](https://github.com/desbma/hddfancontrol/commit/bb624c884a01c39231b7e100f64c5a3b7540e8ca) by desbma)
- README: Add cargo install instructions ([1e7643f](https://github.com/desbma/hddfancontrol/commit/1e7643fcb394d465fe65e0586820a31b903488c8) by desbma)

### <!-- 8 -->⚙️ Continuous integration

- Add Debian package ([bd52d96](https://github.com/desbma/hddfancontrol/commit/bd52d96ac44aa7888dc672aed426ab9664b774c9) by desbma)

### <!-- 9 -->💼 Miscellaneous tasks

- Update release script ([f5d282b](https://github.com/desbma/hddfancontrol/commit/f5d282bf674a8b83c5f2cd31098e9f74bb8756e2) by desbma)
