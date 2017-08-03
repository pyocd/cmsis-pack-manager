extern crate cmsis_pack_manager;

use cmsis_pack_manager::pack_index::{Vidx, Pidx, Pdsc, Error};
use cmsis_pack_manager::pack_index::network::flatten_to_pdsc;

use std::path::Path;
use std::io::{self, Write};

fn main() {
    println!("{:#?}", Vidx::from_path(Path::new("keil.vidx")).and_then(flatten_to_pdsc));
}
