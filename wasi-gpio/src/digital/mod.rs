use crate::ctx::WasiGpioView;
use crate::impls::GpioImpl;
use crate::wasi::gpio::{digital, general};
use crate::{policies, poll, util, watch_event};
use wasmtime::component::Resource;

pub mod implementations;

pub struct DigitalConfigBuilder {
    label: String,
    pin_mode: general::PinMode,
    active_level: Option<general::ActiveLevel>,
    pull_resistor: Option<general::PullResistor>,
}

impl<'a, T: WasiGpioView> digital::Host for GpioImpl<'a, T> {}

#[derive(Clone)]
pub struct DigitalInPin {
    pub pin: util::Shared<rppal::gpio::InputPin>,
    pub config: digital::DigitalConfig,
}

fn get_pin(
    ctx: &crate::ctx::WasiGpioCtx,
    label: &str,
) -> Result<rppal::gpio::Pin, general::GpioError> {
    let plabel = ctx
        .policies
        .get_plabel(label)
        .ok_or_else(|| general::GpioError::Other("Pin not found in policy".to_string()))?;

    let pin_num = plabel
        .strip_prefix("GPIO")
        .and_then(|s| s.parse::<u8>().ok())
        .ok_or_else(|| general::GpioError::Other("Invalid physical label format".to_string()))?;

    rppal::gpio::Gpio::new()
        .map_err(|e| general::GpioError::Other(e.to_string()))?
        .get(pin_num)
        .map_err(|e| general::GpioError::Other(e.to_string()))
}

