# fancontrold

A program that controls a Raspberry Pi's cooling fan based on the CPU's temperature.

It periodically reads the temperature value with `vcgencmd measure_temp` and toggle a GPIO pin connected to the fan based on a given threshold.