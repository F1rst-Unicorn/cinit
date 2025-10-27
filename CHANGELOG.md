# Change Log

All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## Unreleased

### Fixed

* If a cronjob execution time falls into a time gap due to daylight savings time
  or other, don't crash but schedule an execution immediately after the gap.

## [1.6.4]

### Maintenance

* Update library dependencies

## [1.6.3]

### Maintenance

* Update library dependencies

## [1.6.2]

### Maintenance

* Fix linter issues

* Update library dependencies

* Building in podman is now supported

## [1.6.1]

### Maintenance

* Update to Rust Edition 2021

* Update library dependencies

### Fixed

* cinit now properly cleans the supplementary groups of each child process.
  Before every process still had the left-over groups from the root user.

### Maintenance

* Replace [capabilities](https://crates.io/crates/capabilities) by
  [caps](https://crates.io/crates/caps) as the former has been yanked from
  crates.io ([#49](https://j.njsm.de/git/veenj/cinit/issues/49))

* Update library dependencies

* Fix Linter issues

## [1.6.0]

### Added

* Return dedicated exit code if child fails with non-zero exit code
  ([#48](https://j.njsm.de/git/veenj/cinit/issues/48))

* Make exit codes part of the public API, see [Integration](doc/Integration.md)
  ([#48](https://j.njsm.de/git/veenj/cinit/issues/48))

### Maintenance

* Update library dependencies

## [1.5.2]

### Maintenance

* Update library dependencies

* Add `cargo audit` to CI to be informed about security vulnerabilities

* Fix linter issues

## [1.5.1]

### Maintenance

* Update library dependencies

## [1.5.0]

### Added

* Cronjobs may now depend on programs
  ([#46](https://j.njsm.de/git/veenj/cinit/issues/46))

### Maintenance

* Fix linter issues

* Update library dependencies

## [1.4.2]

### Maintenance

* Update library dependencies

## [1.4.1]

### Maintenance

* Fix linter issues

## [1.4.0]

### Maintenance

* Update library dependencies

### Added

* New process type `notify`
  ([#41](https://j.njsm.de/git/veenj/cinit/issues/41))

## [1.3.9]

### Added

* `cinit --version` also prints the commit hash and date of the build

* Publish [tera](https://tera.netlify.app/docs/) as part of the public API
  ([#43](https://j.njsm.de/git/veenj/cinit/issues/43))

* Warn users about template errors
  ([#42](https://j.njsm.de/git/veenj/cinit/issues/42))

### Maintenance

* Update library dependencies

* Fix linter issues of Rust 1.41

## [1.3.8]

### Maintenance

* Update library dependencies

## [1.3.7]

### Maintenance

* Update library dependencies

## [1.3.6]

### Maintenance

* Update library dependencies

* Delegate user/group resolution to library
  ([#18](https://j.njsm.de/git/veenj/cinit/issues/18))

* Fix linter issues of Rust 1.40

## Release [1.3.5]

### Maintenance

* Update library dependencies

* Fix linter issues of Rust 1.39

## Release [1.3.4]

### Fixed

* Don't mask signals in child processes
  ([#38](https://j.njsm.de/git/veenj/cinit/issues/38))

## Release [1.3.3]

### Maintenance

* Fix code issues with rust 1.38

## Release [1.3.2]

### Maintenance

* Update library dependencies

## Release [1.3.1]

### Maintenance

* Update library dependencies

## Release [1.3.0]

### Added

* Let cinit always inherit zombie processes left by its children
  ([#37](https://j.njsm.de/git/veenj/cinit/issues/37))

* Implement
  [drop-in configuration](https://j.njsm.de/git/veenj/cinit/src/branch/master/doc#merging-configuration)
  ([#36](https://j.njsm.de/git/veenj/cinit/issues/36))

## Release [1.2.6]

### Maintenance

* Update to Rust 1.36

* Update library dependencies

## Release [1.2.5]

### Fixed

* Fix accidental dropping of cronjobs
  ([#35](https://j.njsm.de/git/veenj/cinit/issues/35))

## Release [1.2.4]

### Fixed

* Fix panic when having cronjobs between dependent applications
  ([#30](https://j.njsm.de/git/veenj/cinit/issues/30))

* Support cron expressions with wildcard and stepping (`*/5`)
  ([#29](https://j.njsm.de/git/veenj/cinit/issues/29))

* Fix bourne shell compliance in build script
  ([#31](https://j.njsm.de/git/veenj/cinit/issues/31))

## [1.2.3]

### Fixed

* Fix kernel version check for 5.0

## [1.2.2]

### Fixed

* Detect invalid references in dependency specification
  ([#28](https://j.njsm.de/git/veenj/cinit/issues/28))

## [1.2.1]

### Fixed

* Implement startup check for mandatory OS
  properties ([#27](https://j.njsm.de/git/veenj/cinit/issues/27))

## [1.2.0]

### Added

* Implement status reporting ([#4](https://j.njsm.de/git/veenj/cinit/issues/4))

## [1.1.2]

#### Fixed

* Crash when reaping zombies ([#26](https://j.njsm.de/git/veenj/cinit/issues/26))

## [1.1.1]

### Fixed

* Heap corruption in user/group lookups

* Sanitise child environment before applying template engine

## [1.1.0]

### Changed

* Redirect child `stderr` output to own `stderr`
  ([#13](https://j.njsm.de/git/veenj/cinit/issues/13))

* Don't forward missing env variables as empty
  ([#21](https://j.njsm.de/git/veenj/cinit/issues/21))

* Add support for cronjobs
  ([#19](https://j.njsm.de/git/veenj/cinit/issues/19))

* Raise error if user or group in configuration doesn't exist
  ([#23](https://j.njsm.de/git/veenj/cinit/issues/23))

* Clean environment from root-only values when starting child
  ([#24](https://j.njsm.de/git/veenj/cinit/issues/24))

