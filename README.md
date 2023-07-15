# Neotron-BMC

Firmware for the Neotron Board Management Controller (NBMC).

## Introduction

The NBMC is an always-on microcontroller used on Neotron systems. It has very
low idle power consumption, allowing it to remain powered up at all times. This
lets it listen to button events from the Power and Reset buttons, and control
the system LEDs, main `~RESET` signal and turn the main 5V Power Rail on and
off. This lets your Neotron system have smart 'ATX PC' style features like a
soft power button, and it also ensures that all power rails come up before the
system is taken out of reset.

The NBMC appears to the main Neotron system processor as an Expansion Device. As
such it sits on the SPI bus as a peripheral device, with a dedicated Chip Select
line and a dedicated IRQ line. It provides to the system:

* an extra I²C bus,
* a four-wire UART,
* two PS/2 ports (one for keyboard, one for a mouse),
* two analog inputs for monitoring for the 3.3V and 5.0V rails,
* two GPIO inputs for a power button and a reset button, and
* three GPIO outputs - nominally used for
  * the main DC/DC enable signal, and
  * the power LED

## Hardware Interface

### Neotron Pico

The NBMC firmware is designed to run on an ST Micro STM32F0 (STM32F030K6T6)
microcontroller, as fitted to a [Neotron
Pico](https://github.com/neotron-compute/neotron-pico).

See the [board-specific README](./neotron-bmc-pico/README.md)

### Nucleo-F401

The NBMC firmware can also run on an ST Micro STM32F4 Nucleo board.

See the [board-specific README](./neotron-bmc-nucleo/README.md).

It's currently quite out of date compared to the Neotron Pico version.

## BMC Registers

See the [neotron-bmc-protocol](./neotron-bmc-protocol/README.md) and
[neotron-bmc-commands](./neotron-bmc-commands/README.md) for more details on how
the BMC registers are accessed and modified.

## Build Requirements

Build requirements are available for
[Neotron-BMC-pico](neotron-bmc-pico/README.md) and
[Neotron-BMC-nucleo](neotron-bmc-nucleo/README.md).

## Licence

This repository as a whole is licenced under the GNU Public Licence version 3. See:

* [The LICENSE file](./LICENSE)
* [The GPL Website](http://www.gnu.org/licenses/gpl-3.0.html)

Our intent behind picking this licence is that you must ensure anyone who
receives a programmed Neotron BMC processor, also receives:

* A note stating that the firmware is licenced under the GPL v3, and
* Access to Complete and Corresponding Source Code (e.g. in the form of the URL
  to a commit/tag in this repository, or the source repository the firmware was
  commercially distributed or was built from a different repo to this one).

For the avoidance of doubt, it is not our intention to:

* prevent you from selling PCBs containing a programmed Neotron BMC processor,
  or
* prevent you from making changes to the Neotron BMC source code.

It is however our intention to ensure that anyone who sells or distributes this
firmware (or products that contain it):

* provides proper attribution, and
* gives the customers/recipients access to the source code, including any
  changes that were made.

Note that this firmware image incorporates a number of third-party modules. You
should review the output of `cargo tree` and ensure that any licence terms for
those modules are upheld. You should also be aware that this application was
based on the Knurling Template at <https://github.com/knurling-rs/app-template>.

Note also that some crates within this tree are made available individually
under different licences. See each individual crate for details.
