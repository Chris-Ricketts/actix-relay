use actix::prelude::*;
use actix_broker::{BrokerSubscribe, BrokerIssue};
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded};
use futures::Stream;

use std::collections::HashMap;
use std::io::{Error, Result};

use msgs::*;

type HandlerFn = Fn(&RelayDevice, &[u8]);
type SubscribeFn = Fn(&RelayDevice, &mut Context<RelayDevice>);
type SetupFn = FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
            -> Result<RelayDataStream>;
type RelayTx = UnboundedSender<RelayData>;
pub type RelayRx = UnboundedReceiver<RelayData>;
pub type RelayDataStream = Box<Stream<Item = RelayData, Error = Error>>;

struct DeviceSetup<F>(F);

pub struct RelayDevice {
    name: Option<String>,
    tx: Option<RelayTx>,
    setup: Box<SetupFn>,
    handlers: HashMap<u64, Box<HandlerFn>>,
}

pub struct RelayDeviceBuilder<F>
where
    F: FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
            -> Result<RelayDataStream>
{
    setup: Box<SetupFn>,
    name: Option<String>,
    subs: Vec<Box<SubscribeFn>>,
    handlers: HashMap<u64, Box<HandlerFn>>,
}

impl RelayDevice {
    fn new<F>(f: F) -> RelayDeviceBuilder<F>
    where
        F: FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
            -> Result<RelayDataStream>
    {
        RelayDeviceBuilder {
            setup: Box::new(f),
            name: None,
            subs: Vec::new(),
            handlers: HashMap::new()
        }
    }
}

impl<F> RelayDeviceBuilder<F> 
where
    F: FnOnce(&RelayDevice, RelayRx, &mut Context<RelayDevice>) 
            -> Result<RelayDataStream>
{
    fn with_name(mut self, name: &str) -> RelayDeviceBuilder<F> {
        self.name = Some(name.to_owned());
        self
    }

    fn add_sub<M: RelayMessage>(mut self) -> RelayDeviceBuilder<F> {
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
            tx: None,
            setup: self.setup,
            handlers: self.handlers,
        };
        for sub in self.subs {
            sub(&dev, ctx);
        }
        Ok(dev)
    }
}

impl Actor for RelayDevice {
    type Context = Context<Self>;

    fn started(&mut self, &mut Self::Context) {
        self.setup(self,
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

    use futures::Stream;
    use futures::stream::once;
    //use tokio::codec::BytesCodec;
    //use tokio::net::{UdpSocket, UdpFramed};
    use actix::prelude::*;
    use bytes::Bytes;
    //use std::net::{SocketAddr, IpAddr, Ipv4Addr};

    use super::*;
    use msgs::*;

    #[derive(Clone, Message, Serialize, Deserialize)]
    struct TestMessage;

    //fn setup_udp(dev: &RelayDevice, rx: RelayRx, ctx: &mut Context<RelayDevice>) 
    //    -> Result<RelayDataStream> 
    //{
    //    //let src = "127.0.0.1:8000".parse()?;
    //    //let dst = "127.0.0.1:9000".parse()?;
    //    //
    //    //let sck = UdpSocket::bind(&src)?;

    //    //let (w, r) = UdpFramed::new(sck, BytesCodec::new()).split();
    //    //
    //    //let send_all_fut = 
    //    //    w.send_all(rx.map(move |msg| (msg.0, dest_addr)))
    //    //        .map_err(|_| ())
    //    //        .map(|_| ())
    //    //        .spawn(ctx);

    //    Ok(r.map(|b| RelayData(b)))
    //}

    #[test]
    fn device_builder_works() {
        System::run(|| {
            let _ = RelayDevice::create(|ctx| {
                RelayDevice::new(|_,_,_| 
                                 Ok(Box::new(once::<RelayData, _>(Ok(RelayData(Bytes::new()))))))
                    .with_name("test")
                    .add_sub::<TestMessage>()
                    .finish(ctx)
                    .unwrap()
            });
            System::current().stop();
        });
    }
}
