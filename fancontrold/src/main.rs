use std::{env, process::Command, time::Duration};

use rppal::gpio::Gpio;
use thiserror::Error;
use tokio::time;

struct Config {
    /// The gpio pin to which the fan is connected (default 17).
    gpio_pin: u8,

    /// The interval duration in seconds (int) to check the temperature (default 15).
    interval: u64,

    /// The temperature passes which the fan is turned on (default 60).
    on_threshold: f32,

    /// The temperature below which the fan is turned off (default 50).
    off_threshold: f32,
}

impl Config {
    fn load() -> Result<Config, ConfigError> {
        let interval = env::var("INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|v| *v > 0) // time::interval panics on zero duration
            .unwrap_or(15);

        let gpio_pin = env::var("GPIO_PIN")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(17);

        let on_threshold = env::var("ON_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60.0);

        let off_threshold = env::var("OFF_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50.0);

        if off_threshold >= on_threshold {
            return Err(ConfigError::InvalidThresholdRange {
                off_threshold,
                on_threshold,
            });
        }

        Ok(Config {
            interval,
            on_threshold,
            off_threshold,
            gpio_pin,
        })
    }
}

#[derive(Error, Debug)]
enum ConfigError {
    #[error("OFF_THRESHOLD must be less than ON_THRESHOLD, but is {off_threshold} and {on_threshold} respectively")]
    InvalidThresholdRange {
        off_threshold: f32,
        on_threshold: f32,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Config {
        interval: period,
        on_threshold,
        off_threshold,
        gpio_pin: fan_pin,
    } = Config::load()?;

    let mut interval = time::interval(Duration::from_secs(period));
    let mut fan_pin = Gpio::new()?.get(fan_pin)?.into_output();

    loop {
        interval.tick().await;

        let temp = read_temperature()?;

        println!("{}", temp);
        if fan_pin.is_set_low() && temp > on_threshold {
            fan_pin.set_high();
            println!("on");
        } else if fan_pin.is_set_high() && temp < off_threshold {
            fan_pin.set_low();
            println!("off");
        }
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
