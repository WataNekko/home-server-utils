# fancontrold

A program that controls a Raspberry Pi's cooling fan based on the CPU's temperature.

It periodically reads the temperature value with `vcgencmd measure_temp` and toggle a GPIO pin connected to the fan based on a given threshold.

The following environment variables can be set when running the program:
- GPIO_PIN: the gpio pin to which the fan is connected (default 17).
- INTERVAL: the interval duration in seconds (int) to check the temperature (default 15).
- ON_THRESHOLD: the temperature passes which the fan is turned on (default 60).
- OFF_THRESHOLD: the temperature below which the fan is turned off (default 50).