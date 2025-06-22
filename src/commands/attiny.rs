use clap::ArgMatches;
use serde_json::json;
use std::time::Duration;

use crate::commands::CommandResult;
use crate::device::DeviceManager;

pub fn handle(matches: &ArgMatches, device_manager: &DeviceManager, ports: &[u8]) -> CommandResult {
    let duration = matches
        .get_one::<Duration>("duration")
        .expect("duration should have default value");

    let dev = device_manager.open_hid_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.touch(port, duration) {
            Ok(_) => json!({
                "port": port,
                "touched": true,
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
