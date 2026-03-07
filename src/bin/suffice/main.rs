use std::error::Error;

use tokio::sync::{broadcast, mpsc};

use suffice::bluetooth::BluetoothDevice;
use suffice::config::Config;
use suffice::ftms::{BikeData, FitnessDevice, SampleDevice};
use suffice::trainer::{Command, Trainer};

mod app;
mod state;
mod stats;
mod widgets;
use app::App;

use clap::{Args, Parser, Subcommand};
use tracing::Level;

#[derive(Debug, Parser)]
#[command(name = "suffice", version)]
#[command(about = "CLI tool for controlling a cycling trainer", long_about = None)]
struct Cli {
    #[clap(flatten)]
    opts: Opts,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Test whether a connection can be made.
    Test {
        /// The bluetooth device to connect to.
        target: String,
    },
    /// Connect to a fake device that generates sample data
    Sample {},
    /// Connect a train on this device.
    Connect {
        /// The bluetooth device to connect to.
        target: String,
    },
}

#[derive(Debug, Args)]
struct Opts {
    /// Verbose logging messages
    #[clap(long, short, global = true, default_value_t = false)]
    verbose: bool,

    /// Only show error messages
    #[clap(long, short, global = true, default_value_t = false)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let level = if args.opts.verbose {
        Level::DEBUG
    } else if args.opts.quiet {
        Level::ERROR
    } else {
        Level::INFO
    };

    let config = Config::default().get_dir();
    println!("logs and FIT files will be written to {:?}", config.clone());
    let file_appender = tracing_appender::rolling::daily(config, "suffice.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(level)
        .with_writer(non_blocking)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    match args.command {
        Commands::Test { target } => {
            let device = BluetoothDevice::new(target.clone()).await;
            if device.is_none() {
                return Err(format!("{} not found", target).into());
            } else {
                println!("{:?} is available for connections", target);
                Ok(())
            }
        }
        Commands::Connect { target } => {
            let device = BluetoothDevice::new(target.clone()).await;
            if device.is_none() {
                return Err(format!("{} not found", target).into());
            }
            return run(device.unwrap()).await;
        }
        Commands::Sample {} => {
            let device = SampleDevice::default();
            return run(device).await;
        }
    }
}

async fn run<T: FitnessDevice + std::marker::Send + 'static>(
    device: T,
) -> Result<(), Box<dyn Error>> {
    let mut trainer = Trainer::<T>::new(device).await;

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (data_tx, data_rx) = broadcast::channel::<BikeData>(100);

    tokio::spawn(async move {
        let _ = trainer.run(cmd_rx, data_tx).await;
    });

    let mut term = ratatui::init();
    let mut app = App::default();
    let res = app.run(&mut term, cmd_tx.clone(), data_rx).await;
    ratatui::restore();
    Ok(res?)
}
