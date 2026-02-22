use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, bleuuid::uuid_from_u16};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // find the device we're interested in
    if let Some(trainer) = find_trainer("Victory").await {
        eprint!("{:?}", trainer);
    }
    Ok(())
}

async fn find_trainer(target: &str) -> Option<Peripheral> {
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
            // eprint!("{:?}", props.local_name);
            if let Some(name) = props.local_name {
                return Some(p);
            }
        }
    }

    None
}
