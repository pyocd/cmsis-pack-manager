extern crate cmsis_pack_manager;

use cmsis_pack_manager::pack_index::Vidx;
use cmsis_pack_manager::parse::FromElem;
use cmsis_pack_manager::config::Config;
use cmsis_pack_manager::pack_index::network::{flatten_to_downloaded_pdscs, Error};

use std::path::Path;

fn main() {
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
