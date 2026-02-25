use btleplug::api::ValueNotification;

use std::io::Error;
use std::pin::Pin;

use tokio_stream::Stream;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct BikeData {
    pub power: Option<i16>,
    pub cadence: Option<u8>,
    pub resistance: Option<i16>,
    pub heart_rate: Option<u8>,
    pub speed: Option<i16>,
}

impl BikeData {
    pub fn parse(v: ValueNotification) -> BikeData {
        let mut data = BikeData::default();
        // NOTE: via spec: 4.9.1.1
        // Important: this is simplified and not generically correct
        // the order of data is tied to the flags available
        // so if other flags (not checked for here) are present
        // the mapping of bytes in the payload to fields will be wrong
        if (v.value[0] & 0b00000001) == 0 {
            // NOTE: this flag does double duty as both 'more data' (if on)
            // and speed (if off)
            data.speed = Some(i16::from_le_bytes(v.value[2..4].try_into().unwrap()));
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
            data.resistance = Some(i16::from_le_bytes(v.value[6..8].try_into().unwrap()));
        }
        if v.value[0] & 0b01000000 != 0 {
            // power
            data.power = Some(i16::from_le_bytes(v.value[8..10].try_into().unwrap()));
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
        let actual = BikeData::parse(v);
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
        let actual = BikeData::parse(v);
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
        let actual = BikeData::parse(v);
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

pub trait FitnessDevice {
    fn setup(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(Option<Range>, Option<Range>), Error>> + Send;
    fn notifications(
        &self,
    ) -> impl std::future::Future<
        Output = Result<Pin<Box<dyn Stream<Item = BikeData> + Send>>, Error>,
    > + Send;
    fn reset(&self) -> impl std::future::Future<Output = Result<(), Error>> + Send;
    fn set_power(&self, level: i16) -> impl std::future::Future<Output = Result<(), Error>> + Send;
    fn set_resistance(
        &self,
        level: u16,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send;
}

#[derive(Debug, Clone, Copy)]
pub struct Range {
    #[allow(unused)]
    min: u16,
    #[allow(unused)]
    max: u16,
    #[allow(unused)]
    inc: u16,
}

impl Range {
    pub fn from_bytes(bytes: Vec<u8>) -> Range {
        Range {
            min: u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            max: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            inc: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        }
    }
    pub fn is_in(&self, n: u16) -> bool {
        n > self.min && n < self.max
    }
}
