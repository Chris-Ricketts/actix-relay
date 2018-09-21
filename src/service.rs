use actix::prelude::*;

use std::io;

use device::{RelayDevice, RelayDeviceBuilder};

#[derive(Default)]
struct RelayService {
    devs: Vec<Addr<RelayDevice>>,
}

#[derive(Default)]
struct RelayServiceBuilder {
    dev_builders: Vec<RelayDeviceBuilder>,
}

impl RelayService {
    fn new() -> RelayServiceBuilder {
        RelayServiceBuilder::default()
    }

    fn add_device(&mut self, dev: Addr<RelayDevice>) {
        self.devs.push(dev);
    }
}

impl RelayServiceBuilder {
    fn add_device(&mut self, dev_builder: RelayDeviceBuilder) -> Self {
        self.dev_builders.push(dev_builder);
        self
    }

    fn start(self) -> Result<(), ()> {
        for builder in self.dev_builders {
            RelayDevice::create(|ctx| {


