extern crate cmsis;
extern crate log;
extern crate clap;
extern crate fern;

use cmsis::config::Config;
use cmsis::pack_index::network::{update_args, update_command, Error};
use cmsis::pdsc::{check_args, check_command};
use log::LogLevelFilter;
use clap::{Arg, App};

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
        .get_matches();

    let myfern = fern::Dispatch::new()
        .chain(std::io::stderr());
    if matches.is_present("verbose") {
        myfern.level(LogLevelFilter::Debug)
            .level_for("hyper", LogLevelFilter::Info)
            .level_for("tokio_core", LogLevelFilter::Info)
            .level_for("tokio_proto", LogLevelFilter::Info)
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{:6} {} {}",
                    record.level(),
                    record.target(),
                    message
                ))
            })
    } else {
        myfern.level(LogLevelFilter::Info)
            .level_for("hyper", LogLevelFilter::Warn)
            .format(|out, message, record| {
                out.finish(format_args!(
                    "{:6} {}",
                    record.level(),
                    message
                ))
            })
    }.apply().unwrap();
    // This   ^ unwrap is necessary, what else would we do on failure of logging?

    match matches.subcommand() {
        ("update", Some(sub_m)) => {
            Config::new()
                .map_err(Error::from)
                .and_then(|config| update_command(&config, sub_m))
                .unwrap();
        }
        ("check", Some(sub_m)) => {
            Config::new()
                .map_err(Error::from)
                .and_then(|config| check_command(&config, sub_m))
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
