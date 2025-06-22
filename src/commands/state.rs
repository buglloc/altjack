use clap::ArgMatches;
use serde_json::json;

use crate::commands::CommandResult;
use crate::device::DeviceManager;

pub fn handle(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let dev = device_manager.open_usb_device()?;

    let results: Vec<serde_json::Value> = ports
        .iter()
        .map(|&port| match dev.port(port).state() {
            Ok(state) => json!(state),
            Err(e) => json!({
                "port": port,
                "err": e.to_string(),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}
