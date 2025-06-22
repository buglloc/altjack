use clap::ArgMatches;
use serde_json::json;

use crate::commands::CommandResult;
use crate::device::DeviceManager;

pub fn handle(
    _matches: &ArgMatches,
    device_manager: &DeviceManager,
    ports: &[u8],
) -> CommandResult {
    let devices = device_manager.list_usb_devices()?;

    let results: Vec<serde_json::Value> = devices
        .map(|di| match di.open() {
            Ok(dev) => {
                let port_states: Vec<serde_json::Value> = ports
                    .iter()
                    .map(|&port| match dev.port(port).state() {
                        Ok(state) => json!(state),
                        Err(e) => json!({
                            "port": port,
                            "err": format!("opening port: {e}")
                        }),
                    })
                    .collect();

                json!({
                    "dev": di,
                    "ports": port_states,
                })
            }
            Err(e) => json!({
                "dev": di,
                "err": format!("opening device: {e}"),
            }),
        })
        .collect();

    println!("{}", json!(results));
    Ok(())
}
