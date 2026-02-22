use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, bleuuid::uuid_from_u16};
use btleplug::platform::{Manager, Peripheral};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

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
}

impl Trainer {
    pub async fn find(target: String) -> Option<Trainer> {
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
                    return Some(Trainer {
                        peri: p,
                        resistance_range: None,
                        power_range: None,
                    });
                }
            }
        }

        None
    }

    pub async fn connect(self: &mut Trainer) {
        self.peri.connect().await;
        self.peri.discover_services().await;
        for c in self.peri.characteristics() {
            if c.uuid == RESISTANCE_RANGE {
                if let Ok(res) = self.peri.read(&c).await {
                    self.resistance_range = Some(Range::from_bytes(res));
                }
            }

            if c.uuid == POWER_RANGE {
                if let Ok(res) = self.peri.read(&c).await {
                    self.power_range = Some(Range::from_bytes(res));
                }
            }

            // TODO: See bitmasks here https://github.com/zacharyedwardbull/pycycling/blob/master/pycycling/ftms_parsers/fitness_machine_feature.py#L132
            // if c.uuid == FEATURES {
            //     eprintln!("Features: {:?}", c);
            //     let res = self.peri.read(&c).await;
            //     eprintln!("{:?}\n", res);
            // }
        }
        eprintln!("Resistance: {:?}", self.resistance_range);
        eprintln!("Power: {:?}", self.power_range);
        }
    }
}
