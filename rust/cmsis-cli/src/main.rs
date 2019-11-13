use slog::{o, debug};
use cmsis_cli::{
    Config,
    update_args,
    update_command,
    install_args,
    install_command,
    check_args,
    check_command,
    dump_devices_args,
    dump_devices_command
};
use clap::{Arg, App};
use slog::Drain;
use failure::Error;

fn main() {
    // Note: This argument parser should do nothing more than handle
    let matches = App::new("CMSIS Pack manager and builder")
        .version("0.1.0")
        .author("Jimmy Brisson")
        .arg(Arg::with_name("verbose").short("v").help(
            "Sets the level of verbosity",
        ))
        .subcommand(update_args())
        .subcommand(check_args())
        .subcommand(dump_devices_args())
        .subcommand(install_args())
        .get_matches();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let log = slog::Logger::root(drain, o!());

    debug!(log, "Logging ready.");

    match matches.subcommand() {
        ("update", Some(sub_m)) => {
            Config::new()
                .map_err(Error::from)
                .and_then(|config| update_command(&config, sub_m, &log))
                .unwrap();
        }
        ("install", Some(sub_m)) => {
            Config::new()
                .map_err(Error::from)
                .and_then(|config| install_command(&config, sub_m, &log))
                .unwrap();
        }
        ("check", Some(sub_m)) => {
            Config::new()
                .map_err(Error::from)
                .and_then(|config| check_command(&config, sub_m, &log))
                .unwrap();
        }
        ("dump-devices", Some(sub_m)) => {
            Config::new()
                .map_err(Error::from)
                .and_then(|config| dump_devices_command(&config, sub_m, &log))
                .unwrap();
        }
        (bad_command, Some(_)) => {
            println!("I did not understand the command {}", bad_command);
        }
        (_, None) => {
            println!("{}", matches.usage());
            println!("Try the help command for more information.");
        }
    }
}
