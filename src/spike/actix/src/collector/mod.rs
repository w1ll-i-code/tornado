use actix::prelude::*;
use matcher::{EventMessage, MatcherActor};
use reader::uds::{UdsConnectMessage, LineFeedMessage, LineFeedMessageDecoder};
use std::io;
use std::thread;
use tokio_codec::Framed;
use tornado_collector_common::Collector;
use tornado_collector_json::JsonCollector;

pub struct JsonReaderActor {
    pub json_collector: JsonCollector,
    pub matcher_addr: Addr<MatcherActor>,
}

impl JsonReaderActor {
    pub fn start_new(uds_connect_msg: UdsConnectMessage, matcher_addr: Addr<MatcherActor>) {
        JsonReaderActor::create(move |ctx| {

            // Default constructor has no buffer size limits. To be used only with trusted sources.
            let codec = LineFeedMessageDecoder::new();

            let framed = Framed::new(uds_connect_msg.0, codec);
            JsonReaderActor::add_stream(framed, ctx);
            JsonReaderActor { json_collector: JsonCollector::new(), matcher_addr }
        });
    }
}

impl Actor for JsonReaderActor {
    type Context = Context<Self>;
}

/// To use `Framed` with an actor, we have to implement `StreamHandler` trait
impl StreamHandler<LineFeedMessage, io::Error> for JsonReaderActor {
    fn handle(&mut self, msg: LineFeedMessage, _ctx: &mut Self::Context) {
        info!(
            "UnixStreamReaderActor - {:?} - received msg: [{}]",
            thread::current().name(),
            &msg.0
        );

        match self.json_collector.to_event(&msg.0) {
            Ok(event) => self.matcher_addr.do_send(EventMessage { event }),
            Err(e) => error!(
                "JsonReaderActor - {:?} - Cannot unmarshal event from json: {}",
                thread::current().name(),
                e
            ),
        };
    }
}
