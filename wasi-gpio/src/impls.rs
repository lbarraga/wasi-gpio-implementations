use crate::ctx::{WasiGpioCtx, WasiGpioView};
use wasmtime_wasi::ResourceTable;

pub struct GpioImpl<'a, T> {
    pub host: &'a mut T,
}

impl<'a, T: WasiGpioView> GpioImpl<'a, T> {
    pub fn ctx(&mut self) -> &mut WasiGpioCtx {
        self.host.gpio_ctx()
    }

    pub fn table(&mut self) -> &mut ResourceTable {
        self.host.table()
    }
}
