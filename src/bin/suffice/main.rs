use std::env;
use std::error::Error;

use tokio::sync::{broadcast, mpsc};

use suffice::bluetooth::BluetoothDevice;
use suffice::ftms::BikeData;
use suffice::trainer::{Command, Trainer};

mod app;
mod stats;
use app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    let device = BluetoothDevice::new(target.clone()).await;
    if device.is_none() {
        return Err(format!("{} not found", target).into());
    }
    let device = device.unwrap();
    let mut trainer = Trainer::<BluetoothDevice>::new(device).await;

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);

    tokio::spawn(async move {
        let _ = trainer.run(cmd_rx, data_tx).await;
    });

    let mut term = ratatui::init();
    let mut app = App::default();
    let res = app
        .run(&mut term, cmd_tx.clone(), data_rx)
        .await
    // .inspect_err(|e| tracing::error!("Error in main event loop: {}", e))
    ;
    ratatui::restore();
    Ok(res?)
}
