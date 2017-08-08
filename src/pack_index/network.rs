use futures::{Stream};
use futures::future::Executor;
use futures::stream::{FuturesUnordered, Concat2};
use futures::sync::mpsc::{self, channel, Receiver};
use hyper::{self, Client, Response, Body};
use tokio_core::reactor::{Core, Handle};

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

fn void<T>(_: T) -> () { () }

pub fn flatten_to_pdsc_future(vendor_index: Vec<Pidx>, client: Client, core: Handle) ->
    Concat2<Receiver<Vec<PdscRef>>> {
    let mut jobs = FuturesUnordered::new();
    let (sender, reciever) = channel(vendor_index.len());
    for Pidx{url, vendor, ..} in vendor_index {
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
    core.execute(jobs.forward(sender).map(void).map_err(void)).unwrap();
    reciever.concat2()
}

pub fn download_pdscs(vidx: Vidx) -> Result<()> {
}

pub fn flatten_to_pdsc(vidx: Vidx) -> Result<Vec<PdscRef>> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let mut toret = Vec::new();
    let client = Client::new(&handle);
    toret.extend(vidx.pdsc_index);
    toret.extend(core.run(flatten_to_pdsc_future(vidx.vendor_index, handle)).unwrap());
    Ok(toret)
}
