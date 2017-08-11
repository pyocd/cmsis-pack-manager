extern crate cmsis_pack_manager;

use cmsis_pack_manager::pack_index::Vidx;
use cmsis_pack_manager::parse::FromElem;
use cmsis_pack_manager::pack_index::network::{flatten_to_downloaded_pdscs, Error};

use std::path::Path;

fn main() {
    println!("{:?}",
             Vidx::from_path(Path::new("keil.vidx"))
             .map_err(Error::from)
             .map(flatten_to_downloaded_pdscs));
}
