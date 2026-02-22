use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, bleuuid::uuid_from_u16};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    // find the device we're interested in
    if let Some(trainer) = find_trainer(target.clone()).await {
        eprint!("{:?}", trainer);
        trainer.connect().await?;
        trainer.discover_services().await?;
        for c in trainer.characteristics() {
            eprintln!("{:?}", c)
        }
    } else {
        // eprint!("{:?} not found!", target);
        return Err(format!("{} not found", target).into());
    }
    Ok(())
}

async fn find_trainer(target: String) -> Option<Peripheral> {
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
                return Some(p);
            }
        }
    }

    None
}
