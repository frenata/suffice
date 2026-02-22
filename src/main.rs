use btleplug::api::Peripheral as _;
use std::env;
use std::error::Error;

use suffice::trainer::Trainer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let target = args[1].clone();
    // find the device we're interested in
    if let Some(trainer) = Trainer::find(target.clone()).await {
        eprint!("{:?}", trainer);
        trainer.connect().await;
    } else {
        // eprint!("{:?} not found!", target);
        return Err(format!("{} not found", target).into());
    }
    Ok(())
}
