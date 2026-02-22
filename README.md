# Suffice

Why run elaborate gamified systems in the background, when it *suffice*s to set an ERG target and sweat?

*Suffice* is a terminal-based tool for controlling a cycling trainer, so you can pound pedals while peddling code.

## Goals

* modes: ERG, Level, maybe Sim if I'm feeling spicy
* run as a daemon, see status of the trainer in starship
* record FIT (or similar) files to prove your sweat

## Non Goals

* a persistent UI, HUD, etc.
* non-cycling machines

## Roadmap

* [ ] properly wrap the Trainer in an Arc and handle notifications in a thread
* [ ] basic REPL controls
* [ ] capture basic sensor data: power, speed, cadence and display it
* [ ] make sure ERG and Level mode seem to work
* [ ] record sessions in FIT files
* [ ] implement daemon mode
* [ ] starship integration