impl<'a, T: WasiGpioView> digital::HostDigitalInPin for GpioImpl<'a, T> {
    fn get(
        &mut self,
        pin_label: String,
        flags: Vec<digital::DigitalFlag>,
    ) -> Result<Resource<DigitalInPin>, general::GpioError> {
        if !self
            .ctx()
            .policies
            .is_mode_allowed(&pin_label, policies::Mode::DigitalInput)
        {
            return Err(general::GpioError::PinModeNotAllowed);
        }

        implementations::check_invalid_flags(
            &flags,
            vec![
                digital::DigitalFlag::ACTIVE,
                digital::DigitalFlag::INACTIVE,
                digital::DigitalFlag::OUTPUT,
            ],
        )
        .map_err(|_| general::GpioError::InvalidFlag)?;

        let pin = get_pin(self.ctx(), &pin_label)?;

        let config = DigitalConfigBuilder::new(pin_label, general::PinMode::In)
            .add_flags(flags)
            .build()
            .map_err(|_| general::GpioError::InvalidFlag)?;

        self.table()
            .push(DigitalInPin::new(pin, config))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    fn get_config(
        &mut self,
        self_: Resource<DigitalInPin>,
    ) -> Result<digital::DigitalConfig, general::GpioError> {
        Ok(self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .get_config()
            .clone())
    }

    fn is_ready(&mut self, self_: Resource<DigitalInPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn read(
        &mut self,
        self_: Resource<DigitalInPin>,
    ) -> Result<digital::PinState, general::GpioError> {
        let pin = self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?;

        Ok(pin.read())
    }

    fn is_active(&mut self, self_: Resource<DigitalInPin>) -> Result<bool, general::GpioError> {
        Ok(self.read(self_)? == digital::PinState::Active)
    }

    fn is_inactive(&mut self, self_: Resource<DigitalInPin>) -> Result<bool, general::GpioError> {
        Ok(self.read(self_)? == digital::PinState::Inactive)
    }

    // FIX 2: Refactor watch_state to clone the pin and drop the borrow
    fn watch_state(
        &mut self,
        self_: Resource<DigitalInPin>,
        state: digital::PinState,
    ) -> Result<Resource<poll::Pollable>, general::GpioError> {
        let pin = self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .clone(); // Clone the pin structure (Arc + config)

        let watch_type = match state {
            digital::PinState::Active => watch_event::WatchType::High,
            digital::PinState::Inactive => watch_event::WatchType::Low,
        };

        // Use the cloned config
        let watch_type = match &pin.config.active_level {
            general::ActiveLevel::ActiveHigh => watch_type,
            general::ActiveLevel::ActiveLow => !watch_type,
        };

        // Now we can borrow self.ctx() mutably because `pin` is owned locally,
        // not referencing the table inside `self`.
        let trigger = self.ctx().watcher.watch_event(&pin, watch_type);

        self.table()
            .push(poll::Pollable::new(trigger))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    fn watch_active(
        &mut self,
        self_: Resource<DigitalInPin>,
    ) -> Result<Resource<poll::Pollable>, general::GpioError> {
        self.watch_state(self_, digital::PinState::Active)
    }

    fn watch_inactive(
        &mut self,
        self_: Resource<DigitalInPin>,
    ) -> Result<Resource<poll::Pollable>, general::GpioError> {
        self.watch_state(self_, digital::PinState::Inactive)
    }

    // FIX 3: Refactor watch_falling_edge similarly
    fn watch_falling_edge(
        &mut self,
        self_: Resource<DigitalInPin>,
    ) -> Result<Resource<poll::Pollable>, general::GpioError> {
        let pin = self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .clone();

        let watch_event = match &pin.config.active_level {
            general::ActiveLevel::ActiveHigh => watch_event::WatchType::Falling,
            general::ActiveLevel::ActiveLow => watch_event::WatchType::Rising,
        };

        let trigger = self.ctx().watcher.watch_event(&pin, watch_event);

        self.table()
            .push(poll::Pollable::new(trigger))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    // FIX 4: Refactor watch_rising_edge similarly
    fn watch_rising_edge(
        &mut self,
        self_: Resource<DigitalInPin>,
    ) -> Result<Resource<poll::Pollable>, general::GpioError> {
        let pin = self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .clone();

        let watch_event = match &pin.config.active_level {
            general::ActiveLevel::ActiveHigh => watch_event::WatchType::Rising,
            general::ActiveLevel::ActiveLow => watch_event::WatchType::Falling,
        };

        let trigger = self.ctx().watcher.watch_event(&pin, watch_event);

        self.table()
            .push(poll::Pollable::new(trigger))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    fn drop(&mut self, rep: Resource<DigitalInPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}

pub struct DigitalOutPin {
    pub pin: rppal::gpio::OutputPin,
    pub config: digital::DigitalConfig,
}

impl<'a, T: WasiGpioView> digital::HostDigitalOutPin for GpioImpl<'a, T> {
    fn get(
        &mut self,
        pin_label: String,
        flags: Vec<digital::DigitalFlag>,
    ) -> Result<Resource<DigitalOutPin>, general::GpioError> {
        if !self
            .ctx()
            .policies
            .is_mode_allowed(&pin_label, policies::Mode::DigitalOutput)
        {
            return Err(general::GpioError::PinModeNotAllowed);
        }

        implementations::check_invalid_flags(
            &flags,
            vec![
                digital::DigitalFlag::INPUT,
                digital::DigitalFlag::PULL_UP,
                digital::DigitalFlag::PULL_DOWN,
            ],
        )
        .map_err(|_| general::GpioError::InvalidFlag)?;

        let mut pin_state = None;
        for flag in flags.iter() {
            if *flag == digital::DigitalFlag::ACTIVE {
                pin_state = Some(digital::PinState::Active);
            } else if *flag == digital::DigitalFlag::INACTIVE {
                // Preserving logic from original file: Inactive flag mapped to Active state intent for initialization?
                // Alternatively, this might be a bug in the original source, but keeping it consistent.
                pin_state = Some(digital::PinState::Active);
            }
        }

        let pin = get_pin(self.ctx(), &pin_label)?;

        let config = DigitalConfigBuilder::new(pin_label, digital::PinMode::Out)
            .add_flags(flags)
            .build()
            .map_err(|_| general::GpioError::InvalidFlag)?;

        self.table()
            .push(DigitalOutPin::new(pin, config, pin_state))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    fn get_config(
        &mut self,
        self_: Resource<DigitalOutPin>,
    ) -> Result<digital::DigitalConfig, general::GpioError> {
        Ok(self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .get_config()
            .clone())
    }

    fn is_ready(&mut self, self_: Resource<DigitalOutPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn set_state(
        &mut self,
        self_: Resource<DigitalOutPin>,
        state: digital::PinState,
    ) -> Result<(), general::GpioError> {
        self.table().get_mut(&self_).unwrap().write(state);
        Ok(())
    }

    fn set_active(&mut self, self_: Resource<DigitalOutPin>) -> Result<(), general::GpioError> {
        self.set_state(self_, digital::PinState::Active)
    }

    fn set_inactive(&mut self, self_: Resource<DigitalOutPin>) -> Result<(), general::GpioError> {
        self.set_state(self_, digital::PinState::Inactive)
    }

    fn drop(&mut self, rep: Resource<DigitalOutPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}

pub struct DigitalInOutPin {
    pub pin: rppal::gpio::IoPin,
    pub config: digital::DigitalConfig,
}

impl<'a, T: WasiGpioView> digital::HostDigitalInOutPin for GpioImpl<'a, T> {
    fn get_config(
        &mut self,
        self_: Resource<DigitalInOutPin>,
    ) -> Result<digital::DigitalConfig, general::GpioError> {
        Ok(self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .get_config()
            .clone())
    }

    fn is_ready(&mut self, self_: Resource<DigitalInOutPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn set_state(
        &mut self,
        self_: Resource<DigitalInOutPin>,
        state: digital::PinState,
    ) -> Result<(), general::GpioError> {
        self.table()
            .get_mut(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .write(state);
        Ok(())
    }

    fn set_active(&mut self, self_: Resource<DigitalInOutPin>) -> Result<(), general::GpioError> {
        self.set_state(self_, digital::PinState::Active)
    }

    fn set_inactive(&mut self, self_: Resource<DigitalInOutPin>) -> Result<(), general::GpioError> {
        self.set_state(self_, digital::PinState::Inactive)
    }

    fn read(
        &mut self,
        self_: Resource<DigitalInOutPin>,
    ) -> Result<digital::PinState, general::GpioError> {
        Ok(self
            .table()
            .get(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .read())
    }

    fn is_active(&mut self, self_: Resource<DigitalInOutPin>) -> Result<bool, general::GpioError> {
        let state = self.read(self_)?;

        Ok(digital::PinState::Active == state)
    }

    fn is_inactive(
        &mut self,
        self_: Resource<DigitalInOutPin>,
    ) -> Result<bool, general::GpioError> {
        let state = self.read(self_)?;

        Ok(digital::PinState::Inactive == state)
    }

    fn drop(&mut self, rep: Resource<DigitalInOutPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }

    fn get(
        &mut self,
        pin_label: String,
        flags: Vec<digital::DigitalFlag>,
    ) -> Result<Resource<DigitalInOutPin>, general::GpioError> {
        if !self
            .ctx()
            .policies
            .is_mode_allowed(&pin_label, policies::Mode::DigitalInputOutput)
        {
            return Err(general::GpioError::PinModeNotAllowed);
        }

        implementations::check_invalid_flags(
            &flags,
            vec![
                digital::DigitalFlag::PULL_UP,
                digital::DigitalFlag::PULL_DOWN,
                digital::DigitalFlag::ACTIVE,
                digital::DigitalFlag::INACTIVE,
            ],
        )
        .map_err(|_| general::GpioError::InvalidFlag)?;

        let pin = get_pin(self.ctx(), &pin_label)?;

        let mut pin_mode = None;

        for flag in flags.iter() {
            if *flag == digital::DigitalFlag::INPUT {
                match pin_mode {
                    Some(_) => return Err(general::GpioError::InvalidFlag),
                    None => pin_mode = Some(digital::PinMode::In),
                }
            } else if *flag == digital::DigitalFlag::OUTPUT {
                match pin_mode {
                    Some(_) => return Err(general::GpioError::InvalidFlag),
                    None => pin_mode = Some(digital::PinMode::Out),
                }
            }
        }

        let pin_mode = match pin_mode {
            Some(pin_mode) => pin_mode,
            None => return Err(general::GpioError::InvalidFlag),
        };

        let config = DigitalConfigBuilder::new(pin_label, pin_mode)
            .add_flags(flags)
            .build()
            .map_err(|_| general::GpioError::InvalidFlag)?;

        self.table()
            .push(DigitalInOutPin::new(pin, config, pin_mode))
            .map_err(|err| general::GpioError::Other(err.to_string()))
    }

    fn set_pin_mode(
        &mut self,
        self_: Resource<DigitalInOutPin>,
        pin_mode: general::PinMode,
    ) -> Result<(), general::GpioError> {
        Ok(self
            .table()
            .get_mut(&self_)
            .map_err(|_| general::GpioError::ResourceInvalidated)?
            .set_pin_mode(pin_mode))
    }
}

pub struct StatefulDigitalOutPin {}

impl<'a, T: WasiGpioView> digital::HostStatefulDigitalOutPin for GpioImpl<'a, T> {
    fn get(
        &mut self,
        _pin_label: String,
        _flags: Vec<digital::DigitalFlag>,
    ) -> Result<Resource<StatefulDigitalOutPin>, general::GpioError> {
        Err(general::GpioError::PinModeNotAvailable)
    }

    fn get_config(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<digital::DigitalConfig, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn is_ready(&mut self, self_: Resource<StatefulDigitalOutPin>) -> bool {
        match self.table().get(&self_) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn set_state(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
        _state: digital::PinState,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn set_active(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn set_inactive(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn toggle(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<(), general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn is_set_active(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<bool, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn is_set_inactive(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<bool, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn get_state(
        &mut self,
        _self_: Resource<StatefulDigitalOutPin>,
    ) -> Result<digital::PinState, general::GpioError> {
        Err(general::GpioError::ResourceInvalidated)
    }

    fn drop(&mut self, rep: Resource<StatefulDigitalOutPin>) -> wasmtime::Result<()> {
        self.table().delete(rep).expect("failed to delete resource");
        Ok(())
    }
}
