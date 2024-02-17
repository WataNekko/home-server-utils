use std::{env, process::Command, time::Duration};

use tokio::time;

struct Config {
    interval: Duration,
    on_threshold: f32,
    off_threshold: f32,
    gpio_pin: u32,
}

impl Config {
    fn load() -> Config {
        let interval = env::var("INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .filter(|v| *v > 0) // time::interval panics on zero duration
            .unwrap_or(15);
        let interval = Duration::from_secs(interval);

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
async fn main() {
    let Config {
        interval: period,
        on_threshold,
        off_threshold,
        gpio_pin,
    } = Config::load();

    let mut interval = time::interval(period);

    loop {
        interval.tick().await;

        let temp = read_temperature();

        print!("{} ", temp);
        if temp > on_threshold {
            print!("on");
        } else if temp < off_threshold {
            print!("off");
        }
        println!();
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
