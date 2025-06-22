use clap::ArgMatches;
use serde_json::json;
use std::thread;
use std::time::Duration;

use crate::commands::CommandResult;
use crate::device::DeviceManager;

pub fn handle_calibrate(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_hid_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.init_ina219(port) {
            Ok(_) => json!({
                "port": port,
                "initialized": true,
                "calibrated": true,
            }),
            Err(e) => json!({
                "port": port,
                "err": e.to_string(),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}

pub fn handle_voltage(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_hid_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.read_bus_voltage(port) {
            Ok(voltage) => json!({
                "port": port,
                "voltage": voltage,
                "unit": "mV",
            }),
            Err(e) => json!({
                "port": port,
                "err": e.to_string(),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}

pub fn handle_current(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_hid_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.read_current(port) {
            Ok(current) => json!({
                "port": port,
                "current": current,
                "unit": "mA",
            }),
            Err(e) => json!({
                "port": port,
                "err": e.to_string(),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}

pub fn handle_power(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_hid_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.read_power(port) {
            Ok(power) => json!({
                "port": port,
                "power": power,
                "unit": "mW",
            }),
            Err(e) => json!({
                "port": port,
                "err": e.to_string(),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}

pub fn handle_measurements(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_hid_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.read_voltage_measurements(port) {
            Ok(measurements) => json!({
                "port": measurements.port,
                "bus_voltage": measurements.bus_voltage,
                "shunt_voltage": measurements.shunt_voltage,
                "current": measurements.current,
                "power": measurements.power,
                "units": {
                    "bus_voltage": "mV",
                    "shunt_voltage": "mV",
                    "current": "mA",
                    "power": "mW",
                },
            }),
            Err(e) => json!({
                "port": port,
                "err": e.to_string(),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}

pub fn handle_monitor(
    matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_hid_device()?;

    let interval = matches
        .get_one::<Duration>("interval")
        .expect("interval should have default value")
        .as_secs_f64();

    for &port in ports {
        if let Err(e) = dev.init_ina219(port) {
            println!(
                "{}",
                json!({
                    "action": "calibrate",
                    "port": port,
                    "status": "error",
                    "error": e.to_string()
                })
            );
            return Ok(());
        }
    }

    loop {
        let results: Vec<serde_json::Value> = ports
            .iter()
            .map(|&port| match dev.read_voltage_measurements(port) {
                Ok(measurements) => json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "port": measurements.port,
                    "bus_voltage": measurements.bus_voltage,
                    "shunt_voltage": measurements.shunt_voltage,
                    "current": measurements.current,
                    "power": measurements.power,
                    "units": {
                        "bus_voltage": "mV",
                        "shunt_voltage": "mV",
                        "current": "mA",
                        "power": "mW",
                    },
                }),
                Err(e) => json!({
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "port": port,
                    "error": e.to_string(),
                }),
            })
            .collect();

        println!("{}", json!(results));

        thread::sleep(Duration::from_secs_f64(interval));
    }
}
