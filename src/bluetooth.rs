use btleplug::api::ValueNotification;
use uuid::{Uuid, uuid};

use std::io::Error;
use std::pin::Pin;

use btleplug::api::{Central, Manager as _, Peripheral as PeriTrait, ScanFilter};
use btleplug::api::{Characteristic, WriteType};
use btleplug::platform::{Manager, Peripheral};
use std::time::Duration;
use tokio::time;
use tokio_stream::{Stream, StreamExt};

use crate::ftms::{BikeData, FitnessData, FitnessDevice, Range};

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
            return Err(Error::other(e.to_string()));
        }
        if let Err(e) = self.peripheral.discover_services().await {
            return Err(Error::other(e.to_string()));
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
            if c.uuid == MACHINE_STATUS
                && let Err(e) = self.peripheral.subscribe(&c).await
            {
                return Err(Error::other(e.to_string()));
            }

            if c.uuid == TRAINING_STATUS
                && let Err(e) = self.peripheral.subscribe(&c).await
            {
                return Err(Error::other(e.to_string()));
            }

            if c.uuid == MACHINE_CONTROL {
                if let Err(e) = self.peripheral.subscribe(&c).await {
                    return Err(Error::other(e.to_string()));
                }
                self.control = Some(c);
            } else if c.uuid == BIKE_DATA
                && let Err(e) = self.peripheral.subscribe(&c).await
            {
                return Err(Error::other(e.to_string()));
            }
        }
        Ok((power_range, resistance_range))
    }

    async fn notifications(&self) -> Result<Pin<Box<dyn Stream<Item = BikeData> + Send>>, Error> {
        if let Ok(get_notif) = self.peripheral.notifications().await {
            let data = get_notif
                .filter(|notify: &ValueNotification| matches!(notify.uuid, BIKE_DATA))
                .map(|notify: ValueNotification| ValueNotification::parse(notify));
            Ok(Box::pin(data))
        } else {
            Err(Error::other("failed to get notifications".to_string()))
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
            return Err(Error::other(err.to_string()));
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
            return Err(Error::other(err.to_string()));
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
            return Err(Error::other(err.to_string()));
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
            Err(Error::other(err.to_string()))
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
            Err(Error::other(err.to_string()))
        } else {
            Ok(())
        }
    }
}

impl FitnessData for ValueNotification {
    fn parse(v: ValueNotification) -> BikeData {
        let mut data = BikeData::default();
        // NOTE: via spec: 4.9.1.1
        // Important: this is simplified and not generically correct
        // the order of data is tied to the flags available
        // so if other flags (not checked for here) are present
        // the mapping of bytes in the payload to fields will be wrong
        if (v.value[0] & 0b00000001) == 0 {
            // NOTE: this flag does double duty as both 'more data' (if on)
            // and speed (if off)
            // TODO: the BLE spec says this is i16, but we need to convert to u16 -- what's the best way?
            data.speed = Some(u16::from_le_bytes(v.value[2..4].try_into().unwrap()));
        }
        if v.value[0] & 0b00000100 != 0 {
            // cadence
            data.cadence = Some(
                (u16::from_le_bytes(v.value[4..6].try_into().unwrap()) / 2)
                    .try_into()
                    .unwrap(),
            );
        }
        if v.value[0] & 0b00100000 != 0 {
            // resistance
            // TODO: the BLE spec says this is i16, but we need to convert to u8 -- what's the best way?
            data.resistance = Some(
                u16::from_le_bytes(v.value[6..8].try_into().unwrap())
                    .try_into()
                    .unwrap(),
            );
        }
        if v.value[0] & 0b01000000 != 0 {
            // power
            // TODO: the BLE spec says this is i16, but we need to convert to u16 -- what's the best way?
            data.power = Some(u16::from_le_bytes(v.value[8..10].try_into().unwrap()));
        }
        if v.value[1] & 0b00000010 != 0 {
            // TODO heartrate
            data.heart_rate = Some(u8::from_le_bytes(v.value[10..11].try_into().unwrap()));
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn all_zero_values() {
        let v: ValueNotification = ValueNotification {
            uuid: Uuid::new_v4(),
            value: vec![100, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let actual = ValueNotification::parse(v);
        let expected = BikeData {
            power: Some(0),
            cadence: Some(0),
            resistance: Some(0),
            heart_rate: Some(0),
            speed: Some(0),
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn no_flag_bits() {
        let v: ValueNotification = ValueNotification {
            uuid: Uuid::new_v4(),
            value: vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let actual = ValueNotification::parse(v);
        let expected = BikeData {
            power: None,
            cadence: None,
            resistance: None,
            heart_rate: None,
            speed: None,
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn arbitrary_data() {
        let v: ValueNotification = ValueNotification {
            uuid: Uuid::new_v4(),
            value: vec![100, 2, 34, 2, 17, 0, 77, 0, 9, 9, 55],
        };
        let actual = ValueNotification::parse(v);
        let expected = BikeData {
            power: Some(2313),
            cadence: Some(8),
            resistance: Some(77),
            heart_rate: Some(55),
            speed: Some(546),
        };

        assert_eq!(actual, expected);
    }
}
