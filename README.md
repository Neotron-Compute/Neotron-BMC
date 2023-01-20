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

## SPI Communications Protocol

The SPI interface runs in SPI mode 0 (clock line idles low, data sampled on
rising edge) at 1 MHz (higher speeds TBD). It uses frames made up of 8-bit
words.

To communicate with the NBMC, the Host Processor must first take the Chip Select
line (`SPI1_nCS`) low, then send a Header. SPI is a full-duplex system, but in
this system only one side is actually transferring useful data at any time, so
whilst the Header is being sent the Host will receive Padding Bytes of `0xFF` in
return (which can be discarded).

The NBMC exposes a number of registers - some can be read, some can be written
to, some are cleared when written to.

See [neotron-bmc-protocol's README](./neotron-bmc-protocol/README.md) for more
details of how the registers are accessed. The registers themselves are defined
below.

## System Registers

| Address | Name                                  | Type  | Contains                                                 | Length   |
| :-----: | ------------------------------------- | :---: | -------------------------------------------------------- | :------: |
| 0x00    | Protocol Version                      | RO    | The NBMC protocol version, [1, 0, 0]                     | 3        |
| 0x01    | Firmware Version                      | RO    | The NBMC firmware version, as a null-padded UTF-8 string | 32       |
| 0x10    | Interrupt Status                      | R/W1C | Which interrupts are currently active, as a bitmask.     | 2        |
| 0x11    | Interrupt Control                     | R/W   | Which interrupts are currently enabled, as a bitmask.    | 2        |
| 0x20    | Button Status                         | RO    | The current state of the buttons                         | 1        |
| 0x21    | System Temperature                    | RO    | Temperature in °C, as an `i8`                            | 1        |
| 0x22    | System Voltage (Standby 3.3V rail)    | RO    | Voltage in Volts/32, as a `u8`                           | 1        |
| 0x23    | System Voltage (Main 3.3V rail)       | RO    | Voltage in Volts/32, as a `u8`                           | 1        |
| 0x24    | System Voltage (5.0V rail)            | RO    | Voltage in Volts/32, as a `u8`                           | 1        |
| 0x25    | Power Control                         | R/W   | Enable/disable the power supply                          | 1        |
| 0x30    | UART Receive/Transmit Buffer          | FIFO  | Data received/to be sent over the UART                   | up to 64 |
| 0x31    | UART FIFO Control                     | R/W   | Settings for the UART FIFO                               | 1        |
| 0x32    | UART Control                          | R/W   | Settings for the UART                                    | 1        |
| 0x33    | UART Status                           | R/W1C | The current state of the UART                            | 1        |
| 0x34    | UART Baud Rate                        | R/W   | The UART baud rate in bps, as a `u32le`                  | 4        |
| 0x40    | PS/2 Keyboard Receive/Transmit Buffer | FIFO  | Data received/to be sent over the PS/2 keyboard port     | up to 16 |
| 0x41    | PS/2 Keyboard Control                 | R/W   | Settings for the PS/2 Keyboard port                      | 1        |
| 0x42    | PS/2 Keyboard Status                  | R/W1C | Current state of the PS/2 Keyboard port                  | 1        |
| 0x50    | PS/2 Mouse Receive/Transmit Buffer    | FIFO  | Data received/to be sent over the PS/2 Mouse port        | up to 16 |
| 0x51    | PS/2 Mouse Control                    | R/W   | Settings for the PS/2 Mouse port                         | 1        |
| 0x52    | PS/2 Mouse Status                     | R/W1C | Current state of the PS/2 Mouse port                     | 1        |
| 0x60    | I²C Receive/Transmit Buffer           | FIFO  | Data received/to be sent over the I²C Bus                | up to 16 |
| 0x61    | I²C FIFO Control                      | R/W   | Settings for the I²C FIFO                                | 1        |
| 0x62    | I²C Control                           | R/W   | Settings for the I²C Bus                                 | 1        |
| 0x63    | I²C Status                            | R/W1C | Current state of the I²C Bus                             | 1        |
| 0x64    | I²C Baud Rate                         | R/W   | The I²C clock rate in Hz, as a `u32le`                   | 4        |

The register types are:

* `RO` - read only register, where writes will return an error
* `R/W` - read/write register
* `R/W1C` - reads as usual, but when writing a 1 bit clears that bit position and a 0 bit is ignored
* `FIFO` - a first-in, first-out buffer

### Address 0x01 - Protocol Version

This read-only register returns the protocol version supported. The protocol
version includes the set of registers, and the meaning of the fields within
those registers. A *Host* should first verify that the *NBMC* it is talking to
is semantically compatible before reading any other registers.

The three bytes are `major`, `minor` and `patch`. This document corresponds to
`[1, 0, 0]` (or *v1.0.0*).

### Address 0x01 - Firmware Version

This read-only register returns the firmware version of the NBMC, as a UTF-8
string. The register length is always 64 bytes, and the string is null-padded.
We also guarantee that the firmware version will always be less than or equal to
63 bytes, so you can also treat this string as null-terminated.

An official release will have a version string of the form `tags/v1.2.3`. An
unofficial release might be `heads/develop-dirty`. It is not recommended that
you rely on these formats or attempt to parse the version string. It is however
useful if you can quote this string when reporting issues with the firmware.

### Address 0x02 - Interrupt Status

This eight bit register indicates which Interrupts are currently 'active'. An
Interrupt will remain 'active' until a word is written to this register with a 1
bit in the relevant position.

| Bit | Interrupt                  |
| --- | -------------------------- |
| 7   | Voltage Alarm              |
| 6   | Button State Change        |
| 5   | UART TX Empty              |
| 4   | UART RX Not Empty          |
| 3   | I²C TX Empty               |
| 2   | I²C RX Not Empty           |
| 1   | PS/2 Mouse RX Not Empty    |
| 0   | PS/2 Keyboard RX Not Empty |

### Address 0x03 - Interrupt Control

This eight bit register indicates which Interrupts are currently 'enabled'. The
IRQ_nHOST signal is a level interrupt and it will be active (LOW) whenever the
value in the Interrupt Control register ANDed with the Interrupt Status register
is non-zero.

The bits have the same ordering as the Interrupt Status register.

### Address 0x04 - Button Status

This eight-bit register indicates the state of the power button.

Note that if the power button is held down for three seconds, the system will
power-off instantly, regardless of what the host does.

Note also that is it not possible to sample the reset button - pressing the
reset button will instantly assert the system reset line, rebooting the Host.

| Bits | Meaning                               |
| ---- | ------------------------------------- |
| 7-1  | Reserved for future use               |
| 0    | Power Button: 0 = normal, 1 = pressed |

### Address 0x04 - System Temperature

This eight-bit register provides the current system temperature in °C, as
measured on the STM32's internal temperature sensor. It is updated around once a
second.

### Address 0x05 - System Voltage (Standby 3.3V rail)

This eight-bit register provides the current 3.3V rail voltage in units of 1/32
of a Volt. It is updated around once a second. A value of 105 (3.28V) to 106
(3.31V) is nominal. An interrupt is raised when the value exceeds 3.63V (116) or
is lower than 2.97V (95).

### Address 0x06 - System Voltage (Main 3.3V rail)

This eight-bit register provides the current 3.3V rail voltage in units of 1/32
of a Volt. It is updated around once a second. A value of 105 (3.28V) to 106
(3.31V) is nominal. An interrupt is raised when the value exceeds 3.63V (116) or
is lower than 2.97V (95).

### Address 0x07 - System Voltage (5.0V rail)

This eight-bit register provides the current 3.3V rail voltage in units of 1/32
of a Volt. It is updated around once a second. A value of 160 (5.00V) is
nominal. An interrupt is raised when the value exceeds 5.5V (176) or is lower
than 4.5V (144).

### Address 0x08 - Power Control

This eight-bit register controls the main DC/DC power supply unit. The Host
should disable the DC/DC supply (by writing zero here) if it wishes to power
down.

| Bits | Meaning                        |
| ---- | ------------------------------ |
| 7-1  | Reserved for future use        |
| 0    | DC/DC control: 0 = off, 1 = on |

### Address 0x10 - UART Receive/Transmit Buffer

TODO

### Address 0x11 - UART FIFO Control

TODO

### Address 0x12 - UART Control

TODO

### Address 0x13 - UART Status

TODO

### Address 0x14 - UART Baud Rate

TODO

### Address 0x20 - PS/2 Keyboard Receive/Transmit Buffer

TODO

### Address 0x21 - PS/2 Keyboard Control

TODO

### Address 0x22 - PS/2 Keyboard Status

TODO

### Address 0x30 - PS/2 Mouse Receive/Transmit Buffer

TODO

### Address 0x31 - PS/2 Mouse Control

TODO

### Address 0x32 - PS/2 Mouse Status

TODO

### Address 0x40 - I²C Receive/Transmit Buffer

TODO

### Address 0x41 - I²C FIFO Control

TODO

### Address 0x42 - I²C Control

TODO

### Address 0x43 - I²C Status

TODO

### Address 0x44 - I²C Baud Rate

TODO

## Build Requirements

Build requirements are available for
[Neotron-BMC-pico](neotron-bmc-pico/README.md) and
[Neotron-BMC-nucleo](neotron-bmc-nucleo/README.md). 

## Licence

This code is licenced under the GNU Public Licence version 3. See:

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
based on the Knurling Template at https://github.com/knurling-rs/app-template.
