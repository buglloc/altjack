use std::io::Error;
use std::time::Duration;

use serde::Serialize;

use nusb;
use nusb::transfer::{Control, ControlType, Recipient, TransferError};

const ALTJACK_VID: u16 = 0x0451;
const USB_CLASS_HUB: u8 = 0x09;
const USABLE_PORTS: u8 = 4;
const USB_TIMEOUT_SEC: u64 = 1;

const USB_PORT_STAT_CONNECTION: u16 = 0x0001;
const USB_PORT_STAT_ENABLE: u16 = 0x0002;
const USB_PORT_STAT_OVERCURRENT: u16 = 0x0008;

const USB_PORT_STAT_HIGHT_SPEED_POWER: u16 = 0x0100;
const USB_PORT_STAT_SUPER_SPEED_POWER: u16 = 0x0200;

pub fn list(serial: &str) -> Result<impl Iterator<Item = DeviceInfo>, Error> {
    match nusb::list_devices() {
        Ok(devs) => Ok(devs
            .filter(move |di| {
                if di.vendor_id() != ALTJACK_VID {
                    return false;
                }

                if di.class() != USB_CLASS_HUB {
                    return false;
                }

                if serial.is_empty() {
                    return true;
                }

                match di.serial_number() {
                    Some(di_serial) => di_serial == serial,
                    None => true,
                }
            })
            .map(|di| DeviceInfo {
                vid: di.vendor_id(),
                pid: di.product_id(),
                serial: di.serial_number().map(|s| s.to_string()),
                speed: match di.speed() {
                    Some(speed) => Speed::from_usb(&speed),
                    _ => None,
                },
                usb: di,
            })),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    vid: u16,
    pid: u16,
    serial: Option<String>,
    speed: Option<Speed>,

    #[serde(skip)]
    usb: nusb::DeviceInfo,
}

impl DeviceInfo {
    pub fn open(&self) -> Result<Device, Error> {
        match self.usb.open() {
            Ok(dev) => Ok(Device::new(self, dev)),
            Err(err) => {
                return Err(err);
            }
        }
    }
}

pub struct Device {
    usb: nusb::Device,
    super_speed: bool,
}

impl Device {
    fn new(di: &DeviceInfo, usb: nusb::Device) -> Self {
        Device {
            usb,
            super_speed: match di.speed {
                Some(Speed::Super) | Some(Speed::SuperPlus) => true,
                _ => false,
            },
        }
    }

    pub fn ports(&self) -> impl Iterator<Item = Port> {
        (1..=USABLE_PORTS).map(|p| self.port(p))
    }

    pub fn port(&self, port: u8) -> Port {
        Port::new(&self, port)
    }
}

#[derive(Copy, Clone, Eq, PartialOrd, Ord, PartialEq, Hash, Debug, Serialize)]
#[non_exhaustive]
pub enum Speed {
    /// Low speed (1.5 Mbit)
    Low,

    /// Full speed (12 Mbit)
    Full,

    /// High speed (480 Mbit)
    High,

    /// Super speed (5000 Mbit)
    Super,

    /// Super speed (10000 Mbit)
    SuperPlus,
}

impl Speed {
    pub fn from_usb(s: &nusb::Speed) -> Option<Self> {
        match s {
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
pub struct Port<'dev> {
    pub num: u8,

    #[serde(skip)]
    dev: &'dev Device,
}

#[derive(Debug, Serialize)]
pub struct PortState {
    pub status: u16,
    pub powered: bool,
    pub connected: bool,
    pub enabled: bool,
    pub overcurrent: bool,
}

impl<'dev> Port<'dev> {
    fn new(dev: &'dev Device, num: u8) -> Self {
        Port { num, dev }
    }

    pub fn state(&self) -> Result<PortState, TransferError> {
        let mut ust: [u8; 4] = [0; 4];
        let rc = self.dev.usb.control_in_blocking(
            Control {
                control_type: ControlType::Class,
                recipient: Recipient::Other,
                request: 0x00, // get status
                value: 0,
                index: self.num as u16,
            },
            &mut ust,
            Duration::from_secs(USB_TIMEOUT_SEC),
        );

        match rc {
            Ok(_) => {
                let status = u16::from_le_bytes([ust[0], ust[1]]);
                let mut power_bit: u16 = USB_PORT_STAT_HIGHT_SPEED_POWER;
                if self.dev.super_speed {
                    power_bit = USB_PORT_STAT_SUPER_SPEED_POWER;
                }

                Ok(PortState {
                    status,
                    powered: status & power_bit != 0,
                    connected: status & USB_PORT_STAT_CONNECTION != 0,
                    enabled: status & USB_PORT_STAT_ENABLE != 0,
                    overcurrent: status & USB_PORT_STAT_OVERCURRENT != 0,
                })
            }
            Err(e) => Err(e),
        }
    }

    pub fn on(&mut self) -> Result<(), TransferError> {
        match self.dev.usb.control_out_blocking(
            Control {
                control_type: ControlType::Class,
                recipient: Recipient::Other,
                request: 0x03,          // set feature
                value: 1 << 3,          // feat power
                index: self.num as u16, // port
            },
            &[],
            Duration::from_secs(USB_TIMEOUT_SEC),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn off(&mut self) -> Result<(), TransferError> {
        match self.dev.usb.control_out_blocking(
            Control {
                control_type: ControlType::Class,
                recipient: Recipient::Other,
                request: 0x01,          // clear feature
                value: 1 << 3,          // feat power
                index: self.num as u16, // port
            },
            &[],
            Duration::from_secs(USB_TIMEOUT_SEC),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
