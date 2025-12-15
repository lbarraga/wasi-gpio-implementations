use super::util::{Shared, SharedExt};
use crate::digital::DigitalInPin;
use std::fmt::Debug;

pub struct Watcher {
    to_watch: Shared<std::collections::HashMap<WatchEventKey, WatchEventValue>>,
}

impl Watcher {
    pub fn new() -> Self {
        Self {
            to_watch: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn watch_event(&mut self, pin: &DigitalInPin, watch_type: WatchType) -> Shared<bool> {
        let key = WatchEventKey {
            watch_type,
            pin_label: pin.get_config().label.clone(),
        };

        let map_clone = self.to_watch.clone();
        let map = &mut *self.to_watch.lock().unwrap();

        let trigger = match map.get_mut(&key) {
            // There is already an event watching this event_type
            Some(value) => value.trigger.clone(),

            // Make a new watch_event
            None => {
                //println!("No trigger yet");
                let trigger = Shared::make_shared(false);

                // Food for thread
                let trigger_clone = trigger.clone();
                let pin_clone = pin.clone_pin();
                let key_clone = key.clone();
                let watch_type_clone = key.watch_type.clone();

                let thread = std::thread::spawn(move || {
                    match watch_type_clone {
                        WatchType::High => watch_high(pin_clone, trigger_clone),
                        WatchType::Low => watch_low(pin_clone, trigger_clone),
                        WatchType::Rising => watch_rising(pin_clone, trigger_clone),
                        WatchType::Falling => watch_falling(pin_clone, trigger_clone),
                    };

                    (*map_clone.lock().unwrap()).remove(&key_clone);
                });

                let value = WatchEventValue {
                    trigger: trigger.clone(),
                    thread,
                };

                // ADD TO MAP, WILL ALWAYS HAPPEN FIRST BECAUSE WE HAVE A LOCK ON THE MAP
                map.insert(key, value);

                trigger
            }
        };

        trigger
    }
}

#[derive(Clone)]
pub struct WatchEventKey {
    watch_type: WatchType,
    pin_label: String,
}

pub struct WatchEventValue {
    trigger: Shared<bool>,
    thread: std::thread::JoinHandle<()>,
}

fn watch_high(pin: Shared<rppal::gpio::InputPin>, trigger: Shared<bool>) {
    while (*pin.lock().unwrap()).is_low() {}

    //println!("Thread: triggered");

    *trigger.lock().unwrap() = true
}

fn watch_low(pin: Shared<rppal::gpio::InputPin>, trigger: Shared<bool>) {
    while (*pin.lock().unwrap()).is_high() {}

    *trigger.lock().unwrap() = true
}

fn watch_rising(pin: Shared<rppal::gpio::InputPin>, trigger: Shared<bool>) {
    // Pin is high so needs to go low first before rising edge can happen
    while (*pin.lock().unwrap()).is_high() {}
    // Pin is low, now check for high event
    while (*pin.lock().unwrap()).is_low() {}

    *trigger.lock().unwrap() = true
}

fn watch_falling(pin: Shared<rppal::gpio::InputPin>, trigger: Shared<bool>) {
    // Pin is low so needs to go high first before falling edge can happen
    while (*pin.lock().unwrap()).is_low() {}
    // Pin is high, now check for low event
    while (*pin.lock().unwrap()).is_high() {}

    *trigger.lock().unwrap() = true
}

impl Eq for WatchEventKey {}

impl PartialEq for WatchEventKey {
    fn eq(&self, other: &Self) -> bool {
        self.watch_type == other.watch_type && self.pin_label == other.pin_label
    }
}

impl std::hash::Hash for WatchEventKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.watch_type.hash(state);
        self.pin_label.hash(state);
    }
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum WatchType {
    High,
    Low,
    Rising,
    Falling,
}

impl Debug for WatchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "High"),
            Self::Low => write!(f, "Low"),
            Self::Rising => write!(f, "Rising"),
            Self::Falling => write!(f, "Falling"),
        }
    }
}

impl std::ops::Not for WatchType {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            WatchType::High => Self::Low,
            WatchType::Low => Self::High,
            WatchType::Rising => Self::Falling,
            WatchType::Falling => Self::Rising,
        }
    }
}
