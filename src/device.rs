use altjack::{hid_device, usb_device};
use anyhow::Result;

pub struct DeviceManager {
    serial: String,
}

impl DeviceManager {
    pub fn new(serial: &str) -> Self {
        Self {
            serial: serial.to_string(),
        }
    }

    pub fn open_hid_device(&self) -> Result<hid_device::Device> {
        let devices = match hid_device::list(&self.serial) {
            Ok(devices) => devices,
            Err(e) => anyhow::bail!("unable to list devices: {e}"),
        };

        match devices.len() {
            0 => anyhow::bail!("AltJack was not found"),
            1 => match devices.first().unwrap().open() {
                Ok(dev) => Ok(dev),
                Err(e) => anyhow::bail!("unable to open device: {e}"),
            },
            _ => anyhow::bail!(
                "more than one AltJack was found, please use --serial to specify concrete device"
            ),
        }
    }

    pub fn open_usb_device(&self) -> Result<usb_device::Device> {
        let mut devices = match usb_device::list(&self.serial) {
            Ok(devices) => devices,
            Err(e) => anyhow::bail!("unable to list devices: {e}"),
        };

        let di = match (devices.next(), devices.next()) {
            (Some(first), None) => first,
            (None, _) => anyhow::bail!("AltJack was not found"),
            (_, Some(_)) => anyhow::bail!(
                "more than one AltJack was found, please use --serial to specify concrete device"
            ),
        };

        match di.open() {
            Ok(dev) => Ok(dev),
            Err(e) => anyhow::bail!("unable to open device: {e}"),
        }
    }

    pub fn list_usb_devices(&self) -> Result<impl Iterator<Item = usb_device::DeviceInfo>> {
        match usb_device::list(&self.serial) {
            Ok(devices) => Ok(devices),
            Err(e) => anyhow::bail!("unable to list devices: {e}"),
        }
    }
}
