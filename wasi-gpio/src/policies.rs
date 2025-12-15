#[derive(clap::Parser, Debug)]
pub struct Config {
    #[arg(short, long)]
    pub policy_file: String,

    #[arg(short, long)]
    pub component: String,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    DigitalInput,
    DigitalOutput,
    StatefulDigitalOutput,
    DigitalInputOutput,
    AnalogInput,
    AnalogOutput,
    AnalogInputOutput,
}

#[derive(serde::Deserialize, Debug)]
pub struct WasiGpioEntry {
    pub vlabel: String,
    pub modes: Vec<Mode>,
    pub plabel: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct Wasi {
    pub gpio: Vec<WasiGpioEntry>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Policies {
    pub wasi: Wasi,
}

impl Config {
    pub fn get_policies(&self) -> Policies {
        let entries =
            std::fs::read_to_string(&self.policy_file).expect("Failed to read policy file");

        match toml::from_str(&entries) {
            Ok(e) => e,
            Err(e) => panic!("{}", e.message()),
        }
    }

    pub fn get_component_path(&self) -> &str {
        &self.component
    }
}

impl Policies {
    #[allow(dead_code)]
    pub fn validate(&self) {
        for entry in self.wasi.gpio.iter() {
            for mode in entry.modes.iter() {
                match mode {
                    Mode::AnalogOutput => match entry.plabel.as_str() {
                        "PWM0" | "PWM1" | "PWM2" | "PWM3" => {}
                        _ => panic!("Invalid PWM channel: {}", entry.plabel),
                    },
                    Mode::AnalogInput | Mode::AnalogInputOutput => {
                        panic!("Analog input not supported on Raspberry Pi")
                    }
                    _ => {}
                }
            }
        }
    }

    fn find(&self, vlabel: &str) -> Option<&WasiGpioEntry> {
        for entry in self.wasi.gpio.iter() {
            if vlabel.eq(&entry.vlabel) {
                return Some(entry);
            }
        }

        None
    }

    pub fn get_plabel(&self, vlabel: &str) -> Option<String> {
        let plabel = match self.find(vlabel).map(|entry| entry.plabel.clone()) {
            Some(plabel) => plabel,
            None => return None,
        };

        if plabel.starts_with("GPIO") {
            return Some(plabel);
        }

        None
    }

    pub fn is_mode_allowed(&self, vlabel: &str, mode: Mode) -> bool {
        let entry = match self.find(vlabel) {
            Some(entry) => entry,
            None => return false,
        };

        for allowed_mode in &entry.modes {
            if mode == *allowed_mode {
                return true;
            }
        }

        false
    }
}
