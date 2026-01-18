pub mod device;
pub mod protocol;

pub use device::{DeviceInfo, DeviceSelector, GmkDevice};
pub use protocol::{AckStatus, Report, ReportBuilder};

pub const DEFAULT_VID: u16 = 0x320f;
pub const DEFAULT_PID: u16 = 0x5055;
pub const REPORT_ID: u8 = 0x04;
pub const REPORT_SIZE: usize = 64;
pub const COMMAND_OFFSET: usize = 3;
pub const PAYLOAD_OFFSET: usize = 4;
pub const PAYLOAD_SIZE: usize = 60;
