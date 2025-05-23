use std::fmt::Debug;
use std::io::Error;
use std::time::Duration;

use nusb;
use nusb::transfer::{Control, ControlType, Recipient, TransferError};
use serde::Serialize;

const USB_CLASS_HUB: u8 = 0x09;
const USB_TIMEOUT: Duration = Duration::from_secs(1);

const USB_PORT_STAT_CONNECTION: u16 = 0x0001;
const USB_PORT_STAT_ENABLE: u16 = 0x0002;
const USB_PORT_STAT_OVERCURRENT: u16 = 0x0008;
const USB_PORT_STAT_HS_POWER: u16 = 0x0100;
const USB_PORT_STAT_SS_POWER: u16 = 0x0200;

pub fn list(serial: &str) -> Result<impl Iterator<Item = DeviceInfo>, Error> {
    let devices = nusb::list_devices()?;
    Ok(devices
        .filter(|di| di.vendor_id() == crate::ALTJACK_VID)
        .filter(|di| di.class() == USB_CLASS_HUB)
        .filter(move |di| serial.is_empty() || di.serial_number().is_none_or(|s| s == serial))
        .map(DeviceInfo::new))
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    vid: u16,
    pid: u16,
    serial: String,
    speed: Option<Speed>,

    #[serde(skip)]
    usb: nusb::DeviceInfo,
}

impl DeviceInfo {
    fn new(di: nusb::DeviceInfo) -> Self {
        DeviceInfo {
            vid: di.vendor_id(),
            pid: di.product_id(),
            serial: di.serial_number().unwrap_or_default().to_string(),
            speed: di.speed().and_then(Speed::from_usb),
            usb: di,
        }
    }

    pub fn open(&self) -> Result<Device, Error> {
        self.usb.open().map(|usb| Device::new(self, usb))
    }
}

pub struct Device {
    usb: nusb::Device,
    super_speed: bool,
}

impl Device {
    fn new(info: &DeviceInfo, usb: nusb::Device) -> Self {
        let super_speed = matches!(info.speed, Some(Speed::Super | Speed::SuperPlus));
        Device { usb, super_speed }
    }

    pub fn ports(&self) -> impl Iterator<Item = Port> {
        crate::USABLE_PORTS.map(|p| self.port(p))
    }

    pub fn port(&self, port: u8) -> Port {
        Port::new(self, port)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize)]
#[non_exhaustive]
pub enum Speed {
    Low,
    Full,
    High,
    Super,
    SuperPlus,
}

impl Speed {
    pub fn from_usb(speed: nusb::Speed) -> Option<Self> {
        match speed {
            nusb::Speed::Low => Some(Speed::Low),
            nusb::Speed::Full => Some(Speed::Full),
            nusb::Speed::High => Some(Speed::High),
            nusb::Speed::Super => Some(Speed::Super),
            nusb::Speed::SuperPlus => Some(Speed::SuperPlus),
            _ => None,
        }
    }
}

#[derive(Serialize)]
pub struct Port<'a> {
    pub port: u8,
    #[serde(skip)]
    dev: &'a Device,
}

#[derive(Debug, Serialize)]
pub struct PortState {
    pub port: u8,
    pub status: u16,
    pub powered: bool,
    pub connected: bool,
    pub enabled: bool,
    pub overcurrent: bool,
}

impl<'a> Port<'a> {
    fn new(dev: &'a Device, port: u8) -> Self {
        Self { dev, port }
    }

    pub fn state(&self) -> Result<PortState, TransferError> {
        let mut buf = [0u8; 4];
        let result = self.dev.usb.control_in_blocking(
            Control {
                control_type: ControlType::Class,
                recipient: Recipient::Other,
                request: 0x00, // Get Status
                value: 0,
                index: self.port as u16,
            },
            &mut buf,
            USB_TIMEOUT,
        )?;

        if result != 4 {
            return Err(TransferError::Unknown);
        }

        let status = u16::from_le_bytes([buf[0], buf[1]]);
        let power_bit = if self.dev.super_speed {
            USB_PORT_STAT_SS_POWER
        } else {
            USB_PORT_STAT_HS_POWER
        };

        Ok(PortState {
            port: self.port,
            status,
            powered: status & power_bit != 0,
            connected: status & USB_PORT_STAT_CONNECTION != 0,
            enabled: status & USB_PORT_STAT_ENABLE != 0,
            overcurrent: status & USB_PORT_STAT_OVERCURRENT != 0,
        })
    }

    pub fn on(&mut self) -> Result<(), TransferError> {
        self.control_out(
            Control {
                control_type: ControlType::Class,
                recipient: Recipient::Other,
                request: 0x03, // Set Feature
                value: 1 << 3, // Feat power
                index: self.port as u16,
            },
            &[],
        )
    }

    pub fn off(&mut self) -> Result<(), TransferError> {
        self.control_out(
            Control {
                control_type: ControlType::Class,
                recipient: Recipient::Other,
                request: 0x01, // Clear Feature
                value: 1 << 3, // Feat power
                index: self.port as u16,
            },
            &[],
        )
    }

    fn control_out(&self, control: Control, data: &[u8]) -> Result<(), TransferError> {
        self.dev
            .usb
            .control_out_blocking(control, data, USB_TIMEOUT)?;

        Ok(())
    }
}
