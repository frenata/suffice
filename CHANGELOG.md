# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Nothing!

## [0.2.0]

### Added

* feature: capability to record fit files

### Fixed

* bugfix: prevent Trainer loop from exiting without recovering

### Improved

* improvement: provide backpressure/dedupe on commands to the trainer so it feels more responsive
* improvement: friendly CLI parsing

## [0.1.0]

### Added

* Basic ability to connect to a trainer
* Modes: ERG and resistance
* Read data back from a trainer
* Calculating rolling stats over a 3s window
