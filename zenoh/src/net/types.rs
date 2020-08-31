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
use crate::net::Session;
use async_std::stream::Stream;
use async_std::sync::{Arc, Receiver, RwLock, Sender, TrySendError};
use pin_project_lite::pin_project;
use std::fmt;

/// A read-only bytes buffer.
pub use zenoh_protocol::io::RBuf;

/// A writable bytes buffer.
pub use zenoh_protocol::io::WBuf;

/// A numerical Id mapped to a resource name with [declare_resource](Session::declare_resource).
pub use zenoh_protocol::core::ResourceId;

/// Informations to configure a subscription.
pub use zenoh_protocol::core::SubInfo;

/// The global unique id of a zenoh peer.
pub use zenoh_protocol::core::PeerId;

/// A time period.
pub use zenoh_protocol::core::Period;

/// The [Queryable](Queryable)s that should be target of a [query](Session::query).
pub use zenoh_protocol::core::Target;

/// The [Queryable](Queryable)s that should be target of a [query](Session::query).
pub use zenoh_protocol::core::QueryTarget;

/// The kind of consolidation that should be applied on replies to a [query](Session::query).
pub use zenoh_protocol::core::QueryConsolidation;

/// The kind of reliability.
pub use zenoh_protocol::core::Reliability;

/// A resource key.
pub use zenoh_protocol::core::ResKey;

/// The subscription mode.
pub use zenoh_protocol::core::SubMode;

/// A zenoh integer.
pub use zenoh_protocol::core::ZInt;

pub use zenoh_protocol::core::whatami;

/// A zenoh Hello message.
pub use zenoh_protocol::proto::Hello;

/// Some informations about the associated data.
///
/// # Examples
/// ```
/// # use zenoh_protocol::io::RBuf;
/// # use zenoh_protocol::proto::DataInfo;
/// # let sample = zenoh::net::Sample { res_name: "".to_string(), payload: RBuf::new(), data_info: None };
/// if let Some(info) = sample.data_info {
///     match info.timestamp {
///         Some(ts) => println!("Sample's timestamp: {}", ts),
///         None => println!("Sample has no timestamp"),
///     }
/// }
/// ```
pub use zenoh_protocol::proto::DataInfo;

/// A zenoh error.
pub use zenoh_util::core::ZError;

/// The kind of zenoh error.
pub use zenoh_util::core::ZErrorKind;

/// A zenoh result.
pub use zenoh_util::core::ZResult;

/// Struct to pass to [open](fn.open.html) to configure the zenoh-net [Session](struct.Session.html).
pub type Config = zenoh_router::runtime::Config;

/// A list of key/value pairs.
pub type Properties = Vec<(ZInt, Vec<u8>)>;

pin_project! {
    /// A stream of [Hello](Hello) messages.
    #[derive(Clone, Debug)]
    pub struct HelloStream {
        #[pin]
        pub(crate) hello_receiver: Receiver<Hello>,
        pub(crate) stop_sender: Sender<()>,
    }
}

impl Stream for HelloStream {
    type Item = Hello;

    #[inline(always)]
    fn poll_next(
        self: async_std::pin::Pin<&mut Self>,
        cx: &mut async_std::task::Context,
    ) -> async_std::task::Poll<Option<Self::Item>> {
        self.project().hello_receiver.poll_next(cx)
    }
}

/// A zenoh value.
#[derive(Debug, Clone)]
pub struct Sample {
    pub res_name: String,
    pub payload: RBuf,
    pub data_info: Option<DataInfo>,
}

/// The callback that will be called on each data for a [CallbackSubscriber](CallbackSubscriber).
pub type DataHandler = dyn FnMut(Sample) + Send + Sync + 'static;

/// Structs received b y a [Queryable](Queryable).
pub struct Query {
    pub res_name: String,
    pub predicate: String,
    pub replies_sender: RepliesSender,
}

impl Query {
    #[inline(always)]
    pub async fn reply(&'_ self, msg: Sample) {
        self.replies_sender.send(msg).await
    }

    #[inline(always)]
    pub fn try_reply(&self, msg: Sample) -> Result<(), TrySendError<Sample>> {
        self.replies_sender.try_send(msg)
    }
}

/// Structs returned by a [query](Session::query).
pub struct Reply {
    pub data: Sample,
    pub source_kind: ZInt,
    pub replier_id: PeerId,
}

pub(crate) type Id = usize;

#[derive(Debug)]
pub(crate) struct PublisherState {
    pub(crate) id: Id,
    pub(crate) reskey: ResKey,
}

/// A publisher.
pub struct Publisher {
    pub(crate) state: Arc<PublisherState>,
}

impl fmt::Debug for Publisher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.state.fmt(f)
    }
}

pub struct SubscriberState {
    pub(crate) id: Id,
    pub(crate) reskey: ResKey,
    pub(crate) resname: String,
    pub(crate) sender: Sender<Sample>,
}

impl fmt::Debug for SubscriberState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Subscriber{{ id:{}, resname:{} }}",
            self.id, self.resname
        )
    }
}

/// A subscriber that provides data through a stream.
pub struct Subscriber {
    pub(crate) session: Session,
    pub(crate) state: Arc<SubscriberState>,
    pub(crate) receiver: Receiver<Sample>,
}

