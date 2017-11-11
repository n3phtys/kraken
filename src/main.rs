#![allow(dead_code)]
#![allow(unused_imports)]

extern crate config;
extern crate lettre;
extern crate futures;
extern crate hyper;

extern crate notify;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use futures::future::Future;
use hyper::server::{Http, Request, Response, Service};
use hyper::{Method, StatusCode};
use futures::Stream;
use futures::future::FutureResult;

use config::*;
use std::sync::RwLock;
use notify::{RecommendedWatcher, DebouncedEvent, Watcher, RecursiveMode};
use std::sync::mpsc::channel;
use std::time::Duration;

use std::fs::File;
use std::io::Write;
use std::io::Read;

use hyper::{Get, Post};
use hyper::header::ContentLength;


fn main() {
    println!("{}", PHRASE);
    let addr = "127.0.0.1:8080".parse().unwrap();

    show_config_changes();
    watch_config_changes();


    let server = Http::new().bind(&addr, || Ok(HelloWorld)).unwrap();
    server.run().unwrap();
}



lazy_static! {
    static ref SETTINGS: RwLock<Config> = RwLock::new({
        let mut settings = Config::default();

        let x = path_to_config_file_and_mkdirs();
        settings.merge(config::File::with_name(x.to_str().unwrap())).unwrap();

        settings
    });
}



struct HelloWorld;

const PHRASE: &'static str = "Hello, World!";
static INDEX: &'static [u8] = b"Try POST /echo";

impl Service for HelloWorld {
    // boilerplate hooking up hyper's server types
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    // The future representing the eventual Response your call will
    // resolve to. This can change to whatever Future you need.
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        futures::future::ok(match (req.method(), req.path()) {
            (&Get, "/") | (&Get, "/echo") => {
                Response::new()
                    .with_header(ContentLength(INDEX.len() as u64))
                    .with_body(INDEX)
            }
            (&Post, "/echo") => {
                let mut res = Response::new();
                if let Some(len) = req.headers().get::<ContentLength>() {
                    res.headers_mut().set(len.clone());
                }
                res.with_body(req.body())
            }
            _ => {
                Response::new()
                    .with_status(StatusCode::NotFound)
            }
        })
    }
}


fn show_config_changes() {
    println!(" * Settings :: \n{:?}",
             SETTINGS
                 .read()
                 .unwrap()
                 .deserialize::<HashMap<String, String>>()
                 .unwrap());
}

fn path_to_config_file_and_mkdirs() -> std::path::PathBuf {
    let mut path = std::env::home_dir().unwrap();
    path.push(".kraken");

    {
        let path = path.clone();
        let _ = std::fs::create_dir_all(path);
    }
    {
        path.push("Settings");
        path.set_extension("toml");
    }

    let path2 = path.clone();

    let f_opt = File::open(path);

    if f_opt.is_ok() {
        println!("File found in {:?}", path2);
    } else {
        let path3 = path2.clone();
        let mut k = File::create(path3).unwrap();
        let str_incl = include_str!("SettingsDefault.toml");
        k.write_all(
            str_incl.as_bytes()).unwrap();
    }

    return path2;
}

fn watch_config_changes() {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.


    let mut path = std::env::home_dir().unwrap();
    path.push(".kraken");

    watcher
        .watch(path, RecursiveMode::NonRecursive)
        .unwrap();

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        match rx.recv() {
            Ok(DebouncedEvent::Write(_)) => {
                println!(" * Settings.toml written; refreshing configuration ...");
                SETTINGS.write().unwrap().refresh().unwrap();
                show_config_changes();
            }

            Err(e) => println!("watch error: {:?}", e),

            _ => {
                // Ignore event
            }
        }
    }
}
