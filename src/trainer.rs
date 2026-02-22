use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::api::{Characteristic, WriteType};
use btleplug::platform::{Manager, Peripheral};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tokio_stream::StreamExt;

use crate::ftms::*;

#[derive(Debug)]
struct Range {
    min: u16,
    max: u16,
    inc: u16,
}

impl Range {
    fn from_bytes(bytes: Vec<u8>) -> Range {
        Range {
            min: u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            max: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            inc: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct Trainer {
    peri: Peripheral,
    resistance_range: Option<Range>,
    power_range: Option<Range>,
    control: Option<Characteristic>,
}

#[derive(Debug, Clone)]
pub struct TrainerHandle {
    pub trainer: Arc<Mutex<Trainer>>,
}

impl TrainerHandle {
    pub async fn find(target: String) -> Option<TrainerHandle> {
        let manager = Manager::new().await.unwrap();

        // get the first bluetooth adapter
        let adapters = manager.adapters().await.ok()?;
        let central = adapters.into_iter().nth(0).unwrap();

        // start scanning for devices
        central.start_scan(ScanFilter::default()).await.ok()?;
        // instead of waiting, you can use central.events() to get a stream which will
        // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
        time::sleep(Duration::from_secs(2)).await;
        for p in central.peripherals().await.ok()? {
            if let Some(props) = p.properties().await.ok()? {
                // eprintln!("{:?}", props.local_name);
                if let Some(name) = props.local_name
                    && name == target
                {
                    let trainer = Arc::new(Mutex::new(Trainer {
                        peri: p,
                        resistance_range: None,
                        power_range: None,
                        control: None,
                    }));
                    return Some(TrainerHandle { trainer });
                }
            }
        }

        None
    }

    pub async fn connect(self: &Self) {
        let mut trainer = self.trainer.lock().await;
        let _ = trainer.peri.connect().await;
        let _ = trainer.peri.discover_services().await;
        for c in trainer.peri.characteristics() {
            if c.uuid == RESISTANCE_RANGE {
                if let Ok(res) = trainer.peri.read(&c).await {
                    trainer.resistance_range = Some(Range::from_bytes(res));
                }
            }

            if c.uuid == POWER_RANGE {
                if let Ok(res) = trainer.peri.read(&c).await {
                    trainer.power_range = Some(Range::from_bytes(res));
                }
            }

            // TODO: See bitmasks here https://github.com/zacharyedwardbull/pycycling/blob/master/pycycling/ftms_parsers/fitness_machine_feature.py#L132
            // if c.uuid == FEATURES {
            //     eprintln!("Features: {:?}", c);
            //     let res = self.peri.read(&c).await;
            //     eprintln!("{:?}\n", res);
            // }
        }
        eprintln!("Resistance: {:?}", trainer.resistance_range);
        eprintln!("Power: {:?}", trainer.power_range);

        for c in trainer.peri.characteristics() {
            if c.uuid == MACHINE_STATUS {
                let res = trainer.peri.subscribe(&c).await;
                eprintln!("Subscribed to machine status: {:?}", res);
            }

            if c.uuid == TRAINING_STATUS {
                let res = trainer.peri.subscribe(&c).await;
                eprintln!("Subscribed to training status: {:?}", res);
            }

            if c.uuid == MACHINE_CONTROL {
                let res = trainer.peri.subscribe(&c).await;
                trainer.control = Some(c);
                eprintln!("Subscribed to control: {:?}", res);
            }
        }
    }

    pub async fn set_resistance(self: &Self, level: u16) {
        let trainer = self.trainer.lock().await;

        if let Some(range) = &trainer.resistance_range {
            if level > range.max || level < range.min || level % range.inc != 0 {
                panic!("out of range")
            }
        } else {
            panic!("cannot set resistance");
        }

        // let data: Vec<u8> = vec![1];
        let res = trainer
            .peri
            .write(
                trainer.control.as_ref().unwrap(),
                &vec![0],
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);

        let res = trainer
            .peri
            .write(
                trainer.control.as_ref().unwrap(),
                &vec![1],
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);

        let res = trainer
            .peri
            .write(
                trainer.control.as_ref().unwrap(),
                &vec![4, level as u8], // FIXME: need to send level as a LE byte array
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);
    }

    pub async fn notifications(handle: Arc<Mutex<Trainer>>) {
        let trainer = handle.lock().await;

        let get_notif = trainer.peri.notifications().await;
        if get_notif.is_err() {
            panic!("failed to get notifs")
        }

        let mut notify = get_notif.unwrap();
        println!("ready for notifs");
        while let Some(v) = notify.next().await {
            println!("GOT = {:?}", v);
        }
    }
}
