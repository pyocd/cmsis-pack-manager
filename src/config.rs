use std::path::PathBuf;
use std::io::{self, BufRead, BufReader};
use std::fs::{OpenOptions};

use xdg::{self, BaseDirectories};

error_chain!{
    foreign_links{
        Xdg(xdg::BaseDirectoriesError);
        Io(io::Error);
    }
}

pub struct Config{
    pub pack_store: BaseDirectories,
    pub vidx_list: PathBuf,
}

impl Config {
    pub fn new() -> Result<Config> {
        let pack_store = BaseDirectories::with_prefix("cmsis")?;
        let vidx_list = pack_store.place_config_file("vendors.list")?;
        Ok(Config{pack_store, vidx_list})
    }
    pub fn read_vidx_list(&self) -> Vec<String> {
        let default_config = vec![String::from("www.keil.com/pack/keil.vidx"),
                                  String::from("www.keil.com/pack/keil.pidx")];
        match OpenOptions::new().read(true).open(&self.vidx_list) {
            Ok(fd) => {
                let br = BufReader::new(fd);
                let mut res = Vec::new();
                for line in br.lines() {
                    if let Ok(l) = line {
                        res.push(l);
                    }
                }
                res
            }
            Err(_) => {
                println!("No vendor index list found. Creating.");
                default_config
            }
        }
    }
}
