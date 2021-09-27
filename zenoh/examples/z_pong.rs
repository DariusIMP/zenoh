//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//
use clap::{App, Arg};
use zenoh::prelude::ResKey::*;
use zenoh::prelude::*;
use zenoh::publisher::CongestionControl;

fn main() {
    // initiate logging
    env_logger::init();

    let config = parse_args();

    let session = zenoh::open(config).wait().unwrap();

    // The resource to read the data from
    let reskey_ping = RId(session.register_resource("/test/ping").wait().unwrap());

    // The resource to echo the data back
    let reskey_pong = RId(session.register_resource("/test/pong").wait().unwrap());
    let _publ = session.publishing(&reskey_pong).wait().unwrap();

    let mut sub = session.subscribe(&reskey_ping).wait().unwrap();

    while let Ok(sample) = sub.receiver().recv() {
        session
            .put(&reskey_ping, sample.value)
            // Make sure to not drop messages because of congestion control
            .congestion_control(CongestionControl::Block)
            .wait()
            .unwrap();
    }
}

fn parse_args() -> Properties {
    let args = App::new("zenoh roundtrip pong example")
        .arg(
            Arg::from_usage("-m, --mode=[MODE]  'The zenoh session mode (peer by default).")
                .possible_values(&["peer", "client"]),
        )
        .arg(Arg::from_usage(
            "-e, --peer=[LOCATOR]...   'Peer locators used to initiate the zenoh session.'",
        ))
        .arg(Arg::from_usage(
            "-l, --listener=[LOCATOR]...   'Locators to listen on.'",
        ))
        .arg(Arg::from_usage(
            "--no-multicast-scouting 'Disable the multicast-based scouting mechanism.'",
        ))
        .get_matches();

    let mut config = Properties::default();
    for key in ["mode", "peer", "listener"].iter() {
        if let Some(value) = args.values_of(key) {
            config.insert(key.to_string(), value.collect::<Vec<&str>>().join(","));
        }
    }
    if args.is_present("no-multicast-scouting") {
        config.insert("multicast_scouting".to_string(), "false".to_string());
    }

    config
}