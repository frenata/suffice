# Suffice

*Suffice* is a terminal-based tool for controlling a cycling trainer, so you can pound pedals while peddling code.

<img width="863" height="301" alt="suffice-data" src="https://github.com/user-attachments/assets/38c073e6-bddc-4336-8c8a-5c17c01d036f" />

## Goals

* modes: ERG, Level, maybe Sim if I'm feeling spicy
* run as a daemon, see status of the trainer in starship
* record FIT (or similar) files to prove your sweat

## Non Goals

* a persistent UI, HUD, etc.
* non-cycling machines

## Resources Used

* the official [FTMS spec](https://www.bluetooth.org/DocMan/handlers/DownloadDoc.ashx?doc_id=423422)
* the official [FIT spec](https://developer.garmin.com/fit/file-types/activity/)
* relevant prior art in python: [pycycling](https://github.com/zacharyedwardbull/pycycling/blob/master/examples/fitness_machine_service_example.py) the underlying BLE library [bleak](https://github.com/hbldh/bleak) and some useful info about the [trainer](https://github.com/zacharyedwardbull/pycycling/issues/47) I was developing against
* examples from the key rust BLE library [btleplug](https://github.com/deviceplug/btleplug/blob/master/examples/subscribe_notify_characteristic.rs)
* the tokio [examples](https://tokio.rs/tokio/tutorial/shared-state) -- quite useful in the many false starts at working out how to deal with the fundamentally async nature of the system
* not directly relevant, but a fascinating blog post on [*creating* a FTMS](https://ftmsemu.github.io/)
* [making sense](https://github.com/caelansar/termirs/blob/master/src/main.rs) of tokio + ratatui
* [verifying](https://www.fitfileviewer.com/) that the fit files were produced correctly
