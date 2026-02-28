use crate::ftms::BikeData;
use chrono::Local;
use rustyfit::{
    Encoder,
    profile::{
        mesgdef::{self},
        typedef::{self, DateTime},
    },
    proto::{FIT, Message},
};
use std::io::Error;
use std::{
    // error::Error,
    fs::File,
    io::{BufWriter, Write},
};

pub fn save_file(data: Vec<BikeData>) -> Result<(), Error> {
    // NOTE: adapted from the example in the documentation
    // https://crates.io/crates/rustyfit#encode-using-mesgdef-module
    let fout_name = "output.fit";
    let fout = File::create(fout_name)?;
    let mut bw = BufWriter::new(fout);
    let mut enc = Encoder::new(&mut bw);
    let now = Local::now();

    let mut messages: Vec<Message> = data.iter().map(to_message).collect();
    messages.insert(0, get_header(start));

    let mut fit = FIT {
        messages,
        ..Default::default()
    };

    if let Err(e) = enc.encode(&mut fit) {
        return Err(Error::other(e.to_string()));
    }
    bw.flush()?;

    Ok(())
}

fn to_message(data: &BikeData) -> Message {
    let mut rec = mesgdef::Record::new();

    if let Some(cadence) = data.cadence {
        rec.cadence = cadence;
    }
    if let Some(power) = data.power {
        rec.power = power;
    }
    if let Some(speed) = data.speed {
        rec.speed = speed;
    }
    if let Some(heart_rate) = data.heart_rate {
        rec.heart_rate = heart_rate;
    }
    if let Some(resistance) = data.resistance {
        rec.resistance = resistance;
    }

    rec.timestamp = to_garmin_datetime(data.time);

    Message::from(rec)
}

    let mut file_id = mesgdef::FileId::new();
    file_id.manufacturer = typedef::Manufacturer::DEVELOPMENT;
    file_id.product_name = "Suffice".to_string();
    file_id.r#type = typedef::File::ACTIVITY;
    file_id.time_created = to_garmin_datetime(time);
    Message::from(file_id)
}

fn to_garmin_datetime(dt: chrono::DateTime<Local>) -> rustyfit::profile::typedef::DateTime {
    // let anchor = chrono::DateTime<Local>::timez
    let garmin_anchor = Local::now()
        .timezone()
        .with_ymd_and_hms(1989, 12, 31, 0, 0, 0)
        .unwrap()
        .timestamp();

    let local = dt.timestamp();

    rustyfit::profile::typedef::DateTime((local - garmin_anchor) as u32)
}

