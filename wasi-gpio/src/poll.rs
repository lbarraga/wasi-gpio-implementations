use crate::ctx::WasiGpioView;
use crate::impls::GpioImpl;
use crate::util::Shared;
use crate::wasi::gpio::poll;
use wasmtime::component::Resource;

pub struct Pollable {
    pub trigger: Shared<bool>,
}

impl Pollable {
    pub fn new(trigger: Shared<bool>) -> Self {
        Pollable { trigger }
    }

    pub fn ready(&self) -> bool {
        *self.trigger.lock().unwrap()
    }
}

impl<'a, T: WasiGpioView> poll::Host for GpioImpl<'a, T> {
    fn poll(&mut self, _in_: Vec<Resource<Pollable>>) -> Vec<u32> {
        todo!("poll implementation")
    }
}

impl<'a, T: WasiGpioView> poll::HostPollable for GpioImpl<'a, T> {
    fn ready(&mut self, self_: Resource<Pollable>) -> bool {
        let poll = self.table().get(&self_).unwrap();
        poll.ready()
    }

    fn block(&mut self, self_: Resource<Pollable>) -> () {
        let poll = self.table().get(&self_).unwrap();
        while !poll.ready() {
            std::thread::yield_now();
        }
    }

    fn drop(&mut self, rep: Resource<Pollable>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}
