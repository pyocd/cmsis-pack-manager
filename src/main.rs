extern crate cmsis_pack_manager;
extern crate log;

use cmsis_pack_manager::config::Config;
use cmsis_pack_manager::logging::log_to_stderr;
use cmsis_pack_manager::pack_index::network::{flatten, Error};
use log::LogLevelFilter;

fn main() {
    log_to_stderr(LogLevelFilter::Info).unwrap();
    println!("{:?}",
             Config::new()
             .map_err(Error::from)
             .and_then(|config| {
                 let vidx_list = config.read_vidx_list();
                 flatten(&config, vidx_list)
             }));
}
