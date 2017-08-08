use futures::Stream;
use futures::future::Executor;
use futures::stream::{FuturesUnordered};
use futures::sync::mpsc::{self, channel};
use hyper::{self, Client, Response, Body};
use tokio_core::reactor::Core;

use minidom;

use super::{PdscRef, Vidx, Pidx};
use ::parse::FromElem;

static PIDX_SUFFIX: &'static str = ".pidx";

error_chain!{
    links{
        MinidomErr(minidom::Error, minidom::ErrorKind);
    }
    foreign_links{
        SinkErr(mpsc::SendError<Vec<PdscRef>>);
        HttpErr(hyper::Error);
    }
}

future_chain!{}

pub fn flatten_to_pdsc(vidx: Vidx) -> Result<Vec<PdscRef>> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);
    let mut jobs = FuturesUnordered::new();
    let mut toret = Vec::new();
    toret.extend(vidx.pdsc_index);
    let (sender, reciever) = channel(vidx.vendor_index.len());
    for Pidx{url, vendor, ..} in vidx.vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let work = client.get(uri)
                    .map(Response::body)
                    .and_then(Body::concat2)
                    .and_then(move |body| {
                        Ok(Vidx::from_string(String::from_utf8_lossy(body.as_ref())
                                             .into_owned()
                                             .as_str())
                           .map(|next_vidx| {next_vidx.pdsc_index})
                           .unwrap_or(Vec::new()))
                    })
                    .map_err(Error::from);
                jobs.push(work);

            }
            Err(e) => {
                println!("{}", e)
            }
        }
    }
    core.execute(jobs.forward(sender).map(|_| {()}).map_err(|_| {()})).unwrap();
    toret.extend(core.run(reciever.concat2()).unwrap());
    Ok(toret)
}
