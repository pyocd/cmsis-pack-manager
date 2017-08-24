extern crate cmsis;
extern crate log;
extern crate clap;

use cmsis::config::Config;
use cmsis::logging::log_to_stderr;
use cmsis::pack_index::network::{update_args, update_command, Error};
use log::LogLevelFilter;
use clap::{Arg, App};

fn main() {
    // Note: This argument parser should do nothing more than handle
    let matches =
        App::new("CMSIS Pack manager and builder")
        .version("0.1.0")
        .author("Jimmy Brisson")
        .arg(Arg::with_name("verbose")
             .short("v")
             .help("Sets the level of verbosity"))
        .subcommand(update_args())
        .get_matches();

    if matches.is_present("verbose"){
        log_to_stderr(LogLevelFilter::Info)
    } else {
        log_to_stderr(LogLevelFilter::Warn)
    }.unwrap();
    // ^ This unwrap is necessary, what else would we do on failure of logging?

    match matches.subcommand() {
        ("update", Some(sub_m)) =>{
            Config::new()
                .map_err(Error::from)
                .and_then(|config|{
                    update_command(&config, sub_m)
                }).unwrap();
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
