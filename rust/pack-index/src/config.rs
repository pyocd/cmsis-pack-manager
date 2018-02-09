use std::path::PathBuf;
use std::io::{self, BufRead, BufReader, Write};
use std::fs::{create_dir_all, OpenOptions};

use app_dirs::{self, app_root, AppDataType, AppInfo};
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

pub struct ConfigBuilder {
    pack_store: Option<PathBuf>,
    vidx_list: Option<PathBuf>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            pack_store: None,
            vidx_list: None,
        }
    }

    pub fn with_pack_store<T: Into<PathBuf>>(self, ps: T) -> Self {
        Self {
            pack_store: Some(ps.into()),
            ..self
        }
    }

    pub fn with_vidx_list<T: Into<PathBuf>>(self, vl: T) -> Self {
        Self {
            vidx_list: Some(vl.into()),
            ..self
        }
    }

    pub fn build(self) -> Result<Config> {
        let app_info = AppInfo {
            name: "cmsis",
            author: "Arm",
        };
        let pack_store = match self.pack_store {
            Some(ps) => {
                if !(&ps).exists() {
                    create_dir_all(&ps)?;
                }
                ps
            }
            None => app_root(AppDataType::UserData, &app_info)?,
        };
        let vidx_list = match self.vidx_list {
            Some(vl) => {
                let _ = OpenOptions::new().read(true).open(&vl)?;
                vl
            }
            None => {
                let mut vl = app_root(AppDataType::UserConfig, &app_info)?;
                vl.push("vendors.list");
                vl
            }
        };
        Ok(Config {
            pack_store,
            vidx_list,
        })
    }
}

impl Config {
    pub fn new() -> Result<Config> {
        ConfigBuilder::new().build()
    }

    pub fn read_vidx_list(&self, l: &Logger) -> Vec<String> {
        let fd = OpenOptions::new().read(true).open(&self.vidx_list);
        match fd.map_err(Error::from) {
            Ok(r) => BufReader::new(r)
                .lines()
                .enumerate()
                .flat_map(|(linenum, line)| {
                    line.map_err(|e| error!(l, "Could not parse line #{}: {}", linenum, e))
                        .into_iter()
                })
                .collect(),
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
                match OpenOptions::new()
                    .create(true)
                    .write(true)
                    .open(&self.vidx_list)
                {
                    Ok(mut fd) => {
                        let lines = new_content.join("\n");
                        fd.write_all(lines.as_bytes()).unwrap_or_else(|e| {
                            error!(l, "Could not create vendor list file: {}", e);
                        });
                    }
                    Err(e) => error!(
                        l,
                        "Could not open vendors index list file for writing {}", e
                    ),
                }
                new_content
            }
        }
    }
}
