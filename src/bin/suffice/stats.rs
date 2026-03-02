/// A module for tracking rolling averages of stats
/// Maybe later graphs/etc.
/// TODO: Could later be moved out of the binary and into the library crate.
use average::Mean;
use fixed_deque::Deque;

#[derive(Debug)]
pub(crate) struct Stats {
    pub(crate) power: fixed_deque::Deque<u16>,
    pub(crate) cadence: fixed_deque::Deque<u8>,
    pub(crate) heart_rate: fixed_deque::Deque<u8>,
}

impl Stats {
    pub(crate) fn rolling_power(&self, n: usize) -> f64 {
        let m: Mean = self.power.iter().rev().take(n).map(|n| *n as f64).collect();
        m.mean()
    }

    pub(crate) fn rolling_cadence(&self, n: usize) -> f64 {
        let m: Mean = self
            .cadence
            .iter()
            .rev()
            .take(n)
            .map(|n| *n as f64)
            .collect();
        m.mean()
    }

    pub(crate) fn rolling_heart_rate(&self, n: usize) -> f64 {
        let m: Mean = self
            .heart_rate
            .iter()
            .rev()
            .take(n)
            .map(|n| *n as f64)
            .collect();
        m.mean()
    }
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            power: Deque::new(30),
            cadence: Deque::new(30),
            heart_rate: Deque::new(30),
        }
    }
}
