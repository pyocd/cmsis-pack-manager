extern crate cmsis_pack_manager;
extern crate log;
extern crate clap;

use cmsis_pack_manager::config::Config;
use cmsis_pack_manager::logging::log_to_stderr;
use cmsis_pack_manager::pack_index::network::{update, Error};
use log::LogLevelFilter;
use clap::{Arg, App, SubCommand};

fn main() {
    let matches =
        App::new("CMSIS Pack manager and builder")
        .version("0.1.0")
        .author("Jimmy Brisson")
        .arg(Arg::with_name("verbose")
             .short("v")
             .help("Sets the level of verbosity"))
        .subcommand(SubCommand::with_name("update")
                    .about("Update CMSIS PDSC files for indexing")
                    .version("0.1.0"))
        .get_matches();
    if matches.is_present("verbose"){
        log_to_stderr(LogLevelFilter::Info)
    } else {
        log_to_stderr(LogLevelFilter::Warn)
    }.unwrap();
    match matches.subcommand_name() {
        Some("update") =>{
            println!("{:#?}",
                     Config::new()
                     .map_err(Error::from)
                     .and_then(|config| {
                         let vidx_list = config.read_vidx_list();
                         update(&config, vidx_list)
                     }))
        }
        Some(bad_command) => {
            println!("I did not understand the command {}", bad_command)
        }
        None => {
            println!("{}", matches.usage())
        }
    }
}
