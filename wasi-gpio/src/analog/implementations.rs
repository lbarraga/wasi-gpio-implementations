use super::{AnalogConfigBuilder, AnalogOutPin};
use crate::wasi::gpio::{analog, general};

impl AnalogConfigBuilder {
    pub fn new(label: String, pin_mode: general::PinMode) -> Self {
        Self {
            label,
            pin_mode,
            output_mode: None,
        }
    }

    pub fn add_flags(mut self, flags: Vec<analog::AnalogFlag>) -> Self {
        for flag in flags {
            if flag == analog::AnalogFlag::PWM {
                self.output_mode = Some(analog::OutputMode::Pwm)
            }
        }

        self
    }

    pub fn build(self) -> Result<analog::AnalogConfig, general::GpioError> {
        match self.pin_mode {
            general::PinMode::Out => {
                if self.output_mode.is_none() {
                    return Err(general::GpioError::InvalidFlag);
                }
            }
            general::PinMode::In => return Err(general::GpioError::PinModeNotAvailable),
        }

        Ok(analog::AnalogConfig {
            label: self.label,
            pin_mode: self.pin_mode,
            output_mode: self.output_mode,
        })
    }
}

impl AnalogOutPin {
    pub fn new(pin: rppal::gpio::OutputPin, config: analog::AnalogConfig) -> Self {
        Self { pin, config }
    }

    pub fn get_config(&self) -> analog::AnalogConfig {
        self.config.clone()
    }

    pub fn set_value(&mut self, value: f32) -> Result<(), String> {
        self.pin
            .set_pwm_frequency(1000., value as f64)
            .map_err(|err| err.to_string())
    }
}

pub fn check_invalid_flags(
    flags: &Vec<analog::AnalogFlag>,
    disallowed_flags: Vec<analog::AnalogFlag>,
) -> Result<(), ()> {
    for flag in flags {
        if disallowed_flags.contains(flag) {
            return Err(());
        }
    }

    Ok(())
}
