use actix::prelude::*;
use actix::fut;
use actix_broker::{BrokerSubscribe, BrokerIssue};

use std::collections::HashMap;

use msgs::*;

type HandlerFn<C> = Fn(&RelayDevice<C>, &[u8]);
type SubscribeFn<C> = Fn(&RelayDevice<C>, &mut Context<RelayDevice<C>>);

pub trait RelayIOChild: Default + Actor<Context = Context<Self>> + Handler<RelayData> 
{
    fn forward_relay_data(&self, rd: RelayData) {
        RelayDevice::<Self>::from_registry().do_send(rd)
    }
}

impl<A> RelayIOChild for A
where 
    A: Default + Actor<Context = Context<A>> + Handler<RelayData> 
{}

#[derive(Default)]
pub struct RelayDevice<C> 
where
    C: RelayIOChild
{
    name: Option<String>,
    io: Option<Addr<C>>,
    handlers: HashMap<u64, Box<HandlerFn<C>>>,
}

pub struct RelayDeviceBuilder<C>
where 
    C: RelayIOChild,
{
    child: C,
    name: Option<String>,
    subs: Vec<Box<SubscribeFn<C>>>,
    handlers: HashMap<u64, Box<HandlerFn<C>>>,
}

impl<C> RelayDevice<C> 
where 
    C: RelayIOChild,
{
    pub fn new() -> RelayDeviceBuilder<C>
    {
        RelayDeviceBuilder {
            child: C::default(),
            name: None,
            subs: Vec::new(),
            handlers: HashMap::new()
        }
    }
}

impl<C> RelayDeviceBuilder<C> 
where
    C: RelayIOChild,
{
    pub fn with_name(mut self, name: &str) -> RelayDeviceBuilder<C> {
        self.name = Some(name.to_owned());
        self
    }

    pub fn add_sub<M: RelayMessage>(mut self) -> RelayDeviceBuilder<C> {
        let sub = |dev: &RelayDevice<C>, ctx: &mut Context<RelayDevice<C>>| {
            dev.subscribe_async::<M>(ctx);
        };
        self.subs.push(Box::new(sub));
        let handler = |dev: &RelayDevice<C>, bs: &[u8]| {
            if let Some(msg) = M::from_byte_slice(bs) {
                dev.issue_async(msg);
            }
        };
        self.handlers.insert(M::tag(), Box::new(handler));
        self
    }

    pub fn finish(self) {
        let addr = RelayDevice::<C>::create(|ctx| {
            let dev = RelayDevice {
                name: self.name,
                io: Some(self.child.start()),
                handlers: self.handlers,
            };

            for sub in self.subs {
                sub(&dev, ctx);
            }

            dev
        });

        System::current().registry().set(addr);
    }
}

impl<C> Actor for RelayDevice<C> 
where 
    C: RelayIOChild,
{
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        trace!("RelayDevice - {} started",
               self.name.as_ref().unwrap_or(&"Unnamed".to_string()));
    }
}

impl<C> SystemService for RelayDevice<C> 
where 
    C: RelayIOChild,
{}

impl<C> Supervised for RelayDevice<C> 
where 
    C: RelayIOChild,
{}

impl<M, C> Handler<M> for RelayDevice<C> 
where
    M: RelayMessage,
    C: RelayIOChild,
{
    type Result = ();

    fn handle(&mut self, msg: M, ctx: &mut Self::Context) {
        let relay_data = msg.into_relay_data();
        if let Some(ref io) = self.io {
            io.send(relay_data)
                .into_actor(self)
                .map_err(|_,_, ctx| ctx.stop())
                .then(|_,_,_| fut::ok(()))
                .spawn(ctx);
        }
    }
}

impl<C> Handler<RelayData> for RelayDevice<C>
where 
    C: RelayIOChild,
{
    type Result = ();

    fn handle(&mut self, msg: RelayData, _ctx: &mut Self::Context) {
        if let Some(tagged_data) = TaggedData::from_relay_data(&msg) {
            if let Some(handler) = self.handlers.get(&tagged_data.tag) {
                handler(self, &tagged_data.data);
            }
        }
    }
} 

#[cfg(test)]
mod test {
    use actix::prelude::*;
    use actix_broker::BrokerIssue;

    use std::time::Duration;

    use msgs::RelayData;
    use super::*;

    #[derive(Clone, Message, Serialize, Deserialize)]
    struct TestRelayMessage;

    #[derive(Default)]
    struct DummyIOChild;

    impl Actor for DummyIOChild {
        type Context = Context<Self>;

        fn started(&mut self, _ctx: &mut Self::Context) {
            self.issue_async(TestRelayMessage);
        }
    }

    impl Handler<RelayData> for DummyIOChild {
        type Result = ();

        fn handle(&mut self, msg: RelayData, _ctx: &mut Self::Context) {
            assert_eq!(msg.0, TestRelayMessage.into_relay_data().0);
            System::current().stop();
        }
    }

    #[test]
    fn relay_device_works() {
        System::run(|| {
            let _ = RelayDevice::<DummyIOChild>::new()
                .with_name("test")
                .add_sub::<TestRelayMessage>()
                .finish();
        });
    }
}
