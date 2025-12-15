pub mod implementations;
use crate::ctx::WasiGpioView;
use crate::impls::GpioImpl;
use crate::policies;
use crate::poll::Pollable;
use crate::wasi::gpio::{analog, general};
use wasmtime::component::Resource;

macro_rules! PWM_MAX {
    () => {
        (1 << 12) - 1
    };
}

pub struct AnalogConfigBuilder {
    label: String,
    pin_mode: general::PinMode,
    output_mode: Option<analog::OutputMode>,
}

pub struct AnalogInPin {}

pub struct AnalogInOutPin {}

pub struct AnalogOutPin {
    pub pin: rppal::gpio::OutputPin,
    pub config: analog::AnalogConfig,
}

// Helper function to resolve pins based on policies
fn get_pin_output(
    ctx: &crate::ctx::WasiGpioCtx,
    label: &str,
) -> Result<rppal::gpio::OutputPin, general::GpioError> {
    let plabel = ctx
        .policies
        .get_plabel(label)
        .ok_or_else(|| general::GpioError::Other("Pin not found in policy".to_string()))?;

    // Parse "GPIO<num>"
    let pin_num = plabel
        .strip_prefix("GPIO")
        .and_then(|s| s.parse::<u8>().ok())
        .ok_or_else(|| general::GpioError::Other("Invalid physical label format".to_string()))?;

    let gpio = rppal::gpio::Gpio::new().map_err(|e| general::GpioError::Other(e.to_string()))?;

    let pin = gpio
        .get(pin_num)
        .map_err(|e| general::GpioError::Other(e.to_string()))?;

    Ok(pin.into_output_low())
}

impl<'a, T: WasiGpioView> analog::Host for GpioImpl<'a, T> {}

impl<'a, T: WasiGpioView> analog::HostAnalogOutPin for GpioImpl<'a, T> {
    fn get(
        &mut self,
        pin_label: String,
        flags: Vec<analog::AnalogFlag>,
    ) -> Result<Resource<AnalogOutPin>, general::GpioError> {
        if !self
            .ctx()
            .policies
            .is_mode_allowed(&pin_label, policies::Mode::AnalogOutput)
        {
            return Err(general::GpioError::PinModeNotAllowed);
        }

        implementations::check_invalid_flags(&flags, vec![analog::AnalogFlag::DAC])
            .map_err(|_| general::GpioError::InvalidFlag)?;

        let pin = get_pin_output(self.ctx(), &pin_label)?;

        let config = AnalogConfigBuilder::new(pin_label, general::PinMode::Out)
            .add_flags(flags)
            .build()
            .map_err(|_| general::GpioError::InvalidFlag)?;

        self.table()
            .push(AnalogOutPin::new(pin, config))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    fn get_config(
        &mut self,
        self_: Resource<AnalogOutPin>,
    ) -> Result<analog::AnalogConfig, general::GpioError> {
        Ok(self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .get_config()
            .clone())
    }

    fn is_ready(&mut self, self_: Resource<AnalogOutPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn set_value_raw(
        &mut self,
        self_: Resource<AnalogOutPin>,
        mut value: u32,
    ) -> Result<(), general::GpioError> {
        if value > PWM_MAX!() {
            value = PWM_MAX!();
        }

        self.set_value(self_, value as f32 / (PWM_MAX!() as f32))
    }

    fn set_value(
        &mut self,
        self_: Resource<AnalogOutPin>,
        value: f32,
    ) -> Result<(), general::GpioError> {
        let pin = self
            .table()
            .get_mut(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?;
        Ok(pin
            .set_value(value)
            .map_err(|err| general::GpioError::Other(err))?)
    }

    fn drop(&mut self, rep: Resource<AnalogOutPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}

impl<'a, T: WasiGpioView> analog::HostAnalogInOutPin for GpioImpl<'a, T> {
    fn get(
        &mut self,
        _pin_label: String,
        _flags: Vec<analog::AnalogFlag>,
    ) -> Result<Resource<AnalogInOutPin>, general::GpioError> {
        Err(general::GpioError::PinModeNotAvailable)
    }

    fn get_config(
        &mut self,
        _self_: Resource<AnalogInOutPin>,
    ) -> Result<analog::AnalogConfig, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn is_ready(&mut self, self_: Resource<AnalogInOutPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn set_value_raw(
        &mut self,
        _self_: Resource<AnalogInOutPin>,
        _value: u32,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn set_value(
        &mut self,
        _self_: Resource<AnalogInOutPin>,
        _value: f32,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn read_raw(&mut self, _self_: Resource<AnalogInOutPin>) -> Result<u32, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn read(&mut self, _self_: Resource<AnalogInOutPin>) -> Result<f32, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn drop(&mut self, rep: Resource<AnalogInOutPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }

    fn set_pin_mode(
        &mut self,
        _self_: Resource<AnalogInOutPin>,
        _pin_mode: general::PinMode,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }
}

impl<'a, T: WasiGpioView> analog::HostAnalogInPin for GpioImpl<'a, T> {
    fn get(
        &mut self,
        _pin_label: String,
        _flags: Vec<analog::AnalogFlag>,
    ) -> Result<Resource<AnalogInPin>, general::GpioError> {
        Err(general::GpioError::PinModeNotAvailable)
    }

    fn get_config(
        &mut self,
        _self_: Resource<AnalogInPin>,
    ) -> Result<analog::AnalogConfig, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn is_ready(&mut self, self_: Resource<AnalogInPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn read_raw(&mut self, _self_: Resource<AnalogInPin>) -> Result<u32, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn read(&mut self, _self_: Resource<AnalogInPin>) -> Result<f32, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn watch_above_raw(
        &mut self,
        _self_: Resource<AnalogInPin>,
        _value: u32,
    ) -> Result<Resource<Pollable>, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn watch_above(
        &mut self,
        _self_: Resource<AnalogInPin>,
        _value: f32,
    ) -> Result<Resource<Pollable>, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn watch_below_raw(
        &mut self,
        _self_: Resource<AnalogInPin>,
        _value: u32,
    ) -> Result<Resource<Pollable>, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn watch_below(
        &mut self,
        _self_: Resource<AnalogInPin>,
        _value: f32,
    ) -> Result<Resource<Pollable>, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn drop(&mut self, rep: Resource<AnalogInPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}
