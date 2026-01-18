use anyhow::{bail, Context, Result};
use hidapi::{DeviceInfo as HidDeviceInfo, HidApi, HidDevice};
use log::{debug, info, trace};
use std::ffi::CString;

use super::protocol::{AckStatus, Report};
use super::{DEFAULT_VID, REPORT_SIZE};

#[derive(Debug, Default, Clone)]
pub struct DeviceSelector {
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub serial: Option<String>,
    pub path: Option<String>,
    pub product_filter: Option<String>,
}

impl DeviceSelector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_vid(mut self, vid: u16) -> Self {
        self.vid = Some(vid);
        self
    }

    pub fn with_pid(mut self, pid: u16) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }

    pub fn effective_vid(&self) -> u16 {
        self.vid.unwrap_or(DEFAULT_VID)
    }

    pub fn effective_pid(&self) -> Option<u16> {
        self.pid.filter(|&p| p != 0)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
}

impl DeviceInfo {
    fn from_hid_info(info: &HidDeviceInfo) -> Self {
        Self {
            path: info.path().to_string_lossy().into_owned(),
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            serial_number: info.serial_number().map(|s| s.to_string()),
            manufacturer: info.manufacturer_string().map(|s| s.to_string()),
            product: info.product_string().map(|s| s.to_string()),
            interface_number: info.interface_number(),
            usage_page: info.usage_page(),
            usage: info.usage(),
        }
    }
}

pub struct GmkDevice {
    device: HidDevice,
    pub info: DeviceInfo,
    read_timeout_ms: i32,
    pub no_ack: bool,
}

impl GmkDevice {
    pub fn enumerate(api: &HidApi, selector: &DeviceSelector) -> Vec<DeviceInfo> {
        let vid = selector.effective_vid();
        let pid = selector.effective_pid();

        api.device_list()
            .filter(|d| {
                if d.vendor_id() != vid {
                    return false;
                }

                if let Some(p) = pid {
                    if d.product_id() != p {
                        return false;
                    }
                }

                if let Some(ref s) = selector.serial {
                    if d.serial_number() != Some(s.as_str()) {
                        return false;
                    }
                }

                if let Some(ref filter) = selector.product_filter {
                    let filter_lower = filter.to_lowercase();
                    let product_match = d
                        .product_string()
                        .map(|p| p.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false);
                    if !product_match {
                        return false;
                    }
                }

                true
            })
            .map(|d| DeviceInfo::from_hid_info(d))
            .collect()
    }

    pub fn enumerate_all(api: &HidApi) -> Vec<DeviceInfo> {
        api.device_list()
            .map(|d| DeviceInfo::from_hid_info(d))
            .collect()
    }

    pub fn open(api: &HidApi, selector: &DeviceSelector) -> Result<Self> {
        let (device, info) = if let Some(path) = &selector.path {
            let cpath =
                CString::new(path.as_str()).context("Invalid device path (contains null byte)")?;
            let device = api
                .open_path(cpath.as_c_str())
                .with_context(|| format!("Failed to open device at path: {}", path))?;

            let info = api
                .device_list()
                .find(|d| d.path().to_string_lossy() == path.as_str())
                .map(|d| DeviceInfo::from_hid_info(d))
                .unwrap_or_else(|| DeviceInfo {
                    path: path.clone(),
                    vendor_id: 0,
                    product_id: 0,
                    serial_number: None,
                    manufacturer: None,
                    product: None,
                    interface_number: -1,
                    usage_page: 0,
                    usage: 0,
                });

            (device, info)
        } else {
            let vid = selector.effective_vid();

            if let Some(serial) = &selector.serial {
                let pid = selector.effective_pid().unwrap_or(0);
                let device = api.open_serial(vid, pid, serial).with_context(|| {
                    format!("Failed to open device {:04x}:{:04x} serial {}", vid, pid, serial)
                })?;

                let info = Self::enumerate(api, selector)
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Device opened but info not found"))?;

                (device, info)
            } else if let Some(pid) = selector.effective_pid() {
                let device = api
                    .open(vid, pid)
                    .with_context(|| format!("Failed to open device {:04x}:{:04x}", vid, pid))?;

                let info = Self::enumerate(api, selector)
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Device opened but info not found"))?;

                (device, info)
            } else {
                let mut devices = Self::enumerate(api, selector);
                if devices.is_empty() {
                    bail!(
                        "No devices found with VID {:04x}. Use 'list' command to see available devices.",
                        vid
                    );
                }

                devices.sort_by(|a, b| {
                    let a_is_lcd = a.usage_page == 0xff1c;
                    let b_is_lcd = b.usage_page == 0xff1c;
                    b_is_lcd.cmp(&a_is_lcd)
                });

                if devices.len() > 1 {
                    debug!(
                        "Multiple devices found ({} total), selecting usage_page=0x{:04x}",
                        devices.len(),
                        devices[0].usage_page
                    );
                }

                let info = devices.into_iter().next().unwrap();
                let cpath = CString::new(info.path.as_str())?;
                let device = api
                    .open_path(cpath.as_c_str())
                    .with_context(|| format!("Failed to open device at path: {}", info.path))?;

                (device, info)
            }
        };

        info!(
            "Opened device {:04x}:{:04x} at {}",
            info.vendor_id, info.product_id, info.path
        );

        Ok(Self {
            device,
            info,
            read_timeout_ms: 1000,
            no_ack: false,
        })
    }

