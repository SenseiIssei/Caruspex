//! The few fields of the OBD-II layer's types that the evaluation modules
//! read. The protocol crate has the full types; whoever calls the analysis
//! copies these fields over (the evaluation does not need the rest, and the
//! analysis crate stays free of the data and protocol crates).

/// A fault code as read from the car.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Fault {
    pub code: String,
    /// "gespeichert"/"stored", "sporadisch"/"intermittent",
    /// "dauerhaft"/"permanent" (active language).
    pub kind: String,
    pub text: String,
    pub throttle_related: bool,
}

/// One named readiness item: the name in the active language and whether the
/// monitor is complete.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ReadinessItem {
    pub name: String,
    pub ready: bool,
}

/// The parsed result of PID 01.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct VehicleStatus {
    pub mil_on: bool,
    pub fault_count: i32,
    pub readiness: Vec<ReadinessItem>,
    pub valid: bool,
    /// Diesel: different readiness monitors.
    pub compression_ignition: bool,
}

/// A result from on-board component monitoring (mode 06): the measured value
/// together with its lower and upper limit.
///
/// On CAN, `component_id` is the monitor ID (OBDMID, e.g. 0xA2 = misfire
/// cylinder 1) and `test_id` the test ID (TID, e.g. 0x0C = misfires in the
/// current driving cycle).
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorResult {
    pub test_id: i32,
    pub component_id: i32,
    /// Plain-text name of the test, if known.
    pub name: String,
    /// Plain-text name of the component.
    pub component: String,
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub unit: String,
    pub has_minimum: bool,
    pub has_maximum: bool,
    pub passed: bool,
}

/// One live value: the PID and its (primary) value.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LiveValue {
    pub pid: i32,
    pub value: f64,
}

impl Default for MonitorResult {
    fn default() -> Self {
        Self {
            test_id: 0,
            component_id: 0,
            name: String::new(),
            component: String::new(),
            value: 0.0,
            minimum: 0.0,
            maximum: 0.0,
            unit: String::new(),
            has_minimum: false,
            has_maximum: false,
            passed: true,
        }
    }
}
