use crate::config::Config;
use crate::ftms::BikeData;

#[allow(unused_imports)]
use chrono::TimeZone as _;
use chrono::{DateTime, Local};
use rustyfit::{
    Encoder,
    profile::{
        mesgdef::{self},
        typedef::{self, Sport, SubSport},
    },
    proto::{FIT, Message},
};
use std::io::Error;
use std::{
    fs::File,
    io::{BufWriter, Write},
};
use tracing::{Level, event, instrument};

#[instrument(skip(data))]
/// Saves a collection of BikeData to a FIT file on disk.
pub(crate) fn save_file(data: &mut [BikeData]) -> Result<(), Error> {
    let fout = File::create(Config::default().get_recording_name())?;
    let mut bw = BufWriter::new(fout);
    let mut enc = Encoder::new(&mut bw);

    if let Err(e) = enc.encode(&mut to_fit(data)) {
        event!(Level::ERROR, "failed to encode FIT {:?}", e);
        return Err(Error::other(e.to_string()));
    }
    bw.flush()?;

    Ok(())
}

fn to_fit(data: &mut [BikeData]) -> FIT {
    let start = data[0].time;
    let end = data[data.len() - 1].time;

    // NOTE: required messages and fields via
    // https://developer.garmin.com/fit/file-types/activity/
    let total_dist: u32 = data
        .iter()
        .map(|bd| bd.distance.unwrap_or_default() * 100)
        .sum();
    let mut messages: Vec<Message> = data.iter().map(to_message).collect();
    messages.insert(0, Message::from(get_file_id(start)));
    messages.push(Message::from(get_activity(start)));
    messages.push(Message::from(get_session(start, end, total_dist)));
    // TODO: Practically speaking this seems sufficient for Strava, intervals.icu, etc.
    // but isn't "best practice" per Garmin.
    // Should add extra messages: Lap, TimerStarted / Stopped, etc.

    FIT {
        messages,
        ..Default::default()
    }
}

#[test]
fn test_to_fit() {
    use fixed_deque::Deque;

    let mut data = Deque::<BikeData>::new(10);
    data.push_back(BikeData::default());

    let buf = std::io::Cursor::new(Vec::<u8>::new());
    let mut bw = BufWriter::new(buf);
    let mut enc = Encoder::new(&mut bw);
    let mut actual = to_fit(data.make_contiguous());
    if let Ok(_res) = enc.encode(&mut actual) {}

    assert_eq!(actual.file_header.data_size, 124);
}

fn to_message(data: &BikeData) -> Message {
    // TODO: consider using developer data fields to add extra info
    let mut rec = mesgdef::Record::new();

    if let Some(cadence) = data.cadence {
        rec.cadence = cadence;
    }
    if let Some(power) = data.power {
        rec.power = power;
    }
    if let Some(speed) = data.speed {
        // Translate from 10m/hour (our internal unit) => mm/second (the FIT unit)
        rec.speed = (speed as f32 * 2.777) as u16;
    }
    if let Some(heart_rate) = data.heart_rate {
        rec.heart_rate = heart_rate;
    }
    if let Some(resistance) = data.resistance {
        rec.resistance = resistance;
    }
    if let Some(distance) = data.distance {
        // Translate from meters (our internal unit) => centimeters (the FIT unit)
        rec.distance = distance * 100;
    }

    rec.timestamp = to_fit_datetime(data.time);

    Message::from(rec)
}

fn get_file_id(time: DateTime<Local>) -> mesgdef::FileId {
    let mut file_id = mesgdef::FileId::new();
    file_id.manufacturer = typedef::Manufacturer::DEVELOPMENT;
    file_id.product_name = "Suffice".to_string();
    file_id.r#type = typedef::File::ACTIVITY;
    file_id.time_created = to_fit_datetime(time);
    file_id
}

fn get_session(start: DateTime<Local>, end: DateTime<Local>, distance: u32) -> mesgdef::Session {
    let mut session = mesgdef::Session::new();
    session.timestamp = to_fit_datetime(start);
    session.start_time = to_fit_datetime(start);
    session.total_elapsed_time = (end.timestamp() - start.timestamp()) as u32;
    session.total_timer_time = (end.timestamp() - start.timestamp()) as u32;
    session.sport = Sport::CYCLING;
    session.sub_sport = SubSport::INDOOR_CYCLING;
    session.total_distance = distance;
    session
}

fn get_activity(start: DateTime<Local>) -> mesgdef::Activity {
    let mut activity = mesgdef::Activity::new();
    activity.timestamp = to_fit_datetime(start);
    activity.num_sessions = 1;
    activity.local_timestamp = to_fit_local_datetime(start);
    activity
}

// The difference in seconds between the FIT epoch (dec 31 1989) and the UNIX epoch (jan 1 1970).
const FIT_EPOCH_OFFSET: u32 = 631065600;

fn to_fit_datetime(dt: chrono::DateTime<Local>) -> rustyfit::profile::typedef::DateTime {
    rustyfit::profile::typedef::DateTime(dt.timestamp() as u32 - FIT_EPOCH_OFFSET)
}

fn to_fit_local_datetime(dt: chrono::DateTime<Local>) -> rustyfit::profile::typedef::LocalDateTime {
    rustyfit::profile::typedef::LocalDateTime(dt.timestamp() as u32 - FIT_EPOCH_OFFSET)
}

#[test]
fn test_datetime_conversion() {
    let dt = Local::now()
        .timezone()
        .with_ymd_and_hms(2029, 2, 23, 3, 44, 2)
        .unwrap();

    let actual = to_fit_datetime(dt).to_string();
    assert_eq!(actual, "DateTime(1235447042)");

    let actual_local = to_fit_local_datetime(dt).to_string();
    assert_eq!(actual_local, "LocalDateTime(1235447042)");
}
