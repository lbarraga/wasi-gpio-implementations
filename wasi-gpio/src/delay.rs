use crate::ctx::WasiGpioView;
use crate::impls::GpioImpl;
use crate::wasi::gpio as bindings;

pub struct Delay {}

// Implement the top-level Host trait directly
impl<T: WasiGpioView> bindings::delay::Host for GpioImpl<'_, T> {
    fn delay_ns(&mut self, ns: u64) -> () {
        std::thread::sleep(std::time::Duration::from_nanos(ns));
    }

    fn delay_us(&mut self, us: u64) -> () {
        std::thread::sleep(std::time::Duration::from_micros(us));
    }

    fn delay_ms(&mut self, ms: u64) -> () {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

// Remove the HostDelay implementation since the resource is gone
