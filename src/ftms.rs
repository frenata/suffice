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
/// A bundle of data collected or derviced from the trainer
pub struct BikeData {
    /// Power is measured in watts.
    pub power: Option<u16>,
    /// Cadence is measured in revolutions-per-minute (rpm).
    pub cadence: Option<u8>,
    /// Resistance measures how hard the trainer is working against you.
    pub resistance: Option<u8>,
    /// Heart Rate is measured in beats-per-minute (bpm).
    pub heart_rate: Option<u8>,
    /// Speed is meausred in millimeters per second.
    pub speed: Option<u16>,
    /// Distance is measured in centimeters.
    pub distance: Option<u32>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Default(value = "Local::now()"))]
    /// Time is a timezone aware moment in time.
    pub time: DateTime<Local>,
}

pub(crate) trait FitnessData {
    fn parse(s: Self) -> BikeData;
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Range represents the possible scope of values
pub struct Range {
    min: u16,
    max: u16,
    #[allow(unused)]
    inc: u16,
}

impl Range {
    /// Constructs a range given a minimum and maximum value.
    pub fn new(min: u16, max: u16) -> Range {
        Range { min, max, inc: 1 }
    }
    /// Constructs a range from little-endian bytes.
    ///
    /// ```
    /// use suffice::ftms::Range;
    /// let r1 = Range::from_bytes(vec![1, 0, 10, 0, 1, 0]);
    /// let r2 = Range::new(1, 10);
    /// assert_eq!(r1, r2);
    /// ```
    pub fn from_bytes(bytes: Vec<u8>) -> Range {
        Range {
            min: u16::from_le_bytes(bytes[0..2].try_into().unwrap()),
            max: u16::from_le_bytes(bytes[2..4].try_into().unwrap()),
            inc: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        }
    }
    /// Returns true if n is valid within the range.
    pub fn contains(&self, n: u16) -> bool {
        // FIXME: check inc
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
/// A FitnessDevice represents the underlying connection to the hardware.
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
