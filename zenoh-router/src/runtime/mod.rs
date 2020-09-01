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
use crate::routing::broker::Broker;
use crate::runtime::orchestrator::SessionOrchestrator;
use async_std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::fmt;
use std::time::Duration;
use uhlc::HLC;
use zenoh_protocol::core::{whatami, PeerId, WhatAmI};
use zenoh_protocol::link::Locator;
use zenoh_protocol::session::{SessionManager, SessionManagerConfig, SessionManagerOptionalConfig};
use zenoh_util::core::{ZError, ZErrorKind, ZResult};
use zenoh_util::{zerror, zerror2};

pub mod orchestrator;

mod adminspace;
pub use adminspace::AdminSpace;

pub struct RuntimeState {
    pub pid: PeerId,
    pub broker: Arc<Broker>,
    pub orchestrator: SessionOrchestrator,
}

#[derive(Clone)]
pub struct Runtime {
    state: Arc<RwLock<RuntimeState>>,
}

impl Runtime {
    pub async fn new(version: u8, config: Config, id: Option<&str>) -> ZResult<Runtime> {
        let pid = if let Some(s) = id {
            let vec = hex::decode(s).map_err(|e| {
                zerror2!(ZErrorKind::Other {
                    descr: format!("Invalid id: {} - {}", s, e)
                })
            })?;
            let size = vec.len();
            if size > PeerId::MAX_SIZE {
                return zerror!(ZErrorKind::Other {
                    descr: format!("Invalid id size: {} ({} bytes max)", size, PeerId::MAX_SIZE)
                });
            }
            let mut id = [0u8; PeerId::MAX_SIZE];
            id[..size].copy_from_slice(vec.as_slice());
            PeerId::new(size, id)
        } else {
            PeerId::from(uuid::Uuid::new_v4())
        };

        log::debug!("Using PID: {}", pid);

        let hlc = if config.add_timestamp {
            Some(HLC::with_system_time(uhlc::ID::from(&pid)))
        } else {
            None
        };
        let broker = Arc::new(Broker::new(hlc));

        let sm_config = SessionManagerConfig {
            version,
            whatami: config.whatami,
            id: pid.clone(),
            handler: broker.clone(),
        };

        let sm_opt_config = SessionManagerOptionalConfig {
            lease: None,
            keep_alive: None,
            sn_resolution: None,
            batch_size: None,
            timeout: None,
            retries: None,
            max_sessions: None,
            max_links: None,
        };

        let session_manager = SessionManager::new(sm_config, Some(sm_opt_config));
        let mut orchestrator = SessionOrchestrator::new(session_manager, config.whatami);
        match orchestrator.init(config).await {
            Ok(()) => Ok(Runtime {
                state: Arc::new(RwLock::new(RuntimeState {
                    pid,
                    broker,
                    orchestrator,
                })),
            }),
            Err(err) => zerror!(
                ZErrorKind::Other {
                    descr: "".to_string()
                },
                err
            ),
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, RuntimeState> {
        self.state.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, RuntimeState> {
        self.state.write().await
    }

    pub async fn close(&self) -> ZResult<()> {
        self.write().await.orchestrator.close().await
    }

    pub async fn get_pid_str(&self) -> String {
        self.read().await.pid.to_string()
    }
}

/// Struct to pass to [open](../../zenoh/net/fn.open.html) to configure the zenoh-net [Session](../../zenoh/net/struct.Session.html).
///
/// # Examples
/// ```
/// # use zenoh_router::runtime::Config;
/// let config = Config::peer()
///     .add_listener("tcp/0.0.0.0:7447")
///     .add_peer("tcp/10.10.10.10:7447");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub whatami: WhatAmI,
    pub peers: Vec<Locator>,
    pub listeners: Vec<Locator>,
    pub multicast_interface: String,
    pub scouting_delay: Duration,
    pub add_timestamp: bool,
}

impl Config {
    fn default(whatami: WhatAmI) -> Config {
        Config {
            whatami,
            peers: vec![],
            listeners: vec![],
            multicast_interface: "auto".to_string(),
            scouting_delay: Duration::new(0, 250_000_000),
            add_timestamp: false,
        }
    }

    pub fn mode(mut self, w: whatami::Type) -> Self {
        self.whatami = w;
        self
    }

    pub fn peer() -> Config {
        Config::default(whatami::PEER)
    }

    pub fn client() -> Config {
        Config::default(whatami::CLIENT)
    }

    pub fn add_peer(mut self, locator: &str) -> Self {
        self.peers.push(locator.parse().unwrap());
        self
    }

    pub fn add_peers(mut self, locators: Vec<&str>) -> Self {
        self.peers
            .extend(locators.iter().map(|l| l.parse().unwrap()));
        self
    }

    pub fn add_listener(mut self, locator: &str) -> Self {
        self.listeners.push(locator.parse().unwrap());
        self
    }

    pub fn add_listeners(mut self, locators: Vec<&str>) -> Self {
        self.listeners
            .extend(locators.iter().map(|l| l.parse().unwrap()));
        self
    }

    pub fn multicast_interface(mut self, name: String) -> Self {
        self.multicast_interface = name;
        self
    }

    pub fn scouting_delay(mut self, delay: Duration) -> Self {
        self.scouting_delay = delay;
        self
    }

    pub fn parse_mode(m: &str) -> Result<whatami::Type, ()> {
        match m {
            "peer" => Ok(whatami::PEER),
            "client" => Ok(whatami::CLIENT),
            "router" => Ok(whatami::ROUTER),
            "broker" => Ok(whatami::BROKER),
            _ => Err(()),
        }
    }

    pub fn add_timestamp(mut self) -> Self {
        self.add_timestamp = true;
        self
    }
}

impl Default for Config {
    fn default() -> Config {
        Config::default(whatami::PEER)
    }
}
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
