use super::{DigitalConfigBuilder, DigitalInOutPin, DigitalInPin, DigitalOutPin};
use crate::util::Shared;
use crate::wasi::gpio::{digital, general};

pub fn check_invalid_flags(
    flags: &Vec<digital::DigitalFlag>,
    disallowed_flags: Vec<digital::DigitalFlag>,
) -> Result<(), ()> {
    for flag in flags {
        if disallowed_flags.contains(flag) {
            return Err(());
        }
    }

    Ok(())
}

impl DigitalOutPin {
    pub fn new(
        pin: rppal::gpio::Pin,
        config: digital::DigitalConfig,
        pin_state: Option<digital::PinState>,
    ) -> Self {
        let pin = if let None = pin_state {
            pin.into_output()
        } else if let Some(digital::PinState::Inactive) = pin_state {
            match &config.active_level {
                general::ActiveLevel::ActiveHigh => pin.into_output_low(),
                general::ActiveLevel::ActiveLow => pin.into_output_high(),
            }
        } else {
            match &config.active_level {
                general::ActiveLevel::ActiveHigh => pin.into_output_high(),
                general::ActiveLevel::ActiveLow => pin.into_output_low(),
            }
        };

        Self { pin, config }
    }

    pub fn get_config(&self) -> &digital::DigitalConfig {
        &self.config
    }

    pub fn write(&mut self, pin_state: digital::PinState) {
        let pin_state = match &self.config.active_level {
            general::ActiveLevel::ActiveHigh => pin_state,
            general::ActiveLevel::ActiveLow => !pin_state,
        };

        let pin_state: rppal::gpio::Level = pin_state.into();

        self.pin.write(pin_state);
    }
}

impl From<digital::PinState> for rppal::gpio::Level {
    fn from(value: digital::PinState) -> Self {
        match value {
            digital::PinState::Active => Self::High,
            digital::PinState::Inactive => Self::Low,
        }
    }
}

impl DigitalInPin {
    pub fn new(pin: rppal::gpio::Pin, config: digital::DigitalConfig) -> Self {
        let pin = match &config.pull_resistor {
            Some(general::PullResistor::PullDown) => pin.into_input_pulldown(),
            Some(general::PullResistor::PullUp) => pin.into_input_pullup(),
            None => pin.into_input(),
        };

        Self {
            pin: std::sync::Arc::new(std::sync::Mutex::new(pin)),
            config,
        }
    }

    pub fn get_config(&self) -> &digital::DigitalConfig {
        &self.config
    }

    pub fn read(&self) -> digital::PinState {
        let pin_state = match (*self.pin.lock().unwrap()).read() {
            rppal::gpio::Level::Low => digital::PinState::Inactive,
            rppal::gpio::Level::High => digital::PinState::Active,
        };

        match self.config.active_level {
            general::ActiveLevel::ActiveHigh => pin_state,
            general::ActiveLevel::ActiveLow => !pin_state,
        }
    }

    pub fn clone_pin(&self) -> Shared<rppal::gpio::InputPin> {
        self.pin.clone()
    }
}

impl DigitalInOutPin {
    pub fn new(
        pin: rppal::gpio::Pin,
        config: digital::DigitalConfig,
        pin_mode: digital::PinMode,
    ) -> Self {
        match pin_mode {
            general::PinMode::In => Self {
                pin: pin.into_io(rppal::gpio::Mode::Input),
                config,
            },
            general::PinMode::Out => Self {
                pin: pin.into_io(rppal::gpio::Mode::Output),
                config,
            },
        }
    }

    pub fn get_config(&self) -> &digital::DigitalConfig {
        &self.config
    }

    pub fn write(&mut self, pin_state: digital::PinState) {
        let pin_state = match &self.config.active_level {
            general::ActiveLevel::ActiveHigh => pin_state,
            general::ActiveLevel::ActiveLow => !pin_state,
        };

        let pin_state: rppal::gpio::Level = pin_state.into();

        self.pin.write(pin_state);
    }

    pub fn read(&self) -> digital::PinState {
        let pin_state = match self.pin.read() {
            rppal::gpio::Level::Low => digital::PinState::Inactive,
            rppal::gpio::Level::High => digital::PinState::Active,
        };

        match self.config.active_level {
            general::ActiveLevel::ActiveHigh => pin_state,
            general::ActiveLevel::ActiveLow => !pin_state,
        }
    }

    pub fn set_pin_mode(&mut self, mode: general::PinMode) {
        self.pin.set_mode(mode.into());
    }
}

impl From<rppal::gpio::Level> for digital::PinState {
    fn from(value: rppal::gpio::Level) -> Self {
        match value {
            rppal::gpio::Level::Low => Self::Inactive,
            rppal::gpio::Level::High => Self::Active,
        }
    }
}

impl std::ops::Not for digital::PinState {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            digital::PinState::Active => Self::Inactive,
            digital::PinState::Inactive => Self::Active,
        }
    }
}

impl From<general::PinMode> for rppal::gpio::Mode {
    fn from(value: general::PinMode) -> Self {
        match value {
            general::PinMode::In => Self::Input,
            general::PinMode::Out => Self::Output,
        }
    }
}

impl DigitalConfigBuilder {
    pub fn new(label: String, pin_mode: general::PinMode) -> Self {
        Self {
            label,
            pin_mode,
            active_level: None,
            pull_resistor: None,
        }
    }

    fn add_active_level(&mut self, active_level: general::ActiveLevel) {
        self.active_level = Some(active_level);
    }

    fn add_pull_resistor(&mut self, pull_resistor: general::PullResistor) {
        self.pull_resistor = Some(pull_resistor)
    }

    pub fn add_flags(mut self, flags: Vec<digital::DigitalFlag>) -> Self {
        for flag in flags {
            if flag == digital::DigitalFlag::ACTIVE_HIGH {
                self.add_active_level(general::ActiveLevel::ActiveHigh);
            } else if flag == digital::DigitalFlag::ACTIVE_LOW {
                self.add_active_level(general::ActiveLevel::ActiveLow);
            } else if flag == digital::DigitalFlag::PULL_UP {
                self.add_pull_resistor(general::PullResistor::PullUp);
            } else if flag == digital::DigitalFlag::PULL_DOWN {
                self.add_pull_resistor(general::PullResistor::PullDown);
            }
        }

        self
    }

    pub fn build(self) -> Result<digital::DigitalConfig, general::GpioError> {
        let active_level = match self.active_level {
            Some(active_level) => active_level,
            None => return Err(general::GpioError::InvalidFlag),
        };

        match self.pin_mode {
            general::PinMode::In => {}
            general::PinMode::Out => {
                if self.pull_resistor.is_some() {
                    return Err(general::GpioError::InvalidFlag);
                }
            }
        }

        Ok(digital::DigitalConfig {
            label: self.label.clone(),
            pin_mode: self.pin_mode,
            active_level,
            pull_resistor: self.pull_resistor,
        })
    }
}
