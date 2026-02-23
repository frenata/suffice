use std::error::Error;
use std::{env, time::Duration};

use suffice::trainer::{Command, TrainerHandle};
use tokio::{sync::mpsc, time::sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    // find the device we're interested in
    if let Some(trainer) = TrainerHandle::find(target.clone()).await {
        eprintln!("{:?}\n", trainer);
        trainer.connect().await;
        let (tx, rx) = mpsc::channel(32);
        let tx2 = tx.clone();
        let tx3 = tx.clone();

        let inner = trainer.trainer.clone();
        tokio::spawn(async move {
            TrainerHandle::run(inner, rx).await;
        });

        sleep(Duration::from_secs(5)).await;

        tokio::spawn(async move {
            tx.send(Command::Power(225)).await.unwrap();
        });

        sleep(Duration::from_secs(25)).await;

        tokio::spawn(async move {
            tx2.clone().send(Command::Power(100)).await.unwrap();
        });

        sleep(Duration::from_secs(25)).await;

        tokio::spawn(async move {
            tx3.clone().send(Command::Power(180)).await.unwrap();
        });

        sleep(Duration::from_secs(25)).await;

        // trainer.set_resistance(5).await;
        // trainer.set_resistance(20).await;
        // trainer.set_resistance(5).await;
    } else {
        // eprint!("{:?} not found!", target);
        return Err(format!("{} not found", target).into());
    }
    Ok(())
}
