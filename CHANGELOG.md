# Change Log

All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

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

## Added

* Let cinit always inherit zombie processes left by its children
  ([#37](https://gitlab.com/veenj/cinit/issues/37))

* Implement
  [drop-in configuration](https://gitlab.com/veenj/cinit/tree/master/doc#merging-configuration)
  ([#36](https://gitlab.com/veenj/cinit/issues/36))

## Release [1.2.6]

### Maintenance

* Update to Rust 1.36

* Update library dependencies

## Release [1.2.5]

### Fixed

* Fix accidental dropping of cronjobs
  ([#35](https://gitlab.com/veenj/cinit/issues/35))

## Release [1.2.4]

### Fixed

* Fix panic when having cronjobs between dependent applications
  ([#30](https://gitlab.com/veenj/cinit/issues/30))

* Support cron expressions with wildcard and stepping (`*/5`)
  ([#29](https://gitlab.com/veenj/cinit/issues/29))

* Fix bourne shell compliance in build script
  ([#31](https://gitlab.com/veenj/cinit/issues/31))

## [1.2.3]

### Fixed

* Fix kernel version check for 5.0

## [1.2.2]

### Fixed

* Detect invalid references in dependency specification
  ([#28](https://gitlab.com/veenj/cinit/issues/28))

## [1.2.1]

### Fixed

* Implement startup check for mandatory OS
  properties ([#27](https://gitlab.com/veenj/cinit/issues/27))

## [1.2.0]

### Added

* Implement status reporting ([#4](https://gitlab.com/veenj/cinit/issues/4))

## [1.1.2]

#### Fixed

* Crash when reaping zombies ([#26](https://gitlab.com/veenj/cinit/issues/26))

## [1.1.1]

### Fixed

* Heap corruption in user/group lookups

* Sanitise child environment before applying template engine

## [1.1.0]

### Changed

* Redirect child `stderr` output to own `stderr`
  ([#13](https://gitlab.com/veenj/cinit/issues/13))

* Don't forward missing env variables as empty
  ([#21](https://gitlab.com/veenj/cinit/issues/21))

* Add support for cronjobs
  ([#19](https://gitlab.com/veenj/cinit/issues/19))

* Raise error if user or group in configuration doesn't exist
  ([#23](https://gitlab.com/veenj/cinit/issues/23))

* Clean environment from root-only values when starting child
  ([#24](https://gitlab.com/veenj/cinit/issues/24))

