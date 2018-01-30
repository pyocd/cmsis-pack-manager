use std::path::PathBuf;
use std::io::{self, BufRead, BufReader, Write};
use std::fs::{create_dir_all, OpenOptions};

use app_dirs::{self, AppInfo, AppDataType, app_root};
use slog::Logger;

error_chain!{
    foreign_links{
        Dirs(app_dirs::AppDirsError);
        Io(io::Error);
    }
}

pub struct Config {
    pub pack_store: PathBuf,
    pub vidx_list: PathBuf,
}

impl Config {
    pub fn new() -> Result<Config> {
        let app_info = AppInfo{name: "cmsis", author: "Arm"};
        let pack_store = app_root(AppDataType::UserData, &app_info)?;
        let mut vidx_list = app_root(AppDataType::UserConfig, &app_info)?;
        vidx_list.push("vendors.list");
        Ok(Config {
            pack_store,
            vidx_list,
        })
    }

    pub fn read_vidx_list(&self, l: &Logger) -> Vec<String> {
        let fd = OpenOptions::new().read(true).open(&self.vidx_list);
        match fd.map_err(Error::from) {
            Ok(r) => {
                BufReader::new(r)
                    .lines()
                    .enumerate()
                    .flat_map(|(linenum, line)| {
                        line.map_err(|e| error!(l, "Could not parse line #{}: {}", linenum, e))
                            .into_iter()
                    })
                    .collect()
            }
            Err(_) => {
                warn!(l, "Failed to open vendor index list read only. Recreating.");
                let new_content = vec![
                    String::from("http://www.keil.com/pack/keil.vidx"),
                    String::from("http://www.keil.com/pack/keil.pidx"),
                ];
                match self.vidx_list.parent() {
                    Some(par) => {
                        create_dir_all(par).unwrap_or_else(|e| {
                            error!(
                                l,
                                "Could not create parent directory for vendor index list.\
                                    Error: {}",
                                e
                            );
                        });
                    }
                    None => {
                        error!(l, "Could not get parent directory for vendors.list");
                    }
                }
                match OpenOptions::new().create(true).write(true).open(
                    &self.vidx_list,
                ) {
                    Ok(mut fd) => {
                        let lines = new_content.join("\n");
                        fd.write_all(lines.as_bytes()).unwrap_or_else(|e| {
                            error!(l, "Could not create vendor list file: {}", e);
                        });
                    }
                    Err(e) => {
                        error!(
                            l,
                            "Could not open vendors index list file for writing {}",
                            e
                        )
                    }
                }
                new_content
            }
        }
    }
}
