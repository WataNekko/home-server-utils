use std::{env, error::Error, process::Command, time::Duration};

use rppal::gpio::Gpio;
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
    fn load() -> Config {
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
            panic!("OFF_THRESHOLD must be less than ON_THRESHOLD");
        }

        Config {
            interval,
            on_threshold,
            off_threshold,
            gpio_pin,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let Config {
        interval: period,
        on_threshold,
        off_threshold,
        gpio_pin: fan_pin,
    } = Config::load();

    let mut interval = time::interval(Duration::from_secs(period));
    let mut fan_pin = Gpio::new()?.get(fan_pin)?.into_output();

    loop {
        interval.tick().await;

        let temp = read_temperature();

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

fn read_temperature() -> f32 {
    let output = Command::new("vcgencmd")
        .arg("measure_temp")
        .output()
        .expect("Temperature read command should not have failed");
    let output = String::from_utf8_lossy(&output.stdout);

    let temp_str = &output["temp=".len()..(output.len() - "'C\n".len())];

    temp_str
        .parse()
        .expect("Temperature reading should be in the format `temp=<num>'C\\n`")
}
