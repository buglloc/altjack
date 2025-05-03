
use std::time::Duration;

#[macro_use]
extern crate serde_json;
use serde_json::json;

use clap::{arg, Arg, ArgAction, Command};

use altjack::device;


fn cli() -> Command {
    Command::new("altjack")
        .about("AltJack CLI utility")
        .arg(
            arg!(--serial <serial> "Serial number of the target AltJack device")
            .global(true)
        )
        .arg_required_else_help(true)
        .subcommand_required(true)
        .allow_external_subcommands(false)
        .subcommand(
            Command::new("list")
                .about("List info about AltJacks")
        )
        .subcommand(
            Command::new("port")
                .about("Various port actions")
                .arg(
                    Arg::new("num")
                    .long("num")
                    .help("Ports num to operate on (comma-separated)")
                    .action(ArgAction::Append)
                    .value_delimiter(',')
                    .value_parser(|s: &str| {
                        let val: u8 = s.parse().map_err(|_| "Not a valid number")?;
                        if val >= 1 && val <=4 {
                            Ok(val)
                        } else {
                            Err("Port must be between 1 and 4")
                        }
                    })
                    .global(true)
                )
                .subcommand(
                    Command::new("state")
                    .about("Port state")
                )
                .subcommand(
                    Command::new("on")
                    .about("Turn on port")
                )
                .subcommand(
                    Command::new("off")
                    .about("Turn off port")
                )
                .subcommand(
                    Command::new("cycle")
                    .about("Cycle port power")
                    .arg(
                        arg!(--delay <delay> "Cycle delay")
                        .value_parser(clap::builder::ValueParser::from(humantime::parse_duration))
                        .default_value("1s")
                    )
                )
                .subcommand(
                    Command::new("toggle")
                    .about("Toggle port power")
                )
        )
}


fn main() {
    let args = cli().get_matches();

    let serial = match args.get_one::<String>("serial").map(|s| s.as_str()) {
        Some(s) => s,
        None => "",
    };

    match args.subcommand() {
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
                    "ports": dev.ports()
                        .filter(|port| {
                            match port {
                                Ok(_) => true,
                                Err(e) => {
                                    eprintln!("unable to get port status: {e}");
                                    false
                                }
                            }
                        })
                        .map(|port| port.unwrap())
                        .collect::<Vec<_>>(),
                });

                println!("{}", out.to_string());
            }
        }
        Some(("port", port_matches)) => {
            let ports: Vec<_> = match port_matches.get_many::<u8>("num") {
                Some(port) => port.map(|x| *x).collect(),
                None => {
                    eprintln!("Error: --num is required");
                    std::process::exit(1);
                }
            };

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
                    eprintln!("Error: more than one AltJack was found, please use --serial to specify concrette device");
                    std::process::exit(1);
                }
            };

            let dev= match di.open() {
                Ok(dev) => dev,
                Err(e) => {
                    eprintln!("Error: unable to open device: {e}");
                    std::process::exit(1);
                }
            };

            match port_matches.subcommand() {
                Some(("state", _sub_matches)) => {
                    let out = json!(
                        ports.iter().map(|port| {
                            match dev.port(*port) {
                                Ok(pi) => {
                                    pi
                                }
                                Err(e) => {
                                    eprintln!("Error: unable get port {port} state: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }).collect::<Vec<_>>()
                    );
                    println!("{}", out.to_string());
                }
                Some(("on", _sub_matches)) => {
                    let out = json!(
                        ports.iter().map(|port| {
                            match dev.port(*port) {
                                Ok(mut pi) => {
                                    match pi.on() {
                                        Ok(_) => json!({
                                            "num": *port,
                                            "powered": true,
                                        }),
                                        Err(e) => {
                                            eprintln!("Error: unable to power on port {port}: {e}");
                                            std::process::exit(1);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: unable to get port {port}: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }).collect::<Vec<_>>()
                    );
                    println!("{}", out.to_string());
                }
                Some(("off", _sub_matches)) => {
                    let out = json!(
                        ports.iter().map(|port| {
                            match dev.port(*port) {
                                Ok(mut pi) => {
                                    match pi.off() {
                                        Ok(_) => json!({
                                            "num": *port,
                                            "powered": false,
                                        }),
                                        Err(e) => {
                                            eprintln!("Error: unable to power off port {port}: {e}");
                                            std::process::exit(1);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: unable to get port {port}: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }).collect::<Vec<_>>()
                    );
                    println!("{}", out.to_string());
                }
                Some(("cycle", cycle_matches)) => {
                    let out = json!(
                        ports.iter().map(|port| {
                            match dev.port(*port) {
                                Ok(mut pi) => {
                                    match pi.off() {
                                        Ok(_) => {
                                            std::thread::sleep(*cycle_matches
                                                .get_one::<Duration>("delay")
                                                .expect("has default"));

                                            match pi.on() {
                                                Ok(_) => json!({
                                                    "num": *port,
                                                    "powered": false,
                                                }),
                                                Err(e) => {
                                                    eprintln!("Error: unable to power off port {port}: {e}");
                                                    std::process::exit(1);
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("Error: unable to power off port {port}: {e}");
                                            std::process::exit(1);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: unable to get port {port}: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }).collect::<Vec<_>>()
                    );
                    println!("{}", out.to_string());
                }
                Some(("toggle", _sub_matches)) => {
                    let out = json!(
                        ports.iter().map(|port| {
                            match dev.port(*port) {
                                Ok(mut pi) => {
                                    match if pi.powered {pi.off()}  else {pi.on()} {
                                        Ok(_) => json!({
                                            "num": *port,
                                            "powered": !pi.powered,
                                        }),
                                        Err(e) => {
                                            eprintln!("Error: unable to power toggle port {port}: {e}");
                                            std::process::exit(1);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error: unable to get port {port}: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }).collect::<Vec<_>>()
                    );
                    println!("{}", out.to_string());
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}
