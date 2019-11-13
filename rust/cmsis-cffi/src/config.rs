use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader, Write};
use std::fs::{create_dir_all, OpenOptions};

use cmsis_pack::cmsis_update::DownloadConfig;

use slog::{Logger, error, warn};
use failure::{Error, err_msg};

pub const DEFAULT_VIDX_LIST: [&str; 1] = [
    "http://www.keil.com/pack/index.pidx",
];

pub struct Config {
    pack_store: PathBuf,
}

pub struct ConfigBuilder {
    pack_store: Option<PathBuf>,
}

impl DownloadConfig for Config {
    fn pack_store(&self) -> PathBuf {
        self.pack_store.clone()
    }
}

impl ConfigBuilder {
    pub fn with_pack_store<T: Into<PathBuf>>(self, ps: T) -> Self {
        Self {
            pack_store: Some(ps.into()),
        }
    }

    pub fn build(self) -> Result<Config, Error> {
        let pack_store = match self.pack_store {
            Some(ps) => ps,
            None => {
                return Err(err_msg("Pack Store missing"));
            }
        };
        Ok(Config {
            pack_store,
        })
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            pack_store: None,
        }
    }
}

impl Config {
    pub fn new() -> Result<Config, Error> {
        ConfigBuilder::default().build()
    }
}

pub fn read_vidx_list(vidx_list: &Path, l: &Logger) -> Vec<String> {
    let fd = OpenOptions::new().read(true).open(vidx_list);
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
            let new_content: Vec<String> = DEFAULT_VIDX_LIST
                .iter()
                .map(|s| String::from(*s))
                .collect();
            match vidx_list.parent() {
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
                .open(vidx_list)
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