    pub fn set_read_timeout(&mut self, timeout_ms: i32) {
        self.read_timeout_ms = timeout_ms;
    }

    pub fn write_report(&self, report: &Report) -> Result<()> {
        let raw = report.to_bytes();
        debug!(
            "TX: cmd=0x{:02x} checksum=0x{:04x} [{} bytes]",
            report.command(),
            report.checksum(),
            raw.len()
        );
        trace!("TX full: {:02x?}", raw);

        let written = self.device.write(&raw).context("HID write failed")?;

        if written != raw.len() {
            bail!(
                "Short write: expected {} bytes, wrote {}",
                raw.len(),
                written
            );
        }

        Ok(())
    }

    pub fn read_report(&self) -> Result<Option<[u8; REPORT_SIZE]>> {
        let mut buf = [0u8; REPORT_SIZE];
        let read = self
            .device
            .read_timeout(&mut buf, self.read_timeout_ms)
            .context("HID read failed")?;

        if read == 0 {
            trace!("RX: timeout (no data)");
            return Ok(None);
        }

        debug!("RX: {} bytes, first 8: {:02x?}", read, &buf[..8.min(read)]);
        trace!("RX full: {:02x?}", &buf[..read]);

        Ok(Some(buf))
    }

    pub fn read_raw(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>> {
        let mut buf = [0u8; REPORT_SIZE];
        let read = self
            .device
            .read_timeout(&mut buf, timeout_ms)
            .context("HID read failed")?;

        if read == 0 {
            return Ok(None);
        }

        Ok(Some(buf[..read].to_vec()))
    }

    pub fn write_raw(&self, data: &[u8]) -> Result<()> {
        trace!("TX raw: {:02x?}", data);
        let written = self.device.write(data).context("HID write failed")?;
        if written != data.len() {
            bail!(
                "Short write: expected {} bytes, wrote {}",
                data.len(),
                written
            );
        }
        Ok(())
    }

    pub fn read_raw_default(&self) -> Result<Option<Vec<u8>>> {
        self.read_raw(self.read_timeout_ms)
    }

    pub fn drain(&self) -> Result<usize> {
        let mut total_drained = 0;
        let mut consecutive_empty = 0;

        loop {
            let mut buf = [0u8; REPORT_SIZE];
            let read = self.device.read_timeout(&mut buf, 50)?;

            if read == 0 {
                consecutive_empty += 1;
                if consecutive_empty >= 2 {
                    break;
                }
            } else {
                consecutive_empty = 0;
                total_drained += read;
                trace!("Drain: {} bytes: {:02x?}", read, &buf[..read]);
            }
        }

        if total_drained > 0 {
            debug!("Drained {} total bytes", total_drained);
        }
        Ok(total_drained)
    }

    pub fn send_with_ack(&self, report: &Report) -> Result<AckStatus> {
        self.write_report(report)?;

        if self.no_ack {
            return Ok(AckStatus::Ok);
        }

        match self.read_report()? {
            Some(response) => Ok(AckStatus::from_response(report, &response)),
            None => Ok(AckStatus::Timeout),
        }
    }

    pub fn send_no_ack(&self, report: &Report) -> Result<()> {
        self.write_report(report)
    }

    pub fn manufacturer(&self) -> Result<Option<String>> {
        Ok(self
            .device
            .get_manufacturer_string()
            .ok()
            .flatten()
            .map(|s| s.to_string()))
    }

    pub fn product(&self) -> Result<Option<String>> {
        Ok(self
            .device
            .get_product_string()
            .ok()
            .flatten()
            .map(|s| s.to_string()))
    }

    pub fn serial(&self) -> Result<Option<String>> {
        Ok(self
            .device
            .get_serial_number_string()
            .ok()
            .flatten()
            .map(|s| s.to_string()))
    }
}
