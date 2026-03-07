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
use tracing::{Level, event, instrument};

use crate::ftms::{BikeData, FitnessData, FitnessDevice, Range};

const RESISTANCE_RANGE: Uuid = uuid!("00002ad6-0000-1000-8000-00805f9b34fb");
const POWER_RANGE: Uuid = uuid!("00002ad8-0000-1000-8000-00805f9b34fb");

const MACHINE_STATUS: Uuid = uuid!("00002ada-0000-1000-8000-00805f9b34fb");
const TRAINING_STATUS: Uuid = uuid!("00002ad3-0000-1000-8000-00805f9b34fb");

const MACHINE_CONTROL: Uuid = uuid!("00002ad9-0000-1000-8000-00805f9b34fb");
const BIKE_DATA: Uuid = uuid!("00002ad2-0000-1000-8000-00805f9b34fb");

#[derive(Debug)]
/// Implements a FitnessDevice over BT-LE
pub struct BluetoothDevice {
    peripheral: Peripheral,
    control: Option<Characteristic>,
}

impl std::fmt::Display for BluetoothDevice {
    fn fmt(&self, format: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if let Err(e) = write!(format, "{}", self.peripheral.id()) {
            Err(e)
        } else {
            Ok(())
        }
    }
}

impl BluetoothDevice {
    #[instrument]
    /// Creates a new connection to a BluetoothDevice via device descriptor.
    pub async fn new(target: String) -> Option<BluetoothDevice> {
        let manager = Manager::new().await.unwrap();
        let adapters = manager.adapters().await.ok()?;
        let central = adapters.into_iter().nth(0).unwrap();
        event!(Level::INFO, "Scanning for Devices...");
        central.start_scan(ScanFilter::default()).await.ok()?;
        time::sleep(Duration::from_secs(1)).await;

        for p in central.peripherals().await.ok()? {
            if let Some(props) = p.properties().await.ok()? {
                event!(Level::DEBUG, "Found {:?}", props.local_name);
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
    #[instrument]
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
        event!(Level::INFO, "Device config complete");
        Ok((power_range, resistance_range))
    }

    #[instrument]
    async fn notifications(&self) -> Result<Pin<Box<dyn Stream<Item = BikeData> + Send>>, Error> {
        if let Ok(get_notif) = self.peripheral.notifications().await {
            let data = get_notif
                .map(|notify| {
                    event!(Level::DEBUG, "BLE notification: {:?}", notify);
                    notify
                })
                .filter(|notify: &ValueNotification| matches!(notify.uuid, BIKE_DATA))
                .map(|notify: ValueNotification| ValueNotification::parse(notify));
            Ok(Box::pin(data))
        } else {
            Err(Error::other("failed to get notifications".to_string()))
        }
    }

    #[instrument(skip(self))]
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
            event!(
                Level::ERROR,
                "failed to request control before reset {}",
                err.to_string()
            );
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
            event!(Level::ERROR, "failed to reset {}", err.to_string());
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
            event!(
                Level::ERROR,
                "failed to request control after reset {}",
                err.to_string()
            );
            return Err(Error::other(err.to_string()));
        }
        Ok(())
    }

    #[instrument(skip(self))]
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
            event!(Level::ERROR, "{}", err.to_string());
            Err(Error::other(err.to_string()))
        } else {
            event!(Level::INFO, "");
            Ok(())
        }
    }

    #[instrument(skip(self))]
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
            event!(Level::ERROR, "{}", err.to_string());
            Err(Error::other(err.to_string()))
        } else {
            event!(Level::INFO, "");
            Ok(())
        }
    }
}

/// Implemented as per FTMS 4.9.1
/// The best description of the individual fields and their values appears to be
/// this doc from Huawei: https://developer.huawei.com/consumer/en/doc/hmscore-guides/ibd-0000001051005923
#[derive(Default, PartialEq, Debug)]
struct BluetoothIndoorBikeData {
    instant_speed: Option<u16>,
    average_speed: Option<u16>,
    instant_cadence: Option<u16>,
    average_cadence: Option<u16>,
    total_distance: Option<u32>,
    resistance_level: Option<i16>,
    instant_power: Option<i16>,
    average_power: Option<i16>,
    total_energy: Option<u16>,
    energy_per_hour: Option<u8>,
    energy_per_minute: Option<u8>,
    heart_rate: Option<u8>,
    metabolic_equivalent: Option<u8>,
    elapsed_time: Option<u16>,
    remaining_time: Option<u16>,
}

