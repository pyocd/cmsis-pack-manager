use futures::{Stream};
use futures::future::Executor;
use futures::stream::{iter, empty, FuturesUnordered, Concat2};
use futures::sync::mpsc::{self, channel, Receiver};
use hyper::{self, Client, Response, Body};
use hyper::client::{Connect};
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
        SinkErr2(mpsc::SendError<PdscRef>);
        HttpErr(hyper::Error);
    }
}

future_chain!{}

fn void<T>(_: T) -> () { () }

pub fn flatten_to_pdsc_future<C>(vendor_index: Vec<Pidx>,
                                 client: Client<C, Body>,
                                 core: Handle) ->
    Receiver<PdscRef>
    where C: Connect {
    let mut job = FuturesUnordered::new();
    let (sender, reciever) = channel(vendor_index.len());
    for Pidx{url, vendor, ..} in vendor_index {
        let urlname = format!("{}{}{}", url, vendor, PIDX_SUFFIX);
        match urlname.parse() {
            Ok(uri) => {
                let work = client.get(uri)
                    .map(Response::body)
                    .flatten_stream()
                    .concat2()
                    .map_err(Error::from)
                    .and_then(move |body| {
                        Vidx::from_string(String::from_utf8_lossy(body.as_ref())
                                          .into_owned()
                                          .as_str())
                            .map_err(Error::from)
                            .map(|next_vidx| {
                                next_vidx.pdsc_index
                                    .into_iter()
                                    .map(Ok::<_, Error>)
                            })
                    });
                job.push(work)

            }
            Err(e) => {
                println!("{}", e)
            }
        }
    }
    core.execute(job.map(|j| iter(j)).flatten().forward(sender).map(void).map_err(void)).unwrap();
    reciever
}

pub fn download_pdscs<F, C>(stream: F,
                            client: Client<C, Body>,
                            core: Handle) -> Result<()>
    where F: Stream<Item = PdscRef>,
          C: Connect{
    unimplemented!()
}

pub fn flatten_to_pdsc(vidx: Vidx) -> Result<Vec<PdscRef>> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let mut toret = Vec::new();
    let client = Client::new(&handle);
    toret.extend(vidx.pdsc_index);
    toret.extend(core.run(flatten_to_pdsc_future(vidx.vendor_index,
                                                 client, handle).collect()).unwrap());
    Ok(toret)
}
