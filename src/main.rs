use btleplug::api::Peripheral as _;
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

use suffice::trainer::TrainerHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    // find the device we're interested in
    if let Some(trainer) = TrainerHandle::find(target.clone()).await {
        eprintln!("{:?}\n", trainer);
        trainer.connect().await;

        let inner = trainer.trainer.clone();
        tokio::spawn(async move {
            TrainerHandle::notifications(inner).await;
        });

        trainer.set_resistance(5).await;
        trainer.set_resistance(20).await;
        trainer.set_resistance(5).await;
    } else {
        // eprint!("{:?} not found!", target);
        return Err(format!("{} not found", target).into());
    }
    Ok(())
}