impl Subscriber {
    /// Get the stream from a [Subscriber](Subscriber).
    ///
    /// # Examples
    /// ```no_run
    /// # async_std::task::block_on(async {
    /// use zenoh::net::*;
    /// use futures::prelude::*;
    ///
    /// let session = open(Config::peer(), None).await.unwrap();
    /// # let sub_info = SubInfo {
    /// #    reliability: Reliability::Reliable,
    /// #    mode: SubMode::Push,
    /// #    period: None,
    /// # };
    /// let mut subscriber = session.declare_subscriber(&"/resource/name".into(), &sub_info).await.unwrap();
    /// while let Some(sample) = subscriber.stream().next().await {
    ///     println!("Received : {:?}", sample);
    /// }
    /// # })
    /// ```
    #[inline]
    pub fn stream(&mut self) -> &mut Receiver<Sample> {
        &mut self.receiver
    }

    /// Pull available data for a pull-mode [Subscriber](Subscriber).
    ///
    /// # Examples
    /// ```
    /// #![feature(async_closure)]
    /// # async_std::task::block_on(async {
    /// use zenoh::net::*;
    /// use futures::prelude::*;
    ///
    /// let session = open(Config::peer(), None).await.unwrap();
    /// # let sub_info = SubInfo {
    /// #     reliability: Reliability::Reliable,
    /// #     mode: SubMode::Pull,
    /// #     period: None
    /// # };
    /// let mut subscriber = session.declare_subscriber(&"/resource/name".into(), &sub_info).await.unwrap();
    /// async_std::task::spawn(subscriber.stream().clone().for_each(
    ///     async move |sample| { println!("Received : {:?}", sample); }
    /// ));
    /// subscriber.pull();
    /// # })
    /// ```
    pub async fn pull(&self) -> ZResult<()> {
        self.session.pull(&self.state.reskey).await
    }
}

impl fmt::Debug for Subscriber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.state.fmt(f)
    }
}

pub struct CallbackSubscriberState {
    pub(crate) id: Id,
    pub(crate) reskey: ResKey,
    pub(crate) resname: String,
    pub(crate) dhandler: Arc<RwLock<DataHandler>>,
}

impl fmt::Debug for CallbackSubscriberState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CallbackSubscriber{{ id:{}, resname:{} }}",
            self.id, self.resname
        )
    }
}

/// A subscriber that provides data through a callback.
pub struct CallbackSubscriber {
    pub(crate) session: Session,
    pub(crate) state: Arc<CallbackSubscriberState>,
}

impl CallbackSubscriber {
    /// Pull available data for a pull-mode [CallbackSubscriber](CallbackSubscriber).
    ///
    /// # Examples
    /// ```
    /// # async_std::task::block_on(async {
    /// use zenoh::net::*;
    ///
    /// let session = open(Config::peer(), None).await.unwrap();
    /// # let sub_info = SubInfo {
    /// #     reliability: Reliability::Reliable,
    /// #     mode: SubMode::Pull,
    /// #     period: None
    /// # };
    /// let subscriber = session.declare_callback_subscriber(&"/resource/name".into(), &sub_info,
    ///     |sample| { println!("Received : {} {}", sample.res_name, sample.payload); }
    /// ).await.unwrap();
    /// subscriber.pull();
    /// # })
    /// ```
    pub async fn pull(&self) -> ZResult<()> {
        self.session.pull(&self.state.reskey).await
    }
}

impl fmt::Debug for CallbackSubscriber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.state.fmt(f)
    }
}

pub struct QueryableState {
    pub(crate) id: Id,
    pub(crate) reskey: ResKey,
    pub(crate) kind: ZInt,
    pub(crate) q_sender: Sender<Query>,
}

impl fmt::Debug for QueryableState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Queryable{{ id:{}, reskey:{} }}", self.id, self.reskey)
    }
}

/// An entity able to reply to queries.
pub struct Queryable {
    pub(crate) state: Arc<QueryableState>,
    pub(crate) q_receiver: Receiver<Query>,
}

impl Queryable {
    /// Get the stream from a [Queryable](Queryable).
    ///
    /// # Examples
    /// ```no_run
    /// # async_std::task::block_on(async {
    /// use zenoh::net::*;
    /// use zenoh::net::queryable::EVAL;
    /// use futures::prelude::*;
    ///
    /// let session = open(Config::peer(), None).await.unwrap();
    /// let mut queryable = session.declare_queryable(&"/resource/name".into(), EVAL).await.unwrap();
    /// while let Some(query) = queryable.stream().next().await {
    ///     query.reply(Sample{
    ///         res_name: "/resource/name".to_string(),
    ///         payload: "value".as_bytes().into(),
    ///         data_info: None,
    ///     }).await;
    /// }
    /// # })
    /// ```
    #[inline]
    pub fn stream(&mut self) -> &mut Receiver<Query> {
        &mut self.q_receiver
    }
}

impl fmt::Debug for Queryable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.state.fmt(f)
    }
}

/// Struct used by a [Queryable](Queryable) to send replies to queries.
pub struct RepliesSender {
    pub(crate) kind: ZInt,
    pub(crate) sender: Sender<(ZInt, Sample)>,
}

impl RepliesSender {
    #[inline(always)]
    pub async fn send(&'_ self, msg: Sample) {
        self.sender.send((self.kind, msg)).await
    }

    #[inline(always)]
    pub fn try_send(&self, msg: Sample) -> Result<(), TrySendError<Sample>> {
        match self.sender.try_send((self.kind, msg)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(sample)) => Err(TrySendError::Full(sample.1)),
            Err(TrySendError::Disconnected(sample)) => Err(TrySendError::Disconnected(sample.1)),
        }
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.sender.capacity()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.sender.is_empty()
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.sender.is_full()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.sender.len()
    }
}
