/// A module for tracking rolling averages of stats
/// Maybe later graphs/etc.
/// TODO: Could later be moved out of the binary and into the library crate.
use average::Mean;
use fixed_deque::Deque;

#[derive(Debug)]
pub(crate) struct Stat {
    values: fixed_deque::Deque<u32>,
    total: u64,
}

impl Stat {
    pub(crate) fn rolling(&self, n: usize) -> f64 {
        let m: Mean = self.values.iter().take(n).map(|n| *n as f64).collect();
        m.mean()
    }

    pub(crate) fn total(&self) -> u64 {
        self.total
    }

    pub(crate) fn add(&mut self, value: u32) {
        self.values.push_front(value);
        self.total += value as u64;
    }

    pub(crate) fn data(&self) -> Vec<(f64, f64)> {
        self.values
            .clone()
            .make_contiguous()
            .iter()
            .enumerate()
            .map(|(i, d)| (i as f64, *d as f64))
            .collect()
    }
}

impl Default for Stat {
    fn default() -> Self {
        Stat {
            values: Deque::new(60),
            total: 0,
        }
    }
}

#[test]
fn test_rolling() {
    let mut stat = Stat::default();
    stat.add(9000);
    stat.add(90);
    stat.add(9);
    stat.add(2);
    stat.add(3);
    stat.add(2);

    assert_eq!(stat.rolling(2), 2.5);
    assert_eq!(stat.rolling(3).round(), 2.0);
    assert_eq!(stat.rolling(4), 4.0);
    assert_eq!(stat.rolling(5), 21.2);
    assert_eq!(stat.total(), 9106)
}
