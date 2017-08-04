extern crate futures;
extern crate tokio_core;
extern crate hyper;

use self::futures::{Future, Stream};
use self::hyper::{Client, Response, Body};
use self::tokio_core::reactor::Core;

use super::{PdscRef, Vidx, Pidx};
use super::parse::{FromElem, Error};
static PIDX_SUFFIX: &'static str = ".pidx";

pub fn flatten_to_pdsc(vidx: Vidx) -> Result<Vec<PdscRef>, Error> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);
    let mut pdscs = Vec::new();
    let turns = vidx.vendor_index.len();
    pdscs.extend(vidx.pdsc_index);
    for Pidx{url, vendor, ..} in vidx.vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let work = client.get(uri)
                    .map(Response::body)
                    .and_then(Body::concat2)
                    .and_then(|body| {
                        Ok(Vidx::from_string(String::from_utf8_lossy(body.as_ref())
                                          .into_owned()
                                          .as_str())
                           .map(|next_vidx| {next_vidx.pdsc_index})
                           .unwrap_or(Vec::new()))
                    });
                pdscs.extend(core.run(work).unwrap_or(Vec::new()))
            }
            Err(e) => {
                println!("{}", e)
            }
        }
    }
    Ok(pdscs)
}
