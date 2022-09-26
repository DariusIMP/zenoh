//
// Copyright (c) 2022 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use async_std::task::sleep;
use std::fs::File;
use std::io::{BufRead, BufReader, Error};
use std::path::Path;
use std::str;
use std::time::Duration;
use zenoh::config::Config;
use zenoh::prelude::r#async::AsyncResolve;

const ENDPOINT: &str = "dragons";
const QUOTES_INPUT_PATH: &str = "quotes.txt";

#[async_std::main]
async fn main() -> Result<(), Error> {
    // Initiate logging
    env_logger::init();

    println!("Opening session...");
    let session = zenoh::open(Config::default()).res().await.unwrap();

    println!("Declaring a publisher for '{}'...", ENDPOINT);
    let publisher = session.declare_publisher(ENDPOINT).res().await.unwrap();

    let mut idx: i32 = 0;
    let quotes = quotes_from_file(QUOTES_INPUT_PATH);
    for quote in quotes.iter().cycle() {
        sleep(Duration::from_secs(1)).await;
        let buf = format!("[{:4}] {}", idx, quote);
        println!("Putting Data('{}': '{}')...", ENDPOINT, buf);
        publisher.put(buf).res().await.unwrap();
        idx += 1;
    }
    Ok(())
}

fn quotes_from_file(input_path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(input_path).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}
