extern crate futures;
extern crate tokio_core;
extern crate hyper;

use self::futures::{Future, Stream};
use self::hyper::Client;
use self::tokio_core::reactor::Core;

use super::{Pdsc, Vidx, Pidx, Error, PIDX_SUFFIX};

pub fn flatten_to_pdsc(vidx: Vidx) -> Result<Vec<Pdsc>, Error> {
    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let mut pdscs = Vec::new();
    if let Some(more) = vidx.pdsc_index {
        pdscs.extend(more);
    }
    if let Some(vidxs) = vidx.vendor_index {
        for Pidx{url, vendor, ..} in vidxs {
            let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
            match urlname.parse() {
                Ok(uri) => {
                    let work = client.get(uri).and_then(|res|{
                        res.body().concat2().and_then(|body| {
                            if let Ok(next_vidx) = Vidx::from_str(
                                String::from_utf8_lossy(body.as_ref())
                                    .into_owned()
                                    .as_str()){
                                if let Some(more) = next_vidx.pdsc_index {
                                    pdscs.extend(more)
                                }
                            }
                            Ok(())
                        })
                    });
                    core.run(work).unwrap()
                }
                Err(e) => {
                    println!("{}", e)
                }
            }
        }
    }
    Ok(pdscs)
}
