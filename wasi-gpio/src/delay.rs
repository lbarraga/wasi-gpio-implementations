use crate::ctx::WasiGpioView;
use crate::impls::GpioImpl;
use crate::wasi::gpio as bindings;

pub struct Delay {}

impl<T: WasiGpioView> bindings::delay::Host for GpioImpl<'_, T> {}

impl<T: WasiGpioView> bindings::delay::HostDelay for GpioImpl<'_, T> {
    fn delay_ns(
        &mut self,
        _self_: wasmtime::component::Resource<bindings::delay::Delay>,
        ns: u64,
    ) -> () {
        std::thread::sleep(std::time::Duration::from_nanos(ns));
    }

    fn delay_us(
        &mut self,
        _self_: wasmtime::component::Resource<bindings::delay::Delay>,
        us: u64,
    ) -> () {
        std::thread::sleep(std::time::Duration::from_micros(us));
    }

    fn delay_ms(
        &mut self,
        _self_: wasmtime::component::Resource<bindings::delay::Delay>,
        ms: u64,
    ) -> () {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<bindings::delay::Delay>,
    ) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}
