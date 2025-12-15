use crate::ctx::WasiGpioView;
use crate::impls::GpioImpl;
use crate::wasi::gpio::general;

impl<'a, T: WasiGpioView> general::Host for GpioImpl<'a, T> {}
