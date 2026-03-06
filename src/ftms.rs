use std::io::Error;
use std::pin::Pin;

use async_stream::stream;
use chrono::{DateTime, Local};
use derivative::Derivative;
use rand;
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
pub trait FitnessDevice: std::fmt::Debug {
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

#[derive(Debug, Default)]
pub struct SampleDevice {}

impl FitnessDevice for SampleDevice {
    fn setup(
        &mut self,
    ) -> impl std::future::Future<
        Output = Result<(Option<crate::ftms::Range>, Option<crate::ftms::Range>), Error>,
    > + Send {
        std::future::ready(Ok((Some(Range::new(1, 1000)), Some(Range::new(1, 100)))))
    }

    fn notifications(
        &self,
    ) -> impl std::future::Future<
        Output = Result<Pin<Box<dyn Stream<Item = BikeData> + Send>>, Error>,
    > + Send {
        let stream = stream! {
        loop {
            let data = BikeData{
                power: Some(rand::random_range(80..350)),
                cadence: Some(rand::random_range(50..110)),
                heart_rate: Some(rand::random_range(80..180)),
                ..Default::default()
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            yield data;
        }};

        let boxx = Box::new(stream);
        let res: Result<Pin<Box<dyn Stream<Item = BikeData> + Send>>, Error> =
            Ok(Box::into_pin(boxx));

        std::future::ready(res)
    }

    fn reset(&self) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        std::future::ready(Ok(()))
    }

    fn set_power(
        &self,
        _level: i16,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        std::future::ready(Ok(()))
    }

    fn set_resistance(
        &self,
        _level: u16,
    ) -> impl std::future::Future<Output = Result<(), Error>> + Send {
        std::future::ready(Ok(()))
    }
}

#[tokio::test]
async fn test_sample_never_fails() {
    let mut sample = SampleDevice::default();
    assert!(sample.set_power(999).await.is_ok());
    assert!(sample.set_resistance(99).await.is_ok());
    assert!(sample.reset().await.is_ok());
    assert!(sample.setup().await.is_ok());
    assert!(sample.notifications().await.is_ok());
}
