use btleplug::api::{Central, Manager as _, Peripheral as PeriTrait, ScanFilter};
use btleplug::api::{Characteristic, WriteType};
use btleplug::platform::{Manager, Peripheral};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time;
use tokio_stream::StreamExt;

use crate::ftms::*;

pub enum Command {
    Reset,
    Resist(u16),
    Power(i16),
}

#[derive(Debug)]
struct Range {
    #[allow(unused)]
    min: u16,
    #[allow(unused)]
    max: u16,
    #[allow(unused)]
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
pub struct Trainer<T: PeriTrait> {
    peri: T,
    resistance_range: Option<Range>,
    power_range: Option<Range>,
    control: Option<Characteristic>,
}

impl<T: PeriTrait> Trainer<T> {
    pub async fn reset(&self) {
        // 1. Request Control
        // 2. Reset params (which gives up control!)
        // 3. Request Control again!

        let res = self
            .peri
            .write(
                self.control.as_ref().unwrap(),
                &[0],
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);

        let res = self
            .peri
            .write(
                self.control.as_ref().unwrap(),
                &[1],
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);

        let res = self
            .peri
            .write(
                self.control.as_ref().unwrap(),
                &[0],
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);
    }

    pub async fn set_resistance(&self, level: u16) {
        let mut data = u16::to_le_bytes(level).to_vec();
        data.insert(0, 4);
        let res = self
            .peri
            .write(
                self.control.as_ref().unwrap(),
                &data,
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);
    }

    pub async fn set_power(&self, level: i16) {
        let mut data = i16::to_le_bytes(level).to_vec();
        data.insert(0, 5);
        let res = self
            .peri
            .write(
                self.control.as_ref().unwrap(),
                &data,
                WriteType::WithResponse,
            )
            .await;
        eprintln!("{:?}", res);
    }
}

#[derive(Debug, Clone)]
pub struct TrainerHandle<T: PeriTrait> {
    pub trainer: Arc<Mutex<Trainer<T>>>,
}

impl<T: PeriTrait> TrainerHandle<T> {
    pub async fn connect(&self) {
        let mut trainer = self.trainer.lock().await;
        let _ = trainer.peri.connect().await;
        let _ = trainer.peri.discover_services().await;
        for c in trainer.peri.characteristics() {
            if c.uuid == RESISTANCE_RANGE
                && let Ok(res) = trainer.peri.read(&c).await
            {
                trainer.resistance_range = Some(Range::from_bytes(res));
            }

            if c.uuid == POWER_RANGE
                && let Ok(res) = trainer.peri.read(&c).await
            {
                trainer.power_range = Some(Range::from_bytes(res));
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
            } else if c.uuid == BIKE_DATA {
                let res = trainer.peri.subscribe(&c).await;
                trainer.control = Some(c);
                eprintln!("Subscribed to bike data: {:?}", res);
            }
        }
    }

    pub async fn run(
        handle: Arc<Mutex<Trainer<T>>>,
        mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<Command>,
        data_tx: tokio::sync::broadcast::Sender<BikeData>,
    ) {
        let trainer = handle.lock().await;

        let get_notif = trainer.peri.notifications().await;
        if get_notif.is_err() {
            panic!("failed to get notifs")
        }

        let mut notify = get_notif
            .unwrap()
            .timeout_repeating(tokio::time::interval(Duration::from_secs(1)));
        eprintln!("ready for notifs");

        loop {
            if let Ok(Some(v)) = notify.try_next().await {
                // eprintln!("GOT = {:?}", v);
                #[allow(clippy::single_match)]
                match v.uuid {
                    BIKE_DATA => {
                        let _ = data_tx.send(BikeData::parse(v));
                    }
                    _ => (),
                };
            }

            if let Ok(c) = cmd_rx.try_recv() {
                match c {
                    Command::Reset => trainer.reset().await,
                    Command::Resist(level) => trainer.set_resistance(level).await,
                    Command::Power(level) => trainer.set_power(level).await,
                }
            }
        }
    }
}

pub async fn find(target: String) -> Option<TrainerHandle<Peripheral>> {
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await.ok()?;
    let central = adapters.into_iter().nth(0).unwrap();
    central.start_scan(ScanFilter::default()).await.ok()?;
    time::sleep(Duration::from_secs(2)).await;

    for p in central.peripherals().await.ok()? {
        if let Some(props) = p.properties().await.ok()? {
            // eprintln!("{:?}", props.local_name);
            if let Some(name) = props.local_name
                && name == target
            {
                let trainer = Arc::new(Mutex::new(Trainer::<Peripheral> {
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
