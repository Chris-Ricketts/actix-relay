#[macro_use]
extern crate actix;
extern crate actix_broker;
extern crate bincode;
extern crate bytes;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod msgs;
mod device;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
