use std::ffi::CString;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hidapi::{HidApi, HidDevice, HidError};
use once_cell::sync::Lazy;
use serde;

const ATTINY_ADDR: u8 = 0x60;

// I2C commands for TUSB8043 HID to I2C proxy
const I2C_READ_CMD: u8 = 0x01;
const I2C_WRITE_STOP_CMD: u8 = 0x02;
const I2C_WRITE_NO_STOP_CMD: u8 = 0x03;

// HID report constants
const REPORT_ID: u8 = 0x00;
const MAX_REPORT_SIZE: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("HID API error: {0}")]
    HidApi(#[from] HidError),

    #[error("Invalid port: {port}. Must be between 1 and 4")]
    InvalidPort { port: u8 },

    #[error("Duration too large: {duration:?}")]
    DurationTooLarge { duration: Duration },

    #[error("I2C read timeout")]
    I2cTimeout,

    #[error("I2C invalid address: {addr}")]
    I2cInvalidAddress { addr: u8 },

    #[error("I2C invalid data")]
    I2cInvalidData,

    #[error("Unexpected response length: expected {expected}, got {actual}")]
    UnexpectedResponseLength { expected: usize, actual: usize },

    #[error("Unknown I2C error code: {code}")]
    UnknownI2cError { code: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Port(u8);

impl Port {
    pub fn new(port: u8) -> Result<Self, DeviceError> {
        if !crate::USABLE_PORTS.contains(&port) {
            return Err(DeviceError::InvalidPort { port });
        }
        Ok(Port(port))
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn attiny_index(&self) -> u8 {
        self.0 - 1
    }

    pub fn ina219_address(&self) -> u8 {
        crate::INA219_ADDRESSES[self.attiny_index() as usize]
    }
}

#[derive(Debug)]
pub struct VoltageMeasurements {
    pub port: Port,
    pub bus_voltage: u32,   // Millivolts
    pub shunt_voltage: u32, // Millivolts
    pub current: u32,       // Milliamps
    pub power: u32,         // Milliwatts
}

static HID_API: Lazy<Mutex<HidApi>> =
    Lazy::new(|| Mutex::new(HidApi::new().expect("Failed to initialize HID Api")));

// ============================================================================
// Device Discovery
// ============================================================================

pub fn list(serial: &str) -> Result<Vec<DeviceInfo>, DeviceError> {
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

    pub fn open(&self) -> Result<Device, DeviceError> {
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
    i2c: I2cOperations,
}

impl Device {
    fn new(dev: HidDevice) -> Self {
        Self {
            i2c: I2cOperations::new(dev),
        }
    }

    /// Get I2C operations interface
    pub fn i2c(&self) -> &I2cOperations {
        &self.i2c
    }

    /// Get ATTINY operations interface
    pub fn attiny(&self) -> AttinyOperations {
        AttinyOperations::new(&self.i2c)
    }

    /// Get INA219 operations interface
    pub fn ina219(&self) -> Ina219Operations {
        Ina219Operations::new(&self.i2c)
    }

    // Convenience methods for backward compatibility
    pub fn touch(&self, port: u8, duration: &Duration) -> Result<(), DeviceError> {
        let port = Port::new(port)?;
        self.attiny().touch(port, duration)
    }

    pub fn init_ina219(&self, port: u8) -> Result<(), DeviceError> {
        let port = Port::new(port)?;
        self.ina219().init(port)
    }

    pub fn read_bus_voltage(&self, port: u8) -> Result<u32, DeviceError> {
        let port = Port::new(port)?;
        self.ina219().read_bus_voltage(port)
    }

    pub fn read_shunt_voltage(&self, port: u8) -> Result<u32, DeviceError> {
        let port = Port::new(port)?;
        self.ina219().read_shunt_voltage(port)
    }

    pub fn read_current(&self, port: u8) -> Result<u32, DeviceError> {
        let port = Port::new(port)?;
        self.ina219().read_current(port)
    }

    pub fn read_power(&self, port: u8) -> Result<u32, DeviceError> {
        let port = Port::new(port)?;
        self.ina219().read_power(port)
    }

    pub fn read_voltage_measurements(&self, port: u8) -> Result<VoltageMeasurements, DeviceError> {
        let port = Port::new(port)?;
        self.ina219().read_voltage_measurements(port)
    }
}

#[derive(Debug, Clone)]
pub struct I2cOperations {
    dev: Arc<Mutex<HidDevice>>,
}

impl I2cOperations {
    fn new(dev: HidDevice) -> Self {
        Self {
            dev: Arc::new(Mutex::new(dev)),
        }
    }

    /// Generic I2C write function
    pub fn write(&self, addr: u8, data: &[u8], stop: bool) -> Result<(), DeviceError> {
        let cmd = if stop {
            I2C_WRITE_STOP_CMD
        } else {
            I2C_WRITE_NO_STOP_CMD
        };
        let data_len = data.len() as u16;

        let mut report = vec![
            REPORT_ID,
            cmd,
            addr,
            (data_len & 0xFF) as u8,
            (data_len >> 8) as u8,
        ];
        report.extend_from_slice(data);

        let dev = self.dev.lock().unwrap();
        dev.write(&report)?;

        // Read the response status
        let mut read_buf = [0u8; MAX_REPORT_SIZE];
        dev.read(&mut read_buf)?;

        match read_buf[0] {
            0x00 => Ok(()),
            0x01 => Err(DeviceError::I2cTimeout),
            0x02 => Err(DeviceError::I2cInvalidAddress { addr }),
            0x03 => Err(DeviceError::I2cInvalidData),
            code => Err(DeviceError::UnknownI2cError { code }),
        }
    }

    /// Generic I2C read function
    pub fn read(&self, addr: u8, length: u16) -> Result<Vec<u8>, DeviceError> {
        let read_report = [
            REPORT_ID,
            I2C_READ_CMD,
            addr,
            (length & 0xFF) as u8,
            (length >> 8) as u8,
        ];

        let dev = self.dev.lock().unwrap();
        dev.write(&read_report)?;

        // Read the response
        let mut read_buf = [0u8; MAX_REPORT_SIZE];
        dev.read(&mut read_buf)?;

        match read_buf[0] {
            0x00 => {
                // Extract the data (starting from index 3)
                let data_length = read_buf[1] as usize;
                let data = read_buf[3..3 + data_length].to_vec();

                Ok(data)
            }
            0x01 => Err(DeviceError::I2cTimeout),
            0x02 => Err(DeviceError::I2cInvalidAddress { addr }),
            0x03 => Err(DeviceError::I2cInvalidData),
            code => Err(DeviceError::UnknownI2cError { code }),
        }
    }
}

#[derive(Debug)]
pub struct AttinyOperations<'a> {
    i2c: &'a I2cOperations,
}

impl<'a> AttinyOperations<'a> {
    fn new(i2c: &'a I2cOperations) -> Self {
        Self { i2c }
    }

    /// "Touches" port with ATTINY (firmware needed to be loaded)
    pub fn touch(&self, port: Port, duration: &Duration) -> Result<(), DeviceError> {
        let millis: u16 =
            duration
                .as_millis()
                .try_into()
                .map_err(|_| DeviceError::DurationTooLarge {
                    duration: *duration,
                })?;

        // Prepare data for ATTINY: [0x03, port-1, millis_low, millis_high]
        let data = [
            port.attiny_index(),
            millis.to_le_bytes()[0],
            millis.to_le_bytes()[1],
        ];

        self.i2c.write(ATTINY_ADDR, &data, true)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Ina219Operations<'a> {
    i2c: &'a I2cOperations,
}

impl<'a> Ina219Operations<'a> {
    fn new(i2c: &'a I2cOperations) -> Self {
        Self { i2c }
    }

    /// Initialize INA219 device on the specified port
    pub fn init(&self, port: Port) -> Result<(), DeviceError> {
        // Write configuration register
        self.write_register(port, crate::INA219_REG_CONFIG, crate::INA219_CONFIG_DEFAULT)?;

        let calibration =
            (0.04096 / (crate::INA219_MAX_CURRENT_LSB * crate::INA219_SHUNT_RESISTOR)) as u16;
        self.write_register(port, crate::INA219_REG_CALIBRATION, calibration)?;

        Ok(())
    }

    /// Read bus voltage from INA219 device on the specified port in mV
    pub fn read_bus_voltage(&self, port: Port) -> Result<u32, DeviceError> {
        let raw_value = self.read_register(port, crate::INA219_REG_BUS_VOLTAGE)?;

        // INA219 bus voltage is 13-bit, LSB = 4mV
        // Remove the 3 LSBs and multiply by 4mV
        let voltage_mv = (((raw_value >> 3) & 0x1FFF) * 4) as u32;

        Ok(voltage_mv)
    }

    /// Read shunt voltage from INA219 device on the specified port in mV
    pub fn read_shunt_voltage(&self, port: Port) -> Result<u32, DeviceError> {
        let raw_value = self.read_register(port, crate::INA219_REG_SHUNT_VOLTAGE)?;

        // INA219 shunt voltage is 16-bit, LSB = 10μV
        // Convert to signed value and multiply by 10μV
        let voltage_uv = (raw_value as i16) * 10;
        let voltage_mv = (voltage_uv as f32 / 1000.0) as u32;

        Ok(voltage_mv)
    }

    /// Read current from INA219 device on the specified port in mA
    pub fn read_current(&self, port: Port) -> Result<u32, DeviceError> {
        let raw_value = self.read_register(port, crate::INA219_REG_CURRENT)?;

        let current_ma =
            ((raw_value as i16) as f32 * crate::INA219_MAX_CURRENT_LSB * 1000.0) as u32;

        Ok(current_ma)
    }

    /// Read power from INA219 device on the specified port in mW
    pub fn read_power(&self, port: Port) -> Result<u32, DeviceError> {
        let raw_value = self.read_register(port, crate::INA219_REG_POWER)?;

        let power_lsb_mw: f32 = 20.0 * crate::INA219_MAX_CURRENT_LSB * 1000.0;
        let power_mw = (raw_value as f32 * power_lsb_mw) as u32;

        Ok(power_mw)
    }

    /// Read all voltage measurements from INA219 device on the specified port
    pub fn read_voltage_measurements(
        &self,
        port: Port,
    ) -> Result<VoltageMeasurements, DeviceError> {
        let bus_voltage = self.read_bus_voltage(port)?;
        let shunt_voltage = self.read_shunt_voltage(port)?;
        let current = self.read_current(port)?;
        let power = self.read_power(port)?;

        Ok(VoltageMeasurements {
            port,
            bus_voltage,
            shunt_voltage,
            current,
            power,
        })
    }

    /// Read a register from INA219 device on the specified port
    fn read_register(&self, port: Port, register: u8) -> Result<u16, DeviceError> {
        let i2c_addr = port.ina219_address();

        // First, write the register address (set pointer)
        self.i2c.write(i2c_addr, &[register], false)?;

        // Now read 2 bytes from the register
        let data = self.i2c.read(i2c_addr, 2)?;

        if data.len() != 2 {
            return Err(DeviceError::UnexpectedResponseLength {
                expected: 2,
                actual: data.len(),
            });
        }

        // Extract the 2-byte value (big-endian format from INA219)
        let value = ((data[0] as u16) << 8) | (data[1] as u16);
        Ok(value)
    }

    /// Write a register to INA219 device on the specified port
    fn write_register(&self, port: Port, register: u8, value: u16) -> Result<(), DeviceError> {
        let i2c_addr = port.ina219_address();

        // Write register address and value (3 bytes total)
        let data = [
            register,             // register address
            (value >> 8) as u8,   // value high byte
            (value & 0xFF) as u8, // value low byte
        ];

        self.i2c.write(i2c_addr, &data, true)?;
        Ok(())
    }
}
