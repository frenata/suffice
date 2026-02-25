use std::io::Error;
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::ftms::{BikeData, FitnessDevice, Range};

pub enum Command {
    Reset,
    Resist(u16),
    Power(i16),
}

#[derive(Debug, Clone)]
pub struct Trainer<T: FitnessDevice> {
    device: T,
    resistance_range: Option<Range>,
    power_range: Option<Range>,
}

impl<T: FitnessDevice> Trainer<T> {
    pub async fn new(mut device: T) -> Trainer<T> {
        if let Ok((power_range, resistance_range)) = device.setup().await {
            Trainer {
                device: device,
                power_range: power_range,
                resistance_range: resistance_range,
            }
        } else {
            panic!()
        }
    }
    pub async fn run(
        &self,
        mut cmd_rx: tokio::sync::mpsc::UnboundedReceiver<Command>,
        data_tx: tokio::sync::broadcast::Sender<BikeData>,
    ) -> Result<(), Error> {
        let mut notify = self
            .device
            .notifications()
            .await
            .unwrap()
            .timeout_repeating(tokio::time::interval(Duration::from_secs(1)));

        loop {
            if let Ok(Some(data)) = notify.try_next().await {
                // eprintln!("GOT {:?}", data);
                let _ = data_tx.send(data);
            }

            if let Ok(c) = cmd_rx.try_recv() {
                match c {
                    Command::Reset => self.device.reset().await?,
                    Command::Resist(level) => {
                        if let Some(r) = self.resistance_range
                            && r.is_in(level)
                        {
                            self.device.set_resistance(level).await?
                        }
                    }
                    Command::Power(level) => {
                        if let Some(r) = self.power_range
                            && r.is_in(level.try_into().unwrap())
                        {
                            self.device.set_power(level).await?
                        }
                    }
                }
            }
        }
    }
}
