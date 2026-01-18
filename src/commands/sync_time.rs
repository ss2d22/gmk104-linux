use anyhow::{bail, Result};
use chrono::Local;
use hidapi::HidApi;

use crate::hid::{DeviceSelector, GmkDevice};

use super::init::build_time_config_report;

pub fn run_sync_time(selector: &DeviceSelector) -> Result<()> {
    let api = HidApi::new()?;
    let device = GmkDevice::open(&api, selector)?;

    let now = Local::now();

    device.drain()?;

    let config = build_time_config_report();
    let ack = device.send_with_ack(&config)?;

    if !ack.is_ok() {
        bail!("Time sync failed: {}", ack);
    }

    println!(
        "Time synchronized: {}",
        now.format("%Y-%m-%d %H:%M:%S %Z")
    );
    Ok(())
}
