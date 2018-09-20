use actix::prelude::*;
use actix_broker::{BrokerSubscribe, BrokerIssue};
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded};
use futures::Stream;

use std::collections::HashMap;
use std::io::{Error, Result};

use msgs::*;

type HandlerFn = Fn(&RelayDevice, &[u8]);
type SubscribeFn = Fn(&RelayDevice, &mut Context<RelayDevice>);
type RelayTx = UnboundedSender<RelayData>;
pub type RelayRx = UnboundedReceiver<RelayData>;

struct DeviceSetup<F>(F);
struct RelayDataStream<S>(S);

pub struct RelayDevice {
    name: Option<String>,
    tx: RelayTx, 
    handlers: HashMap<u64, Box<HandlerFn>>,
}

pub struct RelayDeviceBuilder<F, S>
where
    S: Stream<Item = RelayData, Error = Error>,
    F: FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
        -> Result<RelayDataStream<S>>,
{
    setup: DeviceSetup<F>,
    name: Option<String>,
    subs: Vec<Box<SubscribeFn>>,
    handlers: HashMap<u64, Box<HandlerFn>>,
}

impl RelayDevice {
    fn new<F, S>(f: F) -> RelayDeviceBuilder<F, S>
    where
        S: Stream<Item = RelayData, Error = Error>,
        F: FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
            -> Result<RelayDataStream<S>>,
    {
        RelayDeviceBuilder {
            setup: DeviceSetup(f),
            name: None,
            subs: Vec::new(),
            handlers: HashMap::new()
        }
    }
}

impl<F, S> RelayDeviceBuilder<F, S> 
where
    S: Stream<Item = RelayData, Error = Error> + 'static,
    F: FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
        -> Result<RelayDataStream<S>>,
{
    fn with_name(mut self, name: &str) -> RelayDeviceBuilder<F, S> {
        self.name = Some(name.to_owned());
        self
    }

    fn add_sub<M: RelayMessage>(mut self) -> RelayDeviceBuilder<F, S> {
        let sub = |dev: &RelayDevice, ctx: &mut Context<RelayDevice>| {
            dev.subscribe_async::<M>(ctx);
        };
        self.subs.push(Box::new(sub));
        let handler = |dev: &RelayDevice, bs: &[u8]| {
            if let Some(msg) = M::from_byte_slice(bs) {
                dev.issue_async(msg);
            } // TODO Log failed parse - should not be possible.
        };
        self.handlers.insert(M::tag(), Box::new(handler));
        self
    }

    fn finish(self, ctx: &mut Context<RelayDevice>) -> Result<RelayDevice> {
        let (tx, rx) = unbounded();
        let dev = RelayDevice {
            name: self.name,
            tx: tx,
            handlers: self.handlers,
        };
        let incoming = self.setup.0(&dev, rx, ctx)?;
        ctx.add_stream(incoming.0);
        for sub in self.subs {
            sub(&dev, ctx);
        }
        Ok(dev)
    }
}

impl Actor for RelayDevice {
    type Context = Context<Self>;
}

impl<M: RelayMessage> Handler<M> for RelayDevice {
    type Result = ();

    fn handle(&mut self, msg: M, _ctx: &mut Self::Context) {
        let relay_data = msg.into_relay_data();
        if let Err(_) = self.tx.unbounded_send(relay_data) {
            // TODO Send 'Teardown' to parent service
            unimplemented!();
        }
    }
}

impl StreamHandler<RelayData, Error> for RelayDevice {
    fn handle(&mut self, msg: RelayData, ctx: &mut Self::Context) {
        if let Some(tagged_data) = TaggedData::from_relay_data(msg) {
           if let Some(handler) = self.handlers.get(&tagged_data.tag) {
               handler(self, &tagged_data.data);
           }
        }
    }

    fn error(&mut self, err: Error, ctx: &mut Self::Context) -> Running {
        // TODO Send 'Teardown' to parent service
        unimplemented!();
        Running::Stop
    }
}

#[cfg(test)]
mod test {
    extern crate tokio;

    use super::*;
    use msgs::*;

    fn setup_udp(dev: &RelayDevice, rx: RelayRx, ctx: &mut Context<RelayDevice>) {
    }
    #[test]
    fn device_builder_works() {
        assert!(true);
    }
}
