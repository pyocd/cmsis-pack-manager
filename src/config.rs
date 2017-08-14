use std::path::PathBuf;
use std::io::{self, BufRead, BufReader, Write};
use std::fs::{create_dir_all, OpenOptions};

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
        let fd = OpenOptions::new()
            .read(true)
            .open(&self.vidx_list);
        match fd.map_err(Error::from) {
            Ok(r) => {
                BufReader::new(r)
                    .lines()
                    .flat_map(|line| {
                        line.map_err(Error::from)
                            .into_iter()
                    })
                    .collect()
            }
            Err(_) => {
                println!("Failed to open vendor index list read only. Recreating.");
                let new_content = vec![String::from("www.keil.com/pack/keil.vidx"),
                                   String::from("www.keil.com/pack/keil.pidx")];
                if let Some(par) = self.vidx_list.parent() {
                    create_dir_all(par).unwrap();
                } else {
                    println!("Config directory creation failed")
                }
                if let Ok(mut fd) = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(&self.vidx_list) {
                        let lines = new_content.join("\n");
                        fd.write_all(lines.as_bytes()).unwrap();
                } else {
                    println!("Config file creation failed")
                }
                new_content
            }
        }
    }
}
