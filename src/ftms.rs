use std::io::Error;
use std::pin::Pin;

use chrono::{DateTime, Local};
use derivative::Derivative;
use tokio_stream::Stream;

#[cfg(test)]
use mockall::{automock, predicate::*};

#[derive(Derivative, Debug, Clone, Copy)]
#[derivative(PartialEq)]
#[derivative(Default)]
pub struct BikeData {
    pub power: Option<u16>,     // watts
    pub cadence: Option<u8>,    // rpm
    pub resistance: Option<u8>, // unitless
    pub heart_rate: Option<u8>, // bpm
    pub speed: Option<u16>,     // mm/s
    pub distance: Option<u32>,  // cm
    #[derivative(PartialEq = "ignore")]
    #[derivative(Default(value = "Local::now()"))]
    pub time: DateTime<Local>,
}

pub trait FitnessData {
    fn parse(s: Self) -> BikeData;
}

#[derive(Debug, Clone, Copy)]
pub struct Range {
    min: u16,
    max: u16,
    #[allow(unused)]
    inc: u16,
}

impl Range {
    pub fn new(min: u16, max: u16) -> Range {
        Range { min, max, inc: 1 }
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Range {
        Range {
            min: u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            max: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            inc: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        }
    }
    pub fn contains(&self, n: u16) -> bool {
        // TODO check inc
        n >= self.min && n <= self.max
    }
}

#[test]
fn n_in_range() {
    let bytes = vec![1, 0, 10, 0, 1, 0];
    let r = Range::from_bytes(bytes);

    assert!(r.contains(4));
    assert!(r.contains(1));
    assert!(r.contains(9));
    assert!(r.contains(10));
    assert!(!r.contains(11));
    assert!(!r.contains(0));
}

#[cfg_attr(test, automock)]
pub trait FitnessDevice {
    fn setup(
        &mut self,
    ) -> impl std::future::Future<
        Output = Result<(Option<crate::ftms::Range>, Option<crate::ftms::Range>), Error>,
    > + Send;
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
