use btleplug::api::ValueNotification;
use uuid::{Uuid, uuid};

use std::io::{Error, ErrorKind};
use std::pin::Pin;

use btleplug::api::{Central, Manager as _, Peripheral as PeriTrait, ScanFilter};
use btleplug::api::{Characteristic, WriteType};
use btleplug::platform::{Manager, Peripheral};
use std::time::Duration;
use tokio::time;
use tokio_stream::{Stream, StreamExt};

use crate::ftms::{BikeData, FitnessDevice, Range};

pub const RESISTANCE_RANGE: Uuid = uuid!("00002ad6-0000-1000-8000-00805f9b34fb");
pub const POWER_RANGE: Uuid = uuid!("00002ad8-0000-1000-8000-00805f9b34fb");
// pub const FEATURES: Uuid = uuid!("00002acc-0000-1000-8000-00805f9b34fb");

pub const MACHINE_STATUS: Uuid = uuid!("00002ada-0000-1000-8000-00805f9b34fb");
pub const TRAINING_STATUS: Uuid = uuid!("00002ad3-0000-1000-8000-00805f9b34fb");

pub const MACHINE_CONTROL: Uuid = uuid!("00002ad9-0000-1000-8000-00805f9b34fb");
pub const BIKE_DATA: Uuid = uuid!("00002ad2-0000-1000-8000-00805f9b34fb");

pub struct BluetoothDevice {
    peripheral: Peripheral,
    control: Option<Characteristic>,
}

impl BluetoothDevice {
    pub async fn new(target: String) -> Option<BluetoothDevice> {
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
                    return Some(BluetoothDevice {
                        peripheral: p,
                        control: None,
                    });
                }
            }
        }
        None
    }
}

impl FitnessDevice for BluetoothDevice {
    async fn setup(&mut self) -> Result<(Option<Range>, Option<Range>), Error> {
        let mut power_range = None;
        let mut resistance_range = None;

        if let Err(e) = self.peripheral.connect().await {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        if let Err(e) = self.peripheral.discover_services().await {
            return Err(Error::new(ErrorKind::Other, e.to_string()));
        }
        for c in self.peripheral.characteristics() {
            if c.uuid == RESISTANCE_RANGE
                && let Ok(res) = self.peripheral.read(&c).await
            {
                resistance_range = Some(Range::from_bytes(res));
            }

            if c.uuid == POWER_RANGE
                && let Ok(res) = self.peripheral.read(&c).await
            {
                power_range = Some(Range::from_bytes(res));
            }
        }

        for c in self.peripheral.characteristics() {
            if c.uuid == MACHINE_STATUS {
                if let Err(e) = self.peripheral.subscribe(&c).await {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
            }

            if c.uuid == TRAINING_STATUS {
                if let Err(e) = self.peripheral.subscribe(&c).await {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
            }

            if c.uuid == MACHINE_CONTROL {
                if let Err(e) = self.peripheral.subscribe(&c).await {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
                self.control = Some(c);
            } else if c.uuid == BIKE_DATA {
                if let Err(e) = self.peripheral.subscribe(&c).await {
                    return Err(Error::new(ErrorKind::Other, e.to_string()));
                }
            }
        }
        Ok((power_range, resistance_range))
    }

    async fn notifications(&self) -> Result<Pin<Box<dyn Stream<Item = BikeData> + Send>>, Error> {
        if let Ok(get_notif) = self.peripheral.notifications().await {
            let data = get_notif
                .filter(|notify: &ValueNotification| match notify.uuid {
                    BIKE_DATA => true,
                    _ => false,
                })
                .map(|notify: ValueNotification| BikeData::parse(notify));
            Ok(Box::pin(data))
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "failed to get notifications".to_string(),
            ));
        }
    }

    async fn reset(&self) -> Result<(), Error> {
        // 1. Request Control
        // 2. Reset params (which gives up control!)
        // 3. Request Control again!

        if let Err(err) = self
            .peripheral
            .write(
                self.control.as_ref().unwrap(),
                &[0],
                WriteType::WithResponse,
            )
            .await
        {
            return Err(Error::new(ErrorKind::Other, err.to_string()));
        }

        if let Err(err) = self
            .peripheral
            .write(
                self.control.as_ref().unwrap(),
                &[1],
                WriteType::WithResponse,
            )
            .await
        {
            return Err(Error::new(ErrorKind::Other, err.to_string()));
        }

        if let Err(err) = self
            .peripheral
            .write(
                self.control.as_ref().unwrap(),
                &[0],
                WriteType::WithResponse,
            )
            .await
        {
            return Err(Error::new(ErrorKind::Other, err.to_string()));
        }
        Ok(())
    }

    async fn set_resistance(&self, level: u16) -> Result<(), Error> {
        let mut data = u16::to_le_bytes(level).to_vec();
        data.insert(0, 4);
        if let Err(err) = self
            .peripheral
            .write(
                self.control.as_ref().unwrap(),
                &data,
                WriteType::WithResponse,
            )
            .await
        {
            Err(Error::new(ErrorKind::Other, err.to_string()))
        } else {
            Ok(())
        }
    }

    async fn set_power(&self, level: i16) -> Result<(), Error> {
        let mut data = i16::to_le_bytes(level).to_vec();
        data.insert(0, 5);
        if let Err(err) = self
            .peripheral
            .write(
                self.control.as_ref().unwrap(),
                &data,
                WriteType::WithResponse,
            )
            .await
        {
            Err(Error::new(ErrorKind::Other, err.to_string()))
        } else {
            Ok(())
        }
    }
}
