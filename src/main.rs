#[macro_use] extern crate log;
#[macro_use] extern crate quick_error;
extern crate env_logger;
extern crate futures;
extern crate quire;
extern crate argparse;
extern crate tokio_core;
extern crate tokio_service;
extern crate minihttp;
extern crate rustc_serialize;

mod config;
mod handler;
mod routing;

use std::io::{self, Write};
use std::time::Duration;
use std::process::exit;

use futures::stream::Stream;
use argparse::{ArgumentParser, Parse, StoreTrue, Print};
use tokio_core::reactor::Core;
use tokio_core::reactor::Interval;

use config::ListenSocket;
use handler::Main;


pub fn main() {
    env_logger::init().unwrap();

    let mut config = String::from("/etc/swindon/main.yaml");
    let mut check = false;
    let mut verbose = false;
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Runs tree of processes");
        ap.refer(&mut config)
          .add_option(&["-c", "--config"], Parse,
            "Configuration file name")
          .metavar("FILE");
        ap.refer(&mut check)
          .add_option(&["-C", "--check-config"], StoreTrue,
            "Check configuration file and exit");
        ap.add_option(&["--version"],
            Print(env!("CARGO_PKG_VERSION").to_string()),
            "Show version");
        ap.refer(&mut verbose)
            .add_option(&["--verbose"], StoreTrue,
            "Print some user-friendly startup messages");
        ap.parse_args_or_exit();
    }

    let mut configurator = match config::Configurator::new(&config) {
        Ok(cfg) => cfg,
        Err(e) => {
            writeln!(&mut io::stderr(), "{}", e).ok();
            exit(1);
        }
    };
    let cfg = configurator.config();

    if check {
        exit(0);
    }

    let mut lp = Core::new().unwrap();
    let handler = Main {
        config: cfg.clone(),
    };
    // TODO(tailhook) do something when config updates
    for sock in &cfg.get().listen {
        match sock {
            &ListenSocket::Tcp(addr) => {
                if verbose {
                    println!("Listening at {}", addr);
                }
                minihttp::serve(&lp.handle(), addr, handler.clone());
            }
        }
    }

    let config_updater = Interval::new(Duration::new(10, 0), &lp.handle())
        .expect("interval created")
        .for_each(move |_| {
            match configurator.try_update() {
                Ok(false) => {}
                Ok(true) => {
                    // TODO(tailhook) update listening sockets
                    info!("Updated config");
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
            Ok(())
        });

    lp.run(config_updater).unwrap();
}
