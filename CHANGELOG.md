# Changelog

## Unreleased

### <!-- 1 -->🚀 Features

- Update lints for rust 1.83 ([7c45bb2](https://github.com/desbma/hddfancontrol/commit/7c45bb290045c06547274b3ee1b31b8b626c8024) by desbma)

### <!-- 4 -->📚 Documentation

- Add changelog ([d346329](https://github.com/desbma/hddfancontrol/commit/d346329cbd7254490825f1ee1ae7a86bc1751ebb) by desbma)

### <!-- 8 -->⚙️ Continuous integration

- Publish changelog with each release ([4b0bd80](https://github.com/desbma/hddfancontrol/commit/4b0bd80f4ad1f95d0ea6e370bad7b27cb9ecbce7) by desbma)

### <!-- 9 -->💼 Miscellaneous tasks

- Update pre-commit hooks ([0c47e9f](https://github.com/desbma/hddfancontrol/commit/0c47e9fb299d604294b1a59627e491c5078a10a4) by desbma)

---

## 2.0.0.b2 - 2024-12-12

### <!-- 1 -->🚀 Features

- ci: Publish static amd64 binary ([f1349bc](https://github.com/desbma/hddfancontrol/commit/f1349bcc73db1561c9254718fec3c8e4400292ac) by desbma)
- pwm-test: Dynamically find RPM sysfs filepath for PWM ([fd2c1b9](https://github.com/desbma/hddfancontrol/commit/fd2c1b91ec1bf24a838141234a723c34f86c6e90) by desbma)
- pwm-test: Dynamically resolve rpm file path (WIP) ([fa4f283](https://github.com/desbma/hddfancontrol/commit/fa4f283dc24040da1c629a2a8b179aec2e6f44f1) by desbma)
- Log chosen probing method ([3c744ef](https://github.com/desbma/hddfancontrol/commit/3c744efb8b5c1e8823257242fedcdd9225fa4187) by desbma)

### <!-- 4 -->📚 Documentation

- README: Add note about Debian package ([c129e2a](https://github.com/desbma/hddfancontrol/commit/c129e2ad91aab562ba58832490835729809b14d7) by desbma)

### <!-- 6 -->🚜 Refactor

- Use Option::transpose ([86ae9af](https://github.com/desbma/hddfancontrol/commit/86ae9afe017a359b12e28bf9e7bd87f47d35f9ad) by desbma)

---

## 2.0.0.b1 - 2024-11-10

### <!-- 2 -->🐛 Bug fixes

- Version script beta handling ([4b0f0ad](https://github.com/desbma/hddfancontrol/commit/4b0f0ad1646b18f7f6e9b840493d1b6f0e88d65f) by desbma)
- Don't interpret pwm 'enable' values other than 0/1 as they may be driver specific ([890a98f](https://github.com/desbma/hddfancontrol/commit/890a98f35694c4be25bbd8de2e0572ba3160fa3c) by desbma)
- README: Fedora package URL ([bb14417](https://github.com/desbma/hddfancontrol/commit/bb1441796665ceb16691c09e14c1167ad02e71e8) by desbma)

### <!-- 4 -->📚 Documentation

- README: Add cargo install instructions ([1e7643f](https://github.com/desbma/hddfancontrol/commit/1e7643fcb394d465fe65e0586820a31b903488c8) by desbma)
- README: Update v1 notice ([bb624c8](https://github.com/desbma/hddfancontrol/commit/bb624c884a01c39231b7e100f64c5a3b7540e8ca) by desbma)
- README: Add crates.io badge ([9036a54](https://github.com/desbma/hddfancontrol/commit/9036a54d05d849b6c9863a724d380c40090ce16c) by desbma)

### <!-- 8 -->⚙️ Continuous integration

- Add Debian package ([bd52d96](https://github.com/desbma/hddfancontrol/commit/bd52d96ac44aa7888dc672aed426ab9664b774c9) by desbma)

### <!-- 9 -->💼 Miscellaneous tasks

- Update release script ([f5d282b](https://github.com/desbma/hddfancontrol/commit/f5d282bf674a8b83c5f2cd31098e9f74bb8756e2) by desbma)
