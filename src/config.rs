#[cfg(not(test))]
use chrono::Local;
use std::path::PathBuf;

#[derive(Default, Debug)]
pub struct Config {}

#[cfg(test)]
impl Config {
    pub fn get_dir(&self) -> PathBuf {
        PathBuf::new()
    }

    pub fn get_recording_name(&self) -> PathBuf {
        let mut fout = self.get_dir();
        fout.push("./output.fit");
        fout
    }
}

#[cfg(not(test))]
impl Config {
    pub fn get_dir(&self) -> std::path::PathBuf {
        let mut dir = std::env::home_dir().unwrap_or_default();
        dir.push(".config/suffice/");
        dir
    }

    pub fn get_recording_name(&self) -> PathBuf {
        let mut fout = self.get_dir();
        let now = Local::now();
        fout.push(format!("recording-{}.fit", now.to_rfc3339()));
        fout
    }
}
