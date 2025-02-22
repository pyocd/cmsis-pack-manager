extern crate cmsis_pack;

use std::fs::{create_dir_all, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::Error;

use cmsis_pack::update::DownloadConfig;

use directories::ProjectDirs;

pub struct Config {
    pub pack_store: PathBuf,
    pub vidx_list: PathBuf,
}

impl DownloadConfig for Config {
    fn pack_store(&self) -> PathBuf {
        self.pack_store.clone()
    }
}

impl Config {
    pub fn new() -> Result<Config, Error> {
        let proj_dir = match ProjectDirs::from("", "", "cmsis-pack-manager") {
            Some(p) => p,
            None => return Err(anyhow::anyhow!("Could not determine home directory")),
        };

        let pack_store = proj_dir.config_dir().to_path_buf();
        let mut vidx_list = proj_dir.config_dir().to_path_buf();
        vidx_list.push("vendors.list");

        Ok(Config {
            pack_store,
            vidx_list,
        })
    }

    pub fn read_vidx_list(&self) -> Vec<String> {
        let fd = OpenOptions::new().read(true).open(&self.vidx_list);
        match fd.map_err(Error::from) {
            Ok(r) => BufReader::new(r)
                .lines()
                .enumerate()
                .flat_map(|(linenum, line)| {
                    line.map_err(|e| log::error!("Could not parse line #{}: {}", linenum, e))
                        .into_iter()
                })
                .collect(),
            Err(_) => {
                log::warn!("Failed to open vendor index list read only. Recreating.");
                let new_content = vec![String::from("http://www.keil.com/pack/index.pidx")];
                match self.vidx_list.parent() {
                    Some(par) => {
                        create_dir_all(par).unwrap_or_else(|e| {
                            log::error!(
                                "Could not create parent directory for vendor index list.\
                                 Error: {}",
                                e
                            );
                        });
                    }
                    None => {
                        log::error!("Could not get parent directory for vendors.list");
                    }
                }
                match OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&self.vidx_list)
                {
                    Ok(mut fd) => {
                        let lines = new_content.join("\n");
                        fd.write_all(lines.as_bytes()).unwrap_or_else(|e| {
                            log::error!("Could not create vendor list file: {}", e);
                        });
                    }
                    Err(e) => {
                        log::error!("Could not open vendors index list file for writing {}", e)
                    }
                }
                new_content
            }
        }
    }
}