impl BluetoothIndoorBikeData {
    fn parse(v: ValueNotification) -> BluetoothIndoorBikeData {
        let mut data = BluetoothIndoorBikeData {
            ..Default::default()
        };
        let mut pos = 2; // because we implicitly read the first two bytes as the flags.

        if (v.value[0] & 0b00000001) == 0 {
            // NOTE: this flag does double duty as both 'more data' (if on)
            // and speed (if off)
            data.instant_speed = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[0] & 0b00000010) != 0 {
            data.average_speed = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[0] & 0b00000100) != 0 {
            data.instant_cadence = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[0] & 0b00001000) != 0 {
            data.average_cadence = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[0] & 0b00010000) != 0 {
            data.total_distance = Some(u32::from_le_bytes([
                v.value[pos],
                v.value[pos + 1],
                v.value[pos + 2],
                0,
            ]));
            pos += 3;
        }
        if (v.value[0] & 0b00100000) != 0 {
            data.resistance_level = Some(i16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[0] & 0b01000000) != 0 {
            data.instant_power = Some(i16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[0] & 0b10000000) != 0 {
            data.average_power = Some(i16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[1] & 0b00000001) != 0 {
            data.total_energy = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            data.energy_per_hour = Some(v.value[pos + 2]);
            data.energy_per_minute = Some(v.value[pos + 3]);
            pos += 4;
        }
        if (v.value[1] & 0b00000010) != 0 {
            data.heart_rate = Some(u8::from_le_bytes([v.value[pos]]));
            pos += 1;
        }
        if (v.value[1] & 0b00000100) != 0 {
            data.metabolic_equivalent = Some(u8::from_le_bytes([v.value[pos]]));
            pos += 1;
        }
        if (v.value[1] & 0b00001000) != 0 {
            data.elapsed_time = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
            pos += 2;
        }
        if (v.value[1] & 0b00010000) != 0 {
            data.remaining_time = Some(u16::from_le_bytes([v.value[pos], v.value[pos + 1]]));
        }

        data
    }
}

impl FitnessData for ValueNotification {
    fn parse(v: ValueNotification) -> BikeData {
        let indoor = BluetoothIndoorBikeData::parse(v);
        BikeData {
            power: indoor.instant_power.map(|p| p as u16),
            cadence: indoor.instant_cadence.map(|c| (c / 2) as u8),
            resistance: indoor.resistance_level.map(|r| r as u8),
            heart_rate: indoor.heart_rate,
            speed: indoor.instant_speed,
            ..Default::default()
        }
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
            ..Default::default()
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
        let expected = BikeData::default();

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
            ..Default::default()
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn all_data_available() {
        let v: ValueNotification = ValueNotification {
            uuid: Uuid::new_v4(),
            // The first *13* bits indicate the remaining data, so the first byte should be at most
            // 255 and the second at most 31.
            // *However* the 0th flag bit designates 'more data' when off, so practically
            // a full set of data should be 254, 31.
            value: vec![
                254, 31, // flag bytes
                // data for first flag byte
                1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 0, 6, 0, 7, 0, 8, 0,
                // data for second flag byte
                9, 0, 10, 11, 12, 13, 14, 0, 15, 0,
            ],
        };
        let actual = BluetoothIndoorBikeData::parse(v);
        let expected = BluetoothIndoorBikeData {
            instant_speed: Some(1),
            average_speed: Some(2),
            instant_cadence: Some(3),
            average_cadence: Some(4),
            total_distance: Some(5),
            resistance_level: Some(6),
            instant_power: Some(7),
            average_power: Some(8),
            total_energy: Some(9),
            energy_per_hour: Some(10),
            energy_per_minute: Some(11),
            heart_rate: Some(12),
            metabolic_equivalent: Some(13),
            elapsed_time: Some(14),
            remaining_time: Some(15),
        };

        assert_eq!(actual, expected);
    }
}
