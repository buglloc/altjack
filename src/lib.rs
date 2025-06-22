use std::ops::RangeInclusive;

pub mod hid_device;
pub mod usb_device;

pub const ALTJACK_VID: u16 = 0x0451;
pub const USABLE_PORTS: RangeInclusive<u8> = 1u8..=4;

// INA219 I2C addresses for the 4 ports (7-bit addresses)
pub const INA219_ADDRESSES: [u8; 4] = [0x40, 0x41, 0x42, 0x43]; // 1000000, 1000001, 1000010, 1000011

// INA219 register addresses
pub const INA219_REG_CONFIG: u8 = 0x00;
pub const INA219_REG_SHUNT_VOLTAGE: u8 = 0x01;
pub const INA219_REG_BUS_VOLTAGE: u8 = 0x02;
pub const INA219_REG_POWER: u8 = 0x03;
pub const INA219_REG_CURRENT: u8 = 0x04;
pub const INA219_REG_CALIBRATION: u8 = 0x05;

// INA219 configuration values for 5V bus, 100mΩ shunt
pub const INA219_CONFIG_DEFAULT: u16 = 0x199F; // 16V bus, 320mV shunt, 12-bit resolution
pub const INA219_MAX_CURRENT: f32 = 0.9; // 900mA max (my USB spec)
pub const INA219_MAX_CURRENT_LSB: f32 = crate::INA219_MAX_CURRENT / 32768.0;
pub const INA219_SHUNT_RESISTOR: f32 = 0.1; // 100 mOhm
