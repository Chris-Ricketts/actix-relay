extern crate tokio;
#[macro_use] extern crate actix;
extern crate actix_broker;
extern crate actix_relay;
extern crate futures;
#[macro_use] extern crate serde_derive;
extern crate bytes;
extern crate simple_logger;
#[macro_use]
extern crate log;

use actix::prelude::*;
use actix::fut;
use actix_relay::{RelayData, RelayIOChild, RelayDevice};
use actix_broker::{BrokerSubscribe, BrokerIssue};
use futures::sync::mpsc::{UnboundedSender, unbounded};
use futures::{Sink, Stream};
use bytes::Bytes;
use tokio::net::{UdpSocket, UdpFramed};
use tokio::codec::BytesCodec;

use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::time::Duration;
use std::marker::PhantomData;
use std::thread;
use std::io;

trait UdpSetup
where 
    Self: Sized + Default
{
    fn setup(act: &mut Udp<Self>, ctx: &mut Context<Udp<Self>>) 
        -> io::Result<()>;
}

#[derive(Clone, Message, Serialize, Deserialize)] 
struct TestMessage(String);

#[derive(Default)]
struct SetupOne;

#[derive(Default)]
struct SetupTwo;

struct Emitter;

struct Receiver;

#[derive(Default)]
struct Udp<S> 
where 
    S: Default + UdpSetup + 'static
{
    tx: Option<UnboundedSender<Bytes>>,
    _p: PhantomData<S>,
}

impl<S> Actor for Udp<S> 
where 
    S: Default + UdpSetup + 'static
{
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        S::setup(self, ctx).unwrap();
        debug!("UDP Setup!");
    }
}

impl<S> Handler<RelayData> for Udp<S> 
where 
    S: Default + UdpSetup + 'static
{
    type Result = ();
    
    fn handle(&mut self, msg: RelayData, ctx: &mut Self::Context) {
        if let Some(ref tx) = self.tx {
            tx.unbounded_send(msg.0).unwrap();
        }
    }
}

impl<S> StreamHandler<RelayData, io::Error> for Udp<S> 
where 
    S: Default + UdpSetup + 'static
{
    fn handle(&mut self, msg: RelayData, ctx: &mut Self::Context) {
        self.forward_relay_data(msg)
    }
}


impl UdpSetup for SetupOne {
    fn setup(act: &mut Udp<Self>, ctx: &mut Context<Udp<Self>>) 
        -> io::Result<()>
    {
        setup_local_udp_actor(act, ctx, 8000, 9000) 
    }
}

impl UdpSetup for SetupTwo {
    fn setup(act: &mut Udp<Self>, ctx: &mut Context<Udp<Self>>) 
        -> io::Result<()>
    {
        setup_local_udp_actor(act, ctx, 9000, 8000) 
    }
}

fn setup_local_udp_actor<S: UdpSetup + Default>(
    act: &mut Udp<S>, 
    ctx: &mut Context<Udp<S>>,
    src_port: u16,
    dst_port: u16) -> io::Result<()> {
    let (tx, rx) = unbounded();
    act.tx = Some(tx);

    let localhost_ip = Ipv4Addr::new(127, 0, 0, 1);
    let src_addr = SocketAddr::new(IpAddr::V4(localhost_ip), src_port);
    let dst_addr = SocketAddr::new(IpAddr::V4(localhost_ip), dst_port);

    let sck = UdpSocket::bind(&src_addr)?;

    let (w, r) = UdpFramed::new(sck, BytesCodec::new()).split();

    let rx = rx
        .map(move |msg| {
            debug!("UDP Sending: {:?}", msg);
            (msg, dst_addr)
        })
        .map_err(|err| io::Error::new(io::ErrorKind::Other, "mpsc error"));

    let f = w.send_all(rx)
        .into_actor(act)
        .then(|_,_,_| fut::ok(()))
        .spawn(ctx);

    ctx.add_stream(r.map(|(b, _)| {
        debug!("UDP Got: {:?}", b);
        RelayData(b.freeze())
    }));

    Ok(())
}

impl Actor for Emitter {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_later(
            Duration::from_millis(100),
            |act, ctx| {
                act.issue_async(TestMessage("hello".to_string()));
                debug!("Test message sent");
                ctx.run_later(
                    Duration::from_millis(100),
                    |_,_| System::current().stop());
            });
    }
}

impl Actor for Receiver {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.subscribe_async::<TestMessage>(ctx);
    }
}

impl Handler<TestMessage> for Receiver {
    type Result = ();

    fn handle(&mut self, msg: TestMessage, _ctx: &mut Self::Context) {
        debug!("Test message received");
        assert_eq!(&msg.0, "hello");
        System::current().stop();
    }
}

#[test]
fn intersystem_relay() {
    simple_logger::init().unwrap();

    let s1 = thread::spawn(|| {
        System::run(|| {
            RelayDevice::<Udp<SetupOne>>::new()
                .add_sub::<TestMessage>()
                .finish();
            Receiver.start();
        })
    });

    let s2 = thread::spawn(|| {
        System::run(|| {
            RelayDevice::<Udp<SetupTwo>>::new()
                .add_sub::<TestMessage>()
                .finish();
            Emitter.start();
        })
    });

    s1.join();
    s2.join();
}
