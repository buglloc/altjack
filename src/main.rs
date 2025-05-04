use std::time::Duration;

#[macro_use]
extern crate serde_json;
use serde_json::json;

use clap::{ArgAction, Command, arg};

use altjack::device;

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
                    if device::USABLE_PORTS.contains(&val) {
                        Ok(val)
                    } else {
                        Err(format!(
                            "Port must be between in range {:?}",
                            device::USABLE_PORTS
                        ))
                    }
                })
                .global(true),
        )
        .subcommand(Command::new("list").about("List connected AltJacks"))
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

fn main() {
    let matches = cli().get_matches();

    let serial = matches
        .get_one::<String>("serial")
        .map(|s| s.as_str())
        .unwrap_or_default();

    let ports: Vec<_> = match matches.get_many::<u8>("ports") {
        Some(port) => port.copied().collect(),
        None => device::USABLE_PORTS.collect::<Vec<_>>(),
    };

    match matches.subcommand() {
        Some(("list", _sub_matches)) => {
            let devices = match device::list(serial) {
                Ok(devices) => devices,
                Err(e) => {
                    eprintln!("Error: unable to list devices: {e}");
                    std::process::exit(1);
                }
            };

            for di in devices {
                let dev = match di.open() {
                    Ok(dev) => dev,
                    Err(e) => {
                        eprintln!("Error: unable to open device: {e}");
                        continue;
                    }
                };

                let out = json!({
                    "dev": di,
                    "ports": ports
                        .iter()
                        .filter_map(|port| {
                            match dev.port(*port).state() {
                                Ok(state) => Some(state),
                                Err(e) => {
                                    eprintln!("unable to get port #{port} status: {e}");
                                    None
                                }
                            }
                        })
                        .collect::<Vec<_>>(),
                });

                println!("{}", out);
            }
        }
        _ => {
            let mut devices = match device::list(serial) {
                Ok(devices) => devices,
                Err(e) => {
                    eprintln!("Error: unable to list devices: {e}");
                    std::process::exit(1);
                }
            };

            let di = match (devices.next(), devices.next()) {
                (Some(first), None) => first,
                (None, _) => {
                    eprintln!("Error: AltJack was not found");
                    std::process::exit(1);
                }
                (_, Some(_)) => {
                    eprintln!(
                        "Error: more than one AltJack was found, please use --serial to specify concrette device"
                    );
                    std::process::exit(1);
                }
            };

            let dev = match di.open() {
                Ok(dev) => dev,
                Err(e) => {
                    eprintln!("Error: unable to open device: {e}");
                    std::process::exit(1);
                }
            };

            match matches.subcommand() {
                Some(("state", _sub_matches)) => {
                    let out = json!(
                        ports
                            .iter()
                            .map(|port| {
                                match dev.port(*port).state() {
                                    Ok(pi) => pi,
                                    Err(e) => {
                                        eprintln!("Error: unable get port {port} state: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                    );
                    println!("{}", out);
                }
                Some(("on", _sub_matches)) => {
                    let out = json!(
                        ports
                            .iter()
                            .map(|port| {
                                match dev.port(*port).on() {
                                    Ok(_) => json!({
                                        "port": *port,
                                        "powered": true,
                                    }),
                                    Err(e) => {
                                        eprintln!("Error: unable to power on port {port}: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                    );
                    println!("{}", out);
                }
                Some(("off", _sub_matches)) => {
                    let out = json!(
                        ports
                            .iter()
                            .map(|port| {
                                match dev.port(*port).off() {
                                    Ok(_) => json!({
                                        "port": *port,
                                        "powered": false,
                                    }),
                                    Err(e) => {
                                        eprintln!("Error: unable to power off port {port}: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                    );
                    println!("{}", out);
                }
                Some(("cycle", cycle_matches)) => {
                    let out = json!(
                        ports
                            .iter()
                            .map(|port| {
                                match dev.port(*port).off() {
                                    Ok(_) => (),
                                    Err(e) => {
                                        eprintln!("Error: unable to power off port {port}: {e}");
                                        std::process::exit(1);
                                    }
                                };

                                std::thread::sleep(
                                    *cycle_matches
                                        .get_one::<Duration>("delay")
                                        .expect("ship happens"),
                                );

                                match dev.port(*port).on() {
                                    Ok(_) => (),
                                    Err(e) => {
                                        eprintln!("Error: unable to power on port {port}: {e}");
                                        std::process::exit(1);
                                    }
                                };
                            })
                            .collect::<Vec<_>>()
                    );
                    println!("{}", out);
                }
                Some(("toggle", _sub_matches)) => {
                    let out = json!(
                        ports
                            .iter()
                            .map(|port| {
                                let mut pi = dev.port(*port);
                                let powered = match pi.state() {
                                    Ok(state) => state.powered,
                                    Err(e) => {
                                        eprintln!("Error: unable to get port {port} state: {e}");
                                        std::process::exit(1);
                                    }
                                };

                                match if powered { pi.off() } else { pi.on() } {
                                    Ok(_) => json!({
                                        "port": *port,
                                        "powered": !powered,
                                    }),
                                    Err(e) => {
                                        eprintln!("Error: unable to power toggle port {port}: {e}");
                                        std::process::exit(1);
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                    );
                    println!("{}", out);
                }
                _ => unreachable!(),
            }
        }
    }
}
