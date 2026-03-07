use crate::stats::Stat;
use derivative::Derivative;

#[derive(Debug, Derivative)]
#[derivative(Default)]
pub(crate) struct RideState {
    pub(crate) mode: Mode,
    #[derivative(Default(value = "true"))]
    pub(crate) mode_dirty: bool,
    #[derivative(Default(value = "true"))]
    pub(crate) level_dirty: bool,

    #[derivative(Default(value = "false"))]
    pub(crate) is_recording: bool,

    pub(crate) resistance: i16,
    pub(crate) power: i16,
}

#[derive(Debug, Default)]
pub(crate) enum Mode {
    Power,
    #[default]
    Resistance,
}

#[derive(Debug, Default)]
pub(crate) struct Stats {
    pub(crate) power: Stat,
    pub(crate) cadence: Stat,
    pub(crate) heart_rate: Stat,
    pub(crate) speed: Stat,
    pub(crate) distance: Stat,
}
