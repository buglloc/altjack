use std::time::Duration;

use clap::{ArgAction, Command, arg};
use serde_json::json;

use altjack::hid_device;
use altjack::usb_device;

fn cli() -> Command {
    Command::new("altjack")
        .about("AltJack CLI utility")
        .arg_required_else_help(true)
        .subcommand_required(true)
        .allow_external_subcommands(false)
        .arg(arg!(--serial <serial> "Serial number of the target AltJack device").global(true))
        .arg(
            arg!(--ports <ports> "Ports to operate on (comma-separated)")
                .action(ArgAction::Append)
                .value_delimiter(',')
                .value_parser(|s: &str| {
                    let val: u8 = s.parse().map_err(|_| "Not a valid number")?;
                    if altjack::USABLE_PORTS.contains(&val) {
                        Ok(val)
                    } else {
                        Err(format!(
                            "Port must be between in range {:?}",
                            altjack::USABLE_PORTS
                        ))
                    }
                })
                .global(true),
        )
        .subcommand(Command::new("list").about("List connected AltJacks"))
        .subcommand(
            Command::new("touch").about("Touch port").arg(
                arg!(--duration <duration> "Touch duration")
                    .value_parser(clap::builder::ValueParser::from(humantime::parse_duration))
                    .default_value("500ms"),
            ),
        )
        .subcommand(Command::new("state").about("Port state"))
        .subcommand(Command::new("on").about("Turn port on"))
        .subcommand(Command::new("off").about("Turn port off"))
        .subcommand(
            Command::new("cycle").about("Cycle port power").arg(
                arg!(--delay <delay> "Cycle delay")
                    .value_parser(clap::builder::ValueParser::from(humantime::parse_duration))
                    .default_value("1s"),
            ),
        )
        .subcommand(Command::new("toggle").about("Toggle port power"))
}

fn open_hid_device(serial: &str) -> anyhow::Result<hid_device::Device> {
    let devices = match hid_device::list(serial) {
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
            "more than one AltJack was found, please use --serial to specify concrette device"
        ),
    }
}

fn open_usb_device(serial: &str) -> anyhow::Result<usb_device::Device> {
    let mut devices = match usb_device::list(serial) {
        Ok(devices) => devices,
        Err(e) => anyhow::bail!("unable to list devices: {e}"),
    };

    let di = match (devices.next(), devices.next()) {
        (Some(first), None) => first,
        (None, _) => anyhow::bail!("AltJack was not found"),
        (_, Some(_)) => anyhow::bail!(
            "more than one AltJack was found, please use --serial to specify concrette device"
        ),
    };

    match di.open() {
        Ok(dev) => Ok(dev),
        Err(e) => anyhow::bail!("unable to open device: {e}"),
    }
}

fn run() -> anyhow::Result<()> {
    let matches = cli().get_matches();

    let serial = matches
        .get_one::<String>("serial")
        .map(|s| s.as_str())
        .unwrap_or_default();

    let ports: Vec<_> = match matches.get_many::<u8>("ports") {
        Some(port) => port.copied().collect(),
        None => altjack::USABLE_PORTS.collect::<Vec<_>>(),
    };

    match matches.subcommand() {
        Some(("list", _sub_matches)) => {
            let devices = match usb_device::list(serial) {
                Ok(devices) => devices,
                Err(e) => anyhow::bail!("unable to list devices: {e}"),
            };

            let out = json!(
                devices
                    .into_iter()
                    .map(|di| {
                        match di.open() {
                            Ok(dev) => json!({
                                "dev": di,
                                "ports": ports
                                    .iter()
                                    .map(|&port| {
                                        match dev.port(port).state() {
                                            Ok(state) => json!(state),
                                            Err(e) => json!({
                                                "port": port,
                                                "err": format!("opening port: {e}")
                                            })
                                        }
                                    })
                                    .collect::<Vec<_>>(),
                            }),
                            Err(e) => json!({
                                "dev": di,
                                "err": format!("opening device: {e}"),
                            }),
                        }
                    })
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        Some(("touch", touch_matches)) => {
            let duration = touch_matches
                .get_one::<Duration>("duration")
                .expect("ship happens");

            let dev = open_hid_device(serial)?;
            let out = json!(
                ports
                    .iter()
                    .map(|&port| {
                        match dev.touch(port, duration) {
                            Ok(_) => json!({
                                "port": port,
                                "touched": true,
                            }),
                            Err(e) => json!({
                                "port": port,
                                "err": e.to_string(),
                            }),
                        }
                    })
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        Some(("state", _sub_matches)) => {
            let dev = open_usb_device(serial)?;
            let out = json!(
                ports
                    .iter()
                    .map(|&port| {
                        match dev.port(port).state() {
                            Ok(pi) => json!(pi),
                            Err(e) => json!({
                                "port": port,
                                "err": e.to_string(),
                            }),
                        }
                    })
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        Some(("on", _sub_matches)) => {
            let dev = open_usb_device(serial)?;
            let out = json!(
                ports
                    .iter()
                    .map(|&port| {
                        match dev.port(port).on() {
                            Ok(_) => json!({
                                "port": port,
                                "powered": true,
                            }),
                            Err(e) => json!({
                                "port": port,
                                "err": e.to_string(),
                            }),
                        }
                    })
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        Some(("off", _sub_matches)) => {
            let dev = open_usb_device(serial)?;
            let out = json!(
                ports
                    .iter()
                    .map(|&port| {
                        match dev.port(port).off() {
                            Ok(_) => json!({
                                "port": port,
                                "powered": false,
                            }),
                            Err(e) => json!({
                                "port": port,
                                "err": e.to_string(),
                            }),
                        }
                    })
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        Some(("cycle", cycle_matches)) => {
            let dev = open_usb_device(serial)?;
            let out = json!(
                ports
                    .iter()
                    .map(|&port| {
                        if let Err(e) = dev.port(port).off() {
                            return json!({
                                "port": port,
                                "err": format!("unable to power off: {e}"),
                            });
                        }

                        std::thread::sleep(
                            *cycle_matches
                                .get_one::<Duration>("delay")
                                .expect("ship happens"),
                        );

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
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        Some(("toggle", _sub_matches)) => {
            let dev = open_usb_device(serial)?;
            let out = json!(
                ports
                    .iter()
                    .map(|&port| {
                        let mut pi = dev.port(port);
                        let powered = match pi.state() {
                            Ok(state) => state.powered,
                            Err(e) => {
                                return json!({
                                    "port": port,
                                    "err": format!("unable to get port state: {e}"),
                                });
                            }
                        };

                        let rc = if powered { pi.off() } else { pi.on() };

                        match rc {
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
                    .collect::<Vec<_>>()
            );
            println!("{}", out);
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    std::process::exit(0);
}
