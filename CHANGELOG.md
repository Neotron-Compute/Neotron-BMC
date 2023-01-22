# Changelog

## Unreleased Changes

* None

## v0.5.1

* Adds a PC speaker driver using TIM14
* Plays a beep on startup
* Lets the host play beeps using the SPI interface

## v0.5.0

* Generates Host Interrupts
* SPI interface reliability is improved

## v0.4.3

* Fix version number reporting - now comes from `git describe`

## v0.4.2

* Improvements to SPI communications link
* Move some processing out of interrupts and into the main loop, to improve reliability

## v0.4.1

* Update dependencies (moves away from yanked critical-section 0.2.x)
* Fixes to the protocol documentation
* Add skeleton SPI command interface, with PS/2 Keyboard FIFO read command
* PS/2 Keyboard words time-out if you get a glitch

## v0.4.0

* Add very basic SPI interface support to neotron-bmc-pico
* No changes to neotron-bmc-nucleo (it's now out of date)
* Added `neotron-bmc-protocol` crate at v0.1.0

## v0.3.1
* Reset button triggers 250ms low pulse
* Fix STM32F030 support and remove STM32F031 support for neotron-bmc-pico

## v0.3.0
* Add STM32F030 support to neotron-bmc-pico

## v0.2.0
* Change to blink power LED when in standby
* Actually controls DC power and reset (but doesn't check the voltage rails yet)

## v0.1.0
* Skeleton application using knurling template
* Started work on command protocol definition
* LED Blinking Modes defined
* SPI Frame Format revised
