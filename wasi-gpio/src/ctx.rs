use std::marker::PhantomData;
use wasmtime::component::HasData;
use wasmtime_wasi::{ResourceTable, WasiView};

use crate::impls::GpioImpl;
use crate::policies::Policies;
use crate::watch_event::Watcher;

pub struct WasiGpioCtx {
    pub policies: Policies,
    pub watcher: Watcher,
}

impl WasiGpioCtx {
    pub fn new(policies: Policies) -> Self {
        Self {
            policies,
            watcher: Watcher::new(),
        }
    }
}

pub trait WasiGpioView: WasiView {
    fn gpio_ctx(&mut self) -> &mut WasiGpioCtx;
    fn table(&mut self) -> &mut ResourceTable;
}

pub struct GpioBindingMarker<T>(PhantomData<T>);

impl<T: WasiGpioView + 'static> HasData for GpioBindingMarker<T> {
    type Data<'a> = GpioImpl<'a, T>;
}
