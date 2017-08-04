extern crate cmsis_pack_manager;

use cmsis_pack_manager::pack_index::Vidx;
use cmsis_pack_manager::pack_index::parse::FromElem;
use cmsis_pack_manager::pack_index::network::flatten_to_pdsc;

use std::path::Path;

fn main() {
    println!("{:#?}",
             Vidx::from_path(Path::new("keil.vidx"))
             .and_then(flatten_to_pdsc));
}
