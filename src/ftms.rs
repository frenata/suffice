use uuid::*;

pub const RESISTANCE_RANGE: Uuid = uuid!("00002ad6-0000-1000-8000-00805f9b34fb");
pub const POWER_RANGE: Uuid = uuid!("00002ad8-0000-1000-8000-00805f9b34fb");
pub const FEATURES: Uuid = uuid!("00002acc-0000-1000-8000-00805f9b34fb");

pub const MACHINE_STATUS: Uuid = uuid!("00002ada-0000-1000-8000-00805f9b34fb");
pub const TRAINING_STATUS: Uuid = uuid!("00002ad3-0000-1000-8000-00805f9b34fb");

pub const MACHINE_CONTROL: Uuid = uuid!("00002ad9-0000-1000-8000-00805f9b34fb");

// # notify: Indoor Bike Data
// ftms_indoor_bike_data_characteristic_id = "00002ad2-0000-1000-8000-00805f9b34fb"
// # notify: Fitness Machine Status
// ftms_fitness_machine_status_characteristic_id = ""
// # notify: Training Status
// ftms_training_status_characteristic_id = ""
// # (write, indicate): Fitness Machine Control Point
// ftms_fitness_machine_control_point_characteristic_id = (
//     ""
// )
