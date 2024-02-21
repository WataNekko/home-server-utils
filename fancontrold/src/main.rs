use std::{process::Command, time::Duration};

use confy::ConfyError;
use rppal::gpio::Gpio;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// The gpio pin to which the fan is connected.
    gpio_pin: u8,

    /// The interval duration in seconds (int) to check the temperature.
    interval: u64,

    /// The temperature passes which the fan is turned on.
    on_threshold: f32,

    /// The temperature below which the fan is turned off.
    off_threshold: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gpio_pin: 17,
            interval: 15,
            on_threshold: 60.0,
            off_threshold: 50.0,
        }
    }
}

impl Config {
    fn load() -> Result<Config, ConfigError> {
        #[cfg(feature = "home_config")]
        {
            confy::load::<Self>(env!("CARGO_PKG_NAME"), "config")?.validated()
        }

        #[cfg(not(feature = "home_config"))]
        {
            const CONFIG_PATH: &str = concat!("/etc/", env!("CARGO_PKG_NAME"), "/config.toml");
            confy::load_path::<Self>(CONFIG_PATH)?.validated()
        }
    }

    fn validated(mut self) -> Result<Self, ConfigError> {
        if self.interval == 0 {
            self.interval = Self::default().interval;
        }

        let Self {
            on_threshold,
            off_threshold,
            ..
        } = self;

        if off_threshold >= on_threshold {
            return Err(ConfigError::InvalidThresholdRange {
                off_threshold,
                on_threshold,
            });
        }

        Ok(self)
    }
}

#[derive(Error, Debug)]
enum ConfigError {
    #[error("`off_threshold` must be less than `on_threshold`, but is {off_threshold} and {on_threshold} respectively")]
    InvalidThresholdRange {
        off_threshold: f32,
        on_threshold: f32,
    },
    #[error("{0}")]
    ConfyError(#[from] ConfyError),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Config {
        interval: period,
        on_threshold,
        off_threshold,
        gpio_pin,
    } = Config::load()?;

    let mut interval = time::interval(Duration::from_secs(period));
    let mut fan_pin = Gpio::new()?.get(gpio_pin)?.into_output();

    print!(
        "⚙️ Monitoring cpu temperature with `vcgencmd measure_temp` every {} seconds (INTERVAL environment variable).\n",
        period
    );
    print!(
        "Turns on fan if over {}'C (ON_THRESHOLD env), off if below {}'C (OFF_THRESHOLD env).\n",
        on_threshold, off_threshold
    );
    println!(
        "Using GPIO pin {} to control the fan (GPIO_PIN env).",
        gpio_pin
    );

    let mut do_if_overheat_change_exceeds_value = {
        let mut last_overheat_amount: Option<f32> = None;

        move |temp: f32, max_change: f32, f: fn(f32)| {
            let is_not_overheating = temp < on_threshold;

            if is_not_overheating {
                if last_overheat_amount.is_some() {
                    last_overheat_amount = None;
                }
                return;
            }

            let overheat_amount = temp - on_threshold;
            let overheat_change = last_overheat_amount.map(|v| (v - overheat_amount).abs());
            let exceeded_max_change = overheat_change.filter(|v| *v <= max_change).is_none();

            if exceeded_max_change {
                last_overheat_amount = Some(overheat_amount);
                f(temp);
            }
        }
    };

    loop {
        interval.tick().await;

        let temp = read_temperature()?;

        if fan_pin.is_set_low() && temp > on_threshold {
            fan_pin.set_high();
        } else if fan_pin.is_set_high() && temp < off_threshold {
            fan_pin.set_low();

            println!("😌: {}'C", temp);
        }

        do_if_overheat_change_exceeds_value(temp, 5.0, |t| println!("🥵: {}'C", t));
    }
}

#[derive(Error, Debug)]
enum ReadTempError {
    #[error("reading failed: {0}")]
    CommandOutputError(#[from] std::io::Error),
    #[error("expected format is `temp=<num>'C\\n`, instead is `{0}`")]
    ParseError(String),
}

fn read_temperature() -> Result<f32, ReadTempError> {
    let output = Command::new("vcgencmd").arg("measure_temp").output()?;
    let output = String::from_utf8_lossy(&output.stdout);

    let temp_str = &output["temp=".len()..(output.len() - "'C\n".len())];

    temp_str
        .parse()
        .map_err(|_| ReadTempError::ParseError(output.into_owned()))
}
