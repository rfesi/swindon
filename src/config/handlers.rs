use std::sync::Arc;

use quire::validate::{Enum, Nothing};

use super::chat;
use super::static_files;
use super::proxy;
use super::empty_gif;


#[derive(RustcDecodable, Debug, PartialEq, Eq)]
pub enum Handler {
    SwindonChat(chat::Chat),
    Static(Arc<static_files::Static>),
    SingleFile(Arc<static_files::SingleFile>),
    Proxy(Arc<proxy::Proxy>),
    EmptyGif(Arc<empty_gif::EmptyGif>),
    HttpBin,
    /// This endpoints is for testing websocket implementation. It's not
    /// guaranteed to work in forward compatible manner. We use it for
    /// autobahn tests, but we might choose to change test suite, so don't use
    /// it for something serious.
    WebsocketEcho,
}

pub fn validator<'x>() -> Enum<'x> {
    Enum::new()
    .option("SwindonChat", chat::validator())
    .option("Static", static_files::validator())
    .option("SingleFile", static_files::single_file())
    .option("Proxy", proxy::validator())
    .option("HttpBin", Nothing)
    .option("EmptyGif", empty_gif::validator())
    .option("WebsocketEcho", Nothing)
}
