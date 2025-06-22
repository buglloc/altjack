use clap::{ArgAction, Command, arg};

mod commands;
mod device;

use device::DeviceManager;

fn cli() -> Command {
    Command::new("altjack")
        .about("AltJack utility")
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
        .subcommand(
            Command::new("monitor")
                .about("Continuously monitor INA219 measurements")
                .arg(
                    arg!(--interval <interval> "Monitoring interval")
                        .value_parser(clap::builder::ValueParser::from(humantime::parse_duration))
                        .default_value("1s"),
                ),
        )
        .subcommand(Command::new("calibrate").about("Initialize INA219 voltage monitoring"))
        .subcommand(Command::new("voltage").about("Read bus voltage from INA219"))
        .subcommand(Command::new("current").about("Read current from INA219"))
        .subcommand(Command::new("power").about("Read power from INA219"))
        .subcommand(Command::new("measurements").about("Read all voltage measurements from INA219"))
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

    let device_manager = DeviceManager::new(serial);

    let result = match matches.subcommand() {
        Some(("list", sub_matches)) => commands::list::handle(sub_matches, &device_manager, &ports),
        Some(("touch", sub_matches)) => {
            commands::attiny::handle(sub_matches, &device_manager, &ports)
        }
        Some(("state", sub_matches)) => {
            commands::state::handle(sub_matches, &device_manager, &ports)
        }
        Some(("on", sub_matches)) => {
            commands::power::handle_on(sub_matches, &device_manager, &ports)
        }
        Some(("off", sub_matches)) => {
            commands::power::handle_off(sub_matches, &device_manager, &ports)
        }
        Some(("cycle", sub_matches)) => {
            commands::power::handle_cycle(sub_matches, &device_manager, &ports)
        }
        Some(("toggle", sub_matches)) => {
            commands::power::handle_toggle(sub_matches, &device_manager, &ports)
        }
        Some(("calibrate", sub_matches)) => {
            commands::ina219::handle_calibrate(sub_matches, &device_manager, &ports)
        }
        Some(("voltage", sub_matches)) => {
            commands::ina219::handle_voltage(sub_matches, &device_manager, &ports)
        }
        Some(("current", sub_matches)) => {
            commands::ina219::handle_current(sub_matches, &device_manager, &ports)
        }
        Some(("power", sub_matches)) => {
            commands::ina219::handle_power(sub_matches, &device_manager, &ports)
        }
        Some(("measurements", sub_matches)) => {
            commands::ina219::handle_measurements(sub_matches, &device_manager, &ports)
        }
        Some(("monitor", sub_matches)) => {
            commands::ina219::handle_monitor(sub_matches, &device_manager, &ports)
        }
        _ => unreachable!(),
    };

    result?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    std::process::exit(0);
}
