extern crate cmsis_pack_manager;
extern crate log;

use cmsis_pack_manager::pack_index::Vidx;
use cmsis_pack_manager::parse::FromElem;
use cmsis_pack_manager::config::Config;
use cmsis_pack_manager::logging::log_to_stderr;
use cmsis_pack_manager::pack_index::network::{flatten_to_downloaded_pdscs, Error};
use log::LogLevelFilter;

use std::path::Path;

fn main() {
    log_to_stderr(LogLevelFilter::Warn);
    println!("{:?}",
             Config::new()
             .map_err(Error::from)
             .and_then(|config| {
                 config.read_vidx_list();
                 Vidx::from_path(Path::new("keil.vidx"))
                     .map_err(Error::from)
                     .map(|vidx| flatten_to_downloaded_pdscs(&config, vidx))
             }));
}
