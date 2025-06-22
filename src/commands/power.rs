use clap::ArgMatches;
use serde_json::json;
use std::time::Duration;

use crate::commands::CommandResult;
use crate::device::DeviceManager;

pub fn handle_on(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_usb_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.port(port).on() {
            Ok(_) => json!({
                "port": port,
                "powered": true,
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

pub fn handle_off(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_usb_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.port(port).off() {
            Ok(_) => json!({
                "port": port,
                "powered": false,
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

pub fn handle_cycle(
    matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_usb_device()?;
    let delay = matches
        .get_one::<Duration>("delay")
        .expect("delay should have default value");

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| {
            if let Err(e) = dev.port(port).off() {
                return json!({
                    "port": port,
                    "err": format!("unable to power off: {e}"),
                });
            }

            std::thread::sleep(*delay);

            if let Err(e) = dev.port(port).on() {
                return json!({
                    "port": port,
                    "err": format!("unable to power on: {e}"),
                });
            }

            json!({
                "port": port,
                "powered": true,
            })
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}

pub fn handle_toggle(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_usb_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| {
            let mut port_interface = dev.port(port);
            let powered = match port_interface.state() {
                Ok(state) => state.powered,
                Err(e) => {
                    return json!({
                        "port": port,
                        "err": format!("unable to get port state: {e}"),
                    });
                }
            };

            let result = if powered {
                port_interface.off()
            } else {
                port_interface.on()
            };

            match result {
                Ok(_) => json!({
                    "port": port,
                    "powered": !powered,
                }),
                Err(e) => json!({
                    "port": port,
                    "err": e.to_string(),
                }),
            }
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}
