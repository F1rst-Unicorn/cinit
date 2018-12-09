# Change Log

All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

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

