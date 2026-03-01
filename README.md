# Suffice

*Suffice* is a terminal-based tool for controlling a cycling trainer, so you can pound pedals while peddling code.

## Goals

* modes: ERG, Level, maybe Sim if I'm feeling spicy
* run as a daemon, see status of the trainer in starship
* record FIT (or similar) files to prove your sweat

## Non Goals

* a persistent UI, HUD, etc.
* non-cycling machines

## Roadmap

### v1 Features

* [x] properly wrap the Trainer in an Arc and handle notifications in a thread
* [x] basic REPL controls
* [x] capture basic sensor data: power, speed, cadence
  * [x] and display it usefully
* [x] make sure ERG and Level mode seem to work
* [x] record sessions in FIT files
* [ ] add screenshots/screencasts
* [ ] implement daemon mode
* [ ] starship integration
* [ ] faked distance data (what algorithm does other software use?)
* [ ] proper cli args
  * [ ] help
  * [ ] verbose/quiet mode
  * [ ] connect-test option
* [ ] bling
  * [ ] make gauges for resist/power level
  * [ ] show a basic graph of stats
  * [ ] animations / exhortations

### v2+ Features

* [ ] other connection types: ANT+ / WiFi
* [ ] workouts
* [ ] Sim mode

### DevEx

* [x] tests and measuring test coverage
* [x] CI checks
* [x] crates.io publication
* [x] tokio tracing logs
* [x] better error handling generally instead of reckless unwraps everywhere
* [x] github branch protection
* [ ] docstrings etc.

### Bugs

* [ ] sometimes the FIT file has more than one session -- why?
* [x] sometimes the trainer stops sending data back
* [x] rapid commands only slowly take effect (because we only loop events every 1s)

## Resources Used

* the official [FTMS spec](https://www.bluetooth.org/DocMan/handlers/DownloadDoc.ashx?doc_id=423422)
* relevant prior art in python: [pycycling](https://github.com/zacharyedwardbull/pycycling/blob/master/examples/fitness_machine_service_example.py) the underlying BLE library [bleak](https://github.com/hbldh/bleak) and some useful info about the [trainer](https://github.com/zacharyedwardbull/pycycling/issues/47) I was developing against
* examples from the key rust BLE library [btleplug](https://github.com/deviceplug/btleplug/blob/master/examples/subscribe_notify_characteristic.rs)
* the tokio [examples](https://tokio.rs/tokio/tutorial/shared-state) -- quite useful in the many false starts at working out how to deal with the fundamentally async nature of the system
* not directly relevant, but a fascinating blog post on [*creating* a FTMS](https://ftmsemu.github.io/)
* [making sense](https://github.com/caelansar/termirs/blob/master/src/main.rs) of tokio + ratatui
* [verifying](https://www.fitfileviewer.com/) that the fit files were produced correctly
