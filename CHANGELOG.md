# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

* refactored: split up bluetooth protocol and the core trainer loop with a trait
* testing: improved test coverage of trainer and TUI

## [0.1.0]

### Added

* Basic ability to connect to a trainer
* Modes: ERG and resistance
* Read data back from a trainer
* Calculating rolling stats over a 3s window
