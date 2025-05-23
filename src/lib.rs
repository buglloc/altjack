use std::ops::RangeInclusive;

pub mod hid_device;
pub mod usb_device;

pub const ALTJACK_VID: u16 = 0x0451;
pub const USABLE_PORTS: RangeInclusive<u8> = 1u8..=4;
