use std::io::Error;
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{Level, event, instrument};

use crate::{
    ftms::{BikeData, FitnessDevice, Range},
    record,
};

/// Send `Commands` to interact with a running trainer's loop.
pub enum Command {
    Reset,
    Resist(u16),
    Power(i16),
    ToggleRecording,
    Quit,
}

#[derive(Debug, Clone)]
/// A Trainer is the core abstraction to connect and interact with a bike trainer.
pub struct Trainer<T: FitnessDevice> {
    device: T,
    resistance_range: Option<Range>,
    power_range: Option<Range>,
    data: Vec<BikeData>,
    is_recording: bool,
}

impl<T: FitnessDevice + std::fmt::Debug> Trainer<T> {
    /// Construct a new Trainer object that can be interacted with.
    pub async fn new(mut device: T) -> Trainer<T> {
        if let Ok((power_range, resistance_range)) = device.setup().await {
            Trainer {
                device,
                power_range,
                resistance_range,
                data: Vec::<BikeData>::new(),
                is_recording: false,
            }
        } else {
            panic!()
        }
    }

    #[instrument(skip(self, cmd_rx, data_tx))]
    /// Runs a forever loop -- designed to be executed in a spawned task
    /// All communication *to* the trainer should happen via the sending half of cmd_rx.
    /// All communication *from* the trainer should be received via the receiving half of data_tx.
    pub async fn run(
        &mut self,
        mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<Command>,
        data_tx: tokio::sync::broadcast::Sender<BikeData>,
    ) -> Result<(), Error> {
        let mut notify = self
            .device
            .notifications()
            .await
            .unwrap()
            .timeout_repeating(tokio::time::interval(Duration::from_millis(200)));

        loop {
            if let Ok(Some(mut data)) = notify.try_next().await {
                event!(Level::DEBUG, "received bike data {:?}", data);
                if !self.data.is_empty()
                    && let Some(speed) = data.speed
                {
                    let last = self.data[self.data.len() - 1];
                    let dt = (data.time - last.time).as_seconds_f32();
                    let dist = (dt * (speed / 360) as f32) as u32;
                    data.distance = Some(dist);
                }
                if self.is_recording {
                    self.data.push(data);
                } else {
                    // self.data.pop();
                    self.data.push(data);
                }
                let _ = data_tx.send(data);
            }

            if let Ok(c) = cmd_rx.try_recv() {
                let _ = match c {
                    Command::Reset => {
                        {
                            let _ = self.device.reset().await;
                        };
                        Ok::<(), Error>(())
                    }
                    Command::Resist(level) => {
                        if let Some(r) = self.resistance_range
                            && r.contains(level)
                        {
                            let _ = self.device.set_resistance(level).await;
                        };
                        Ok(())
                    }
                    Command::Power(level) => {
                        let _: () = if let Some(r) = self.power_range
                            && r.contains(level.try_into().unwrap())
                        {
                            let _ = self.device.set_power(level).await;
                        };
                        Ok(())
                    }
                    Command::ToggleRecording => {
                        {
                            let _: () = match self.is_recording {
                                true => {
                                    self.is_recording = false;
                                    event!(Level::INFO, "Finished recording");
                                    if let Ok(_res) = record::save_file(self.data.clone()) {
                                        event!(Level::INFO, "FIT file saved");
                                        self.data.clear();
                                    }
                                }
                                false => {
                                    self.is_recording = true;
                                    event!(Level::INFO, "Began recording");
                                }
                            };
                        }
                        Ok(())
                    }
                    Command::Quit => return Ok(()),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::ready;

    use super::*;
    use crate::ftms::MockFitnessDevice;
    use mockall::predicate;
    use std::pin::Pin;
    use tokio::sync::{broadcast, mpsc};
    use tokio_stream::Stream;

    #[tokio::test]
    async fn test_trainer_run() {
        let mut mock = MockFitnessDevice::new();
        mock.expect_setup()
            .returning(|| Box::pin(ready(Ok((None, Some(Range::new(1, 10)))))));

        mock.expect_set_resistance()
            .with(predicate::eq(4))
            .times(1)
            .returning(|_x| Box::pin(ready(Ok(()))));

        let mut trainer = Trainer::<MockFitnessDevice>::new(mock).await;

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
        let (data_tx, _data_rx) = broadcast::channel::<BikeData>(100);

        let handle = tokio::spawn(async move {
            let _ = trainer.run(cmd_rx, data_tx).await;
        });

        let res = cmd_tx.send(Command::Resist(4));
        assert!(res.is_ok());
        handle.abort();
    }

    #[tokio::test]
    async fn test_trainer_record() {
        let mut mock_device = MockFitnessDevice::new();
        mock_device.expect_notifications().returning(|| {
            let stream: Pin<Box<dyn Stream<Item = BikeData> + Send>> =
                Box::pin(tokio_stream::iter(vec![BikeData {
                    power: Some(100),
                    cadence: Some(88),
                    speed: Some(20),
                    resistance: Some(4),
                    heart_rate: Some(80),
                    ..BikeData::default()
                }]));

            Box::pin(ready(Ok(stream)))
        });

        let mut trainer = Trainer::<MockFitnessDevice> {
            device: mock_device,
            resistance_range: None,
            power_range: None,
            data: Vec::<BikeData>::new(),
            is_recording: false,
        };
        trainer.data.push(BikeData {
            power: Some(90),
            cadence: Some(78),
            speed: Some(20),
            resistance: Some(5),
            heart_rate: Some(81),
            ..BikeData::default()
        });

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
        let (data_tx, _data_rx) = broadcast::channel::<BikeData>(100);

        let _ = cmd_tx.send(Command::ToggleRecording);
        let _ = cmd_tx.send(Command::ToggleRecording);
        let _ = cmd_tx.send(Command::Quit);
        let _ = trainer.run(cmd_rx, data_tx).await;

        assert!(!trainer.is_recording);

        use rustyfit::{Decoder, profile::mesgdef};
        use std::{
            fs::{File, remove_file},
            io::BufReader,
        };

        let name = "output.fit";
        let f = File::open(name).unwrap();
        let br = BufReader::new(f);
        let mut dec = Decoder::new(br);

        let fit = dec.decode().unwrap().unwrap(); // First decode call is either Ok(Some(fit)) or Err(err), never Ok(None).
        let msg = &fit.messages[1];
        for field in msg.fields.clone() {
            if field.num == mesgdef::Record::CADENCE {
                assert_eq!(field.value.as_u8(), 78)
            }

            if field.num == mesgdef::Record::POWER {
                assert_eq!(field.value.as_u16(), 90)
            }

            if field.num == mesgdef::Record::SPEED {
                assert_eq!(field.value.as_u16(), 20)
            }

            if field.num == mesgdef::Record::HEART_RATE {
                assert_eq!(field.value.as_u8(), 81)
            }

            if field.num == mesgdef::Record::RESISTANCE {
                assert_eq!(field.value.as_u8(), 5)
            }
        }

        let msg = &fit.messages[2];
        for field in msg.fields.clone() {
            if field.num == mesgdef::Record::CADENCE {
                assert_eq!(field.value.as_u8(), 88)
            }
            if field.num == mesgdef::Record::HEART_RATE {
                assert_eq!(field.value.as_u8(), 80)
            }
            if field.num == mesgdef::Record::RESISTANCE {
                assert_eq!(field.value.as_u8(), 4)
            }
            if field.num == mesgdef::Record::DISTANCE {
                assert_eq!(field.value.as_u8(), 255)
            }
        }

        println!("{:?}", msg.fields);
        assert_eq!(fit.messages.len(), 5);
        let _ = remove_file("output.fit");
    }
}
