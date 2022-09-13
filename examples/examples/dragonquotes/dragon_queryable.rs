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
use futures::prelude::*;
use futures::select;
use rand::{thread_rng, Rng};
use std::convert::TryFrom;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::num::ParseIntError;
use std::path::Path;
use std::time::Duration;
use zenoh::config::Config;
use zenoh::prelude::r#async::AsyncResolve;
use zenoh::prelude::*;

const QUOTES_INPUT_PATH: &str = "quotes.txt";

const GET_ANY_PATH: &str = "zenoh/quote/any";
const GET_QUOTE: &str = "zenoh/quote/id/*";

#[async_std::main]
async fn main() {
    // initiate logging
    env_logger::init();

    let get_any_key_expr = KeyExpr::try_from(GET_ANY_PATH).unwrap();
    let get_quote_key_expr = KeyExpr::try_from(GET_QUOTE).unwrap();

    let quotes: Vec<String> = quotes_from_file(QUOTES_INPUT_PATH);

    println!("Opening session...");
    let session = zenoh::open(Config::default()).res().await.unwrap();

    println!("Creating Queryable on '{}'...", GET_ANY_PATH);
    let get_any_queryable = session.declare_queryable(GET_ANY_PATH).res().await.unwrap();

    println!("Creating Queryable on '{}'...", GET_QUOTE);
    let get_quote_queryable = session.declare_queryable(GET_QUOTE).res().await.unwrap();

    println!("Enter 'q' to quit...");
    let mut stdin = async_std::io::stdin();
    let mut input = [0_u8];
    loop {
        select!(
            query = get_any_queryable.recv_async() => {
                let query = query.unwrap();
                println!(">> [Queryable ] Received Query '{}'", query.selector());
                let random_quote = get_random_quote(&quotes);
                query.reply(Ok(Sample::new(get_any_key_expr.clone(), random_quote))).res().await.unwrap();
            },

            query = get_quote_queryable.recv_async() => {
                let query = query.unwrap();
                println!(">> [Queryable ] Received Query '{}'", query.selector());
                if query.key_expr().is_wild() {
                    let all_quotes = get_all_quotes(&quotes);
                    query.reply(Ok(Sample::new(get_quote_key_expr.clone(), all_quotes))).res().await.unwrap();
                } else {
                    let quote = get_quote(&quotes, get_quote_number(query.key_expr()).unwrap());
                    query.reply(Ok(Sample::new(get_quote_key_expr.clone(), quote))).res().await.unwrap();
                }
            },

            _ = stdin.read_exact(&mut input).fuse() => {
                match input[0] {
                    b'q' => break,
                    0 => sleep(Duration::from_secs(1)).await,
                    _ => (),
                }
            }
        );
    }
}

fn get_quote_number(key_expr: &str) -> Result<usize, ParseIntError> {
    return key_expr
        .rfind('/')
        .map(|i| &key_expr[i + 1..])
        .unwrap()
        .parse::<usize>();
}

fn get_random_quote(quotes: &Vec<String>) -> String {
    let mut rng = thread_rng();
    let n = rng.gen_range(0..quotes.len());
    quotes[n].clone()
}

fn get_all_quotes(quotes: &Vec<String>) -> String {
    quotes.join("\n")
}

fn get_quote(quotes: &Vec<String>, number: usize) -> String {
    quotes[number].clone()
}

fn quotes_from_file(input_path: impl AsRef<Path>) -> Vec<String> {
    let file = File::open(input_path).expect("no such file");
    let buf = BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

#[test]
fn get_quote_number_test() {
    let example_key_expr = "zenoh/quotes/12";
    assert_eq!(get_quote_number(example_key_expr).unwrap(), 12);
}
