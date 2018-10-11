#[macro_use]
extern crate actix;
extern crate actix_broker;
extern crate bincode;
extern crate bytes;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

mod msgs;
mod device;

pub use msgs::{
    RelayMessage,
    RelayMessageID,
    RelayData,
};

pub use device::{
    RelayDevice,
    RelayIOChild,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
