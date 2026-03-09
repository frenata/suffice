# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

* fix: properly translate the `BikeData` internal units to the FIT file units

## [0.3.0]

### Added

* feature: add a CLI command to generate sample data, very useful for UX testing
* feature: display distance and speed in the TUI
* UX: provide visual feedback on whether recording is active or not
* feature: add distinct TUI views for rolling stats, total stats, or visual charts
* feature: add an in-TUI help screen with all commands

### Fixed

* fix: properly handle all the bluetooth FTMS flags
* fix: add timeouts on bluetooth await code to prevent the `run` loop from ever getting completely stuck

### Improved

* fix: hold a Fixed Deque of `BikeData` instead of an uncapped `Vec`
* chore: compute virtual distance even when not recording
* chore: write logs and output files to a config dir, not `PWD`

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
