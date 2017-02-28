use std::sync::Arc;
use std::ascii::AsciiExt;

use minihttp::Status;
use tokio_core::io::Io;
use futures::future::ok;

use default_error_page::serve_error_page;
use config::redirect::BaseRedirect;
use incoming::{reply, Request, Input};


pub fn base_redirect<S: Io + 'static>(settings: &Arc<BaseRedirect>, inp: Input)
    -> Request<S>
{
    serve_redirect(settings.redirect_to_domain.as_str(), Status::Found, inp)
}


pub fn strip_www_redirect<S: Io + 'static>(inp: Input)
    -> Request<S>
{

    let base_host = inp.headers.host().and_then(|h| {
        if h.len() > 4 && h[0..4].eq_ignore_ascii_case("www.") {
            Some(h.split_at(4).1)
        } else {
            None
        }
    });
    match base_host {
        Some(host) => serve_redirect(host, Status::MovedPermanently, inp),
        None => serve_error_page(Status::NotFound, inp),
    }
}


fn serve_redirect<S: Io + 'static>(host: &str, status: Status, inp: Input)
    -> Request<S>
{
    // TODO: properly identify request scheme
    let dest = format!("http://{}{}", host, inp.headers.path().unwrap_or("/"));
    reply(inp, move |mut e| {
        e.status(status);
        e.add_header("Location", dest);
        e.add_length(0);
        if e.done_headers() {
            // TODO: add HTML with redirect link;
            //      link must be url-encoded;
        }
        Box::new(ok(e.done()))
    })
}
