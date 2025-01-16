# Changelog

## Unreleased

### <!-- 02 -->🐛 Bug fixes

- MSRV ([b521f08](https://github.com/desbma/hddfancontrol/commit/b521f08e63d4c59c7f9c51729302e4c0d035e810) by desbma)

### <!-- 04 -->📗 Documentation

- Update changelog template ([7ca6e54](https://github.com/desbma/hddfancontrol/commit/7ca6e543006de55d7e49ebb13806e2629058b5d1) by desbma)

### <!-- 05 -->🧪 Testing

- Minor cosmetic changes ([10d891a](https://github.com/desbma/hddfancontrol/commit/10d891a2d0dd36bf5c579efec303be1311e41526) by desbma)

### <!-- 09 -->🤖 Continuous integration

- Fix possible incorrect release changelog reference version ([626b38b](https://github.com/desbma/hddfancontrol/commit/626b38bd13be679a7007db0e7a85fcc0242fad7a) by desbma)

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

---

## 2.0.0.b0 - 2024-10-21

### <!-- 01 -->💡 Features

- Add opt-in activity stats logging ([b33db9f](https://github.com/desbma/hddfancontrol/commit/b33db9f944938e21a9cb914b9d6281a2a5cc2ba1) by desbma)
- Command line interface ([2da12db](https://github.com/desbma/hddfancontrol/commit/2da12db11bd07acc6f0fe01f1859abdbdca19b10) by desbma)
- PWM & fan code (WIP) ([d462cf6](https://github.com/desbma/hddfancontrol/commit/d462cf61814660098529eb9088d8e1bede6c903a) by desbma)
- Fan thresholds dynamic testing ([3e5c946](https://github.com/desbma/hddfancontrol/commit/3e5c946c417abdc70377c5e2cb44acd155e28ffa) by desbma)
- Drive model ([94f24cf](https://github.com/desbma/hddfancontrol/commit/94f24cfe98c07ee38098e08425cefc944ceadf97) by desbma)
- Drivetemp prober ([772994c](https://github.com/desbma/hddfancontrol/commit/772994c87c1b4eded0c17e35598db1064269c5b8) by desbma)
- Generic temp probers ([bd62cbd](https://github.com/desbma/hddfancontrol/commit/bd62cbd2f2d481578b0fc883618fe54765ad0d83) by desbma)
- Smartctl temp probers ([81cbeda](https://github.com/desbma/hddfancontrol/commit/81cbedab62e1280ee34779e5ad9da08c30f18b9c) by desbma)
- Hdparm temp prober ([860a536](https://github.com/desbma/hddfancontrol/commit/860a536755c6b37213998aa0ab13e6fe96217cbb) by desbma)
- Hddtemp temp probers ([1290f93](https://github.com/desbma/hddfancontrol/commit/1290f93eeae8e49f0e8bcf9f555dcb2cec8a674e) by desbma)
- Drive runtime state ([5af4995](https://github.com/desbma/hddfancontrol/commit/5af4995436a7be0944e739df9b11c0596bf93af5) by desbma)
- Fan control loop ([efe626e](https://github.com/desbma/hddfancontrol/commit/efe626ec4361fdfbc28c01832c7269448da62274) by desbma)
- Restore pwm state or set full speed on exit ([b918008](https://github.com/desbma/hddfancontrol/commit/b918008ff055cb7c38f14dbadffbc4066149370a) by desbma)
- Cpu temperature monitoring ([6015f2d](https://github.com/desbma/hddfancontrol/commit/6015f2d7f93f4a98973d13fe7545071613a13493) by desbma)
- Filter drive probing by state ([4f0bf67](https://github.com/desbma/hddfancontrol/commit/4f0bf673d777b34f655f94be7f25e3598b34a82f) by desbma)
- Improve logging ([581072e](https://github.com/desbma/hddfancontrol/commit/581072e1945d3c1f22c4294041beb05c287ef0cf) by desbma)
- Cl goodies ([d73309e](https://github.com/desbma/hddfancontrol/commit/d73309e62e992ee8abd660f46baec97ea440840e) by desbma)
- Fan start/stop thresholds ([ffa4172](https://github.com/desbma/hddfancontrol/commit/ffa41721ec744c899d6926818c886069d226698f) by desbma)
- Pwm logging ([7268a79](https://github.com/desbma/hddfancontrol/commit/7268a7988379736702797630eef6875eeafe92ce) by desbma)
- Minor CL doc improvements ([ca5e143](https://github.com/desbma/hddfancontrol/commit/ca5e1432f2372f676c71fc74b1f51e34876288bf) by desbma)
- Support arbitrary hwmon sensors (#55) ([345d4bc](https://github.com/desbma/hddfancontrol/commit/345d4bc38a73475df1529a0928c7b33c1680ab05) by desbma)
- Pretty hwmon logging ([f2e3826](https://github.com/desbma/hddfancontrol/commit/f2e3826de216676527ee1ba0d93713a6f31cb3d1) by desbma)
- Add cargo package metadata for publishing ([970cd99](https://github.com/desbma/hddfancontrol/commit/970cd997ddc4cffb8d1515a9a6f15901839b0cac) by desbma)

### <!-- 02 -->🐛 Bug fixes

- Setup.py version check ([51d623e](https://github.com/desbma/hddfancontrol/commit/51d623e9db049f72a962de05bd420b5a2eb1b173) by desbma)
- Fixes ([7f562bd](https://github.com/desbma/hddfancontrol/commit/7f562bd113d4a2a8e86cd581c0c32c23fea10b4e) by desbma)
- Exit hook ([1b67cd8](https://github.com/desbma/hddfancontrol/commit/1b67cd8705d65c4c287e7d6665951e479e93faa0) by desbma)
- Removed unneeded systemd service options ([5f67cce](https://github.com/desbma/hddfancontrol/commit/5f67cce18adf469b92c19cd8c8147772dc2d9247) by desbma)
- Retry if PWM path is not immediately available ([ce15edc](https://github.com/desbma/hddfancontrol/commit/ce15edc721cfcd3ccb203d265fcb0cb6c032e5fc) by desbma)
- Allow scttempsts probing method for asleep drives (see #56) ([c9a28e9](https://github.com/desbma/hddfancontrol/commit/c9a28e987680225222f58b51dd246428a7597b9e) by desbma)

### <!-- 04 -->📗 Documentation

- Update README for v2 ([b05d68b](https://github.com/desbma/hddfancontrol/commit/b05d68b53869a6a74eec3b9b8a5b2e23fd01f9ab) by desbma)
- Minor README edits ([f0ebd94](https://github.com/desbma/hddfancontrol/commit/f0ebd948821851bce9dae72ac496db75a87a40c6) by desbma)
- Add README v1 notice ([5ebe60b](https://github.com/desbma/hddfancontrol/commit/5ebe60b8ea566d4733e1ee26e0733cd57a122631) by desbma)

### <!-- 05 -->🧪 Testing

- Add unit test for fan taget_speed ([6d0c59c](https://github.com/desbma/hddfancontrol/commit/6d0c59cab758389b6342da270fae00e95f8cb61e) by desbma)
- Add test for Fan::set_speed ([e3ae3f6](https://github.com/desbma/hddfancontrol/commit/e3ae3f639dc0a091f6cced20aeb0c5d3c2fa72af) by desbma)

### <!-- 06 -->🚜 Refactor

- Rewrite speed calculations ([fb94e45](https://github.com/desbma/hddfancontrol/commit/fb94e456b7aad56393190549f508f273b3644768) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Change pre-commit hooks and formatter ([6cdc7d1](https://github.com/desbma/hddfancontrol/commit/6cdc7d1ecdc8fb789fb370e2bd91f1ca640ee4ca) by desbma)
- Drop Python <3.8 support + officially support 3.12 ([7683aea](https://github.com/desbma/hddfancontrol/commit/7683aea267ff09a20ff82ad7ae2ba0a79168a448) by desbma)
- Drop coveralls + update/fix badges ([b6c182f](https://github.com/desbma/hddfancontrol/commit/b6c182f65e8b632140bacf90f4c74779d3990ad3) by desbma)
- Rust skeleton ([f12699b](https://github.com/desbma/hddfancontrol/commit/f12699b8fab0a908078687894acf90d5e39338df) by desbma)
- Cargo.toml tweaks ([efcceae](https://github.com/desbma/hddfancontrol/commit/efcceae158e8364b465ec12b7388ad3b02ac1126) by desbma)
- Lint ([a138de3](https://github.com/desbma/hddfancontrol/commit/a138de3ca41269fe8f292d70801972cf32864d8c) by desbma)
- Update lints ([a48e2e3](https://github.com/desbma/hddfancontrol/commit/a48e2e31feec5c4ee5ce5de66f841801d68f7614) by desbma)
- Update lints ([9692257](https://github.com/desbma/hddfancontrol/commit/96922572e68efcdc9058686faf91690c991859be) by desbma)
- Update dependencies ([b76da32](https://github.com/desbma/hddfancontrol/commit/b76da32f9335fae01d4ba0feec615b3aefb8e44a) by desbma)

---

## 1.6.2 - 2024-06-16

### <!-- 01 -->💡 Features

- Add more exception logging ([2028113](https://github.com/desbma/hddfancontrol/commit/202811314ac999e119779a44c0efcfc9050b6f86) by desbma)

### <!-- 02 -->🐛 Bug fixes

- Remove duplicate release script ([3b05d43](https://github.com/desbma/hddfancontrol/commit/3b05d43c0b46f64185efc9f42f678ea1db5f0603) by desbma)

### <!-- 04 -->📗 Documentation

- Minor README updates ([3b0789c](https://github.com/desbma/hddfancontrol/commit/3b0789c898e519c8533457f094cfd02b1f07b261) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Fix flake8 warning ([ce83d42](https://github.com/desbma/hddfancontrol/commit/ce83d42246d168f86d93fd852c9b07feb1bf7309) by desbma)

---

## 1.6.0 - 2023-12-17

### <!-- 01 -->💡 Features

- Support libc without sched_getscheduler (#44) ([3410e6c](https://github.com/desbma/hddfancontrol/commit/3410e6ce9bf6fcf936453c97f004e105048e3688) by Petr Řehoř)
- Use unknown state if hdparm fails (merges #46) ([f74a475](https://github.com/desbma/hddfancontrol/commit/f74a475472d94a00ab3d07bbd189dba9b98c1df6) by desbma)
- Get SAS model name ([19a26dd](https://github.com/desbma/hddfancontrol/commit/19a26dda71c0641a2aa41c654d820c3b757f3f13) by desbma)

### <!-- 05 -->🧪 Testing

- Improve getPrettyName coverage ([39fbafa](https://github.com/desbma/hddfancontrol/commit/39fbafa96424a03ee6f18a10ab1ee0beb929ce84) by desbma)

---

## 1.5.1 - 2023-06-25

### <!-- 02 -->🐛 Bug fixes

- Allow pwmX_enable file to be absent ([47d9de3](https://github.com/desbma/hddfancontrol/commit/47d9de32fed26829e46a8cc748173a3e4738f51d) by desbma)
- Handling of some hdparm state output ([4e13dc6](https://github.com/desbma/hddfancontrol/commit/4e13dc67a062ef9eb193ee367dea706e60ae38b5) by desbma)
- Don't require hddtemp/hdparm for tests ([f57d8b6](https://github.com/desbma/hddfancontrol/commit/f57d8b6775e5dd170a014044b78c5fa04f158f49) by desbma)

### <!-- 10 -->🧰 Miscellaneous tasks

- Update pre-commit hooks ([dbf581d](https://github.com/desbma/hddfancontrol/commit/dbf581d0bcde9b787fb30eb5424a5d3d22e24edb) by desbma)
- Update pre-commit hooks ([3ed7f26](https://github.com/desbma/hddfancontrol/commit/3ed7f26a3a6721ca313b82fdca0314b06c04b680) by desbma)
- Drop Python <3.7, officially support 3.10 & 3.11 ([8795bea](https://github.com/desbma/hddfancontrol/commit/8795bea2ba0aa28fd425bccc96b233a7931a84db) by desbma)

---

## 1.5.0 - 2021-11-14

---

## 1.4.3 - 2021-04-07

---

## 1.4.2 - 2021-03-21

---

## 1.4.1 - 2021-03-20

---

## 1.4.0 - 2021-03-02

### CI

- Improve config ([314354a](https://github.com/desbma/hddfancontrol/commit/314354a709f095f494d7cc0746ae6fe1015b5927) by desbma)
- Coveralls fix ([2d65fb4](https://github.com/desbma/hddfancontrol/commit/2d65fb406abb0889ec71a49e25223e8ba6a90c83) by desbma)

---

## 1.3.1 - 2019-11-16

---

## 1.3.0 - 2019-09-21

### README

- FIx typos ([6e42455](https://github.com/desbma/hddfancontrol/commit/6e42455fbc6f59f58a5b65500822779b3d952013) by desbma)

---

## 1.2.10 - 2019-01-26

---

## 1.2.9 - 2019-01-16

### README

- Fix systemd service install command - 2 ([e47a318](https://github.com/desbma/hddfancontrol/commit/e47a318a7c8485e02a1a36afb088bb36aed87689) by desbma)
- Use canonical systemd path ([5f983a5](https://github.com/desbma/hddfancontrol/commit/5f983a51b3a3e6fe4f2345ad195d7e6b723a42c2) by desbma)

---

## 1.2.8 - 2018-02-04

### README

- Update AUR package name ([a70a5b7](https://github.com/desbma/hddfancontrol/commit/a70a5b7345834bfc46314b7070c0cfa4e8c2214c) by desbma)
- Fix systemd service install command ([7c15822](https://github.com/desbma/hddfancontrol/commit/7c158221b93e06a020a27719e6a45f79364686b8) by desbma)

---

## 1.2.7 - 2017-02-25

---

## 1.2.6 - 2017-01-30

---

## 1.2.5 - 2017-01-13

---

## 1.2.4 - 2016-07-24

---

## 1.2.3 - 2015-12-14

---

## 1.2.2 - 2015-11-02

---

## 1.2.1 - 2015-09-19

---

## 1.2.0 - 2015-06-06

---

## 1.1.3 - 2015-06-05

---

## 1.1.2 - 2015-01-31

---

## 1.1.1 - 2015-01-16

---

## 1.1.0 - 2015-01-16

---

## 1.0.3 - 2014-12-21

---

## 1.0.2 - 2014-12-01

---

## 1.0.1 - 2014-11-30

---

## 1.0.0 - 2014-11-30
