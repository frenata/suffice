use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, bleuuid::uuid_from_u16};
use btleplug::platform::{Manager, Peripheral};
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

#[derive(Debug)]
pub struct Trainer {
    peri: Peripheral,
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
                    return Some(Trainer { peri: p });
                }
            }
        }

        None
    }

    pub async fn connect(self: &Trainer) {
        self.peri.connect().await;
        self.peri.discover_services().await;
        for c in self.peri.characteristics() {
            eprintln!("{:?}", c)
        }
    }
}
