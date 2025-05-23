use std::ffi::CString;
use std::sync::Mutex;
use std::time::Duration;

use hidapi::{HidApi, HidDevice, HidError};
use once_cell::sync::Lazy;

// const HID_READ_CMD: u8 = 0x01;
const HID_WRITE_STOP_CMD: u8 = 0x02;
// const HID_WRITE_CMD: u8 = 0x03;
const ATTINY_ADDR: u8 = 0x60;

static HID_API: Lazy<Mutex<HidApi>> =
    Lazy::new(|| Mutex::new(HidApi::new().expect("Failed to initialize HID Api")));

pub fn list(serial: &str) -> Result<Vec<DeviceInfo>, HidError> {
    let api = HID_API.lock().unwrap();

    Ok(api
        .device_list()
        .filter(|di| di.vendor_id() == crate::ALTJACK_VID)
        .filter(|di| serial.is_empty() || di.serial_number().unwrap_or_default() == serial)
        .map(DeviceInfo::new)
        .collect())
}

#[derive(Debug)]
pub struct DeviceInfo {
    vid: u16,
    pid: u16,
    path: CString,
    serial: String,
}

impl DeviceInfo {
    fn new(di: &hidapi::DeviceInfo) -> Self {
        DeviceInfo {
            vid: di.vendor_id(),
            pid: di.product_id(),
            serial: di.serial_number().unwrap_or_default().to_string(),
            path: di.path().to_owned(),
        }
    }

    pub fn open(&self) -> Result<Device, HidError> {
        let api = HID_API.lock().unwrap();

        let dev = if !self.path.as_bytes().is_empty() {
            api.open_path(self.path.as_c_str())
        } else if !self.serial.is_empty() {
            api.open_serial(self.vid, self.pid, &self.serial)
        } else {
            Err(HidError::HidApiError {
                message: "unexpected device info".into(),
            })
        }?;

        Ok(Device::new(dev))
    }
}

#[derive(Debug)]
pub struct Device {
    dev: HidDevice,
}

impl Device {
    fn new(dev: HidDevice) -> Self {
        Self { dev }
    }

    pub fn touch(&self, port: u8, duration: &Duration) -> Result<(), HidError> {
        if !crate::USABLE_PORTS.contains(&port) {
            return Err(HidError::HidApiError {
                message: "invalid port".into(),
            });
        }

        let millis: u16 = duration
            .as_millis()
            .try_into()
            .map_err(|_| HidError::HidApiError {
                message: "duration too large".into(),
            })?;

        let report = [
            0x00, // report id
            HID_WRITE_STOP_CMD,
            ATTINY_ADDR,
            0x03,
            0x00,
            port - 1,
            millis.to_le_bytes()[0],
            millis.to_le_bytes()[1],
        ];

        self.dev.write(&report)?;

        let mut read_buf = [0u8; 64];
        self.dev.read(&mut read_buf)?;

        match read_buf[0] {
            0x00 => Ok(()),
            0x01 => Err(HidError::HidApiError {
                message: "read: timeout".into(),
            }),
            0x02 => Err(HidError::HidApiError {
                message: "read: invalid addr".into(),
            }),
            0x03 => Err(HidError::HidApiError {
                message: "read: invalid data".into(),
            }),
            _ => Err(HidError::HidApiError {
                message: "read: unknown error".into(),
            }),
        }
    }
}
