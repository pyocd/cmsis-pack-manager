use futures::prelude::{await, async_block, Future};
use hyper::{Error, Body, Client, Response, StatusCode, Uri};
use hyper::client::Connect;
use hyper::header::Location;
use slog::Logger;

pub(crate) trait ClientRedirExt<C>
where
    C: Connect,
{
    fn redirectable<'a>(
        &'a self,
        uri: Uri,
        logger: &'a Logger,
    ) -> Box<Future<Item = Response, Error = Error> + 'a>;
}

impl<C: Connect> ClientRedirExt<C> for Client<C, Body> {
    fn redirectable<'a>(
        &'a self,
        mut uri: Uri,
        logger: &'a Logger,
    ) -> Box<Future<Item = Response, Error = Error> + 'a> {
        Box::new(async_block!{
            let mut urls = Vec::new();
            loop {
                urls.push(uri.clone());
                let res = await!(self.get(uri))?;
                match res.status() {
                    StatusCode::MovedPermanently |
                    StatusCode::Found |
                    StatusCode::SeeOther |
                    StatusCode::TemporaryRedirect |
                    StatusCode::PermanentRedirect => {
                        let mut new_uri: Uri = res.headers()
                            .get::<Location>()
                            .unwrap_or(&Location::new(""))
                            .parse()?;
                        if let Some(ref old_uri) = urls.last() {
                            if new_uri.authority().is_none() {
                                if let Some(authority) = old_uri.authority() {
                                    new_uri = format!("{}{}", authority, old_uri).parse()?
                                }
                            }
                            debug!(logger, "Redirecting from {} to {}", old_uri, new_uri);
                        }
                        uri = new_uri;
                    }
                    _ => {
                        return Ok(res);
                    }
                }
            }
        })
    }
}
