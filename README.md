# Neotron-BMC

Firmware for the Neotron Board Management Controller (NBMC).

## Introduction

The NMBC is an always-on microcontroller used on Neotron systems. It has very low idle power consumption, allowing it to remain powered up at all times. This lets it listen to button events from the Power and Reset buttons, and control the system LEDs, main `~RESET` signal and turn the main 5V Power Rail on and off. This lets your Neotron system have smart 'ATX PC' style features like a soft power button, and it also ensures that all power rails come up before the system is taken out of reset.

The NBMC appears to the main Neotron system processor as an Expansion Device. As such it sits on the SPI bus as a peripheral device, with a dedicated Chip Select line and a dedicated IRQ line. It provides to the system:

* an extra I²C bus,
* a four-wire UART,
* two PS/2 ports (one for keyboard, one for a mouse),
* two analog inputs for monitoring for the 3.3V and 5.0V rails,
* two GPIO inputs for a power button and a reset button, and
* three GPIO outputs - nominally used for
    * the main DC/DC enable signal,
    * the power LED, and
    * a status LED.

## Hardware Interface

The NBMC firmware is designed to run on an ST Micro STM32F0 (STM32F031K6T6) microcontroller

* 32-bit Arm Cortex-M0+ Core
* 3.3V I/O (5V tolerant)
* 32 KiB Flash
* 4 KiB SRAM
* LQFP-32 package (0.8mm pitch)


| Pin  | Name | Signal      | Function                                     |
| :--- | :--- | :---------- | :------------------------------------------- |
| 02   | PF0  | BUTTON_nPWR | Power Button Input (active low)              |
| 03   | PF1  | HOST_nRST   | Reset Output to reset the rest of the system |
| 06   | PA0  | MON_3V3     | 3.3V rail monitor Input (1.65V nominal)      |
| 07   | PA1  | MON_5V      | 5.0V rail monitor Input (1.65V nominal)      |
| 08   | PA2  | LED0        | PWM output for first Status LED              |
| 09   | PA3  | LED1        | PWM output for second Status LED             |
| 10   | PA4  | SPI1_nCS    | SPI Chip Select Input (active low)           |
| 11   | PA5  | SPI1_SCK    | SPI Clock Input                              |
| 12   | PA6  | SPI1_CIPO   | SPI Data Output                              |
| 13   | PA7  | SPI1_COPI   | SPI Data Input                               |
| 14   | PB0  | BUTTON_nRST | Reset Button Input (active low)              |
| 15   | PB1  | DC_ON       | PSU Enable Output                            |
| 18   | PA8  | IRQ_nHOST   | Interrupt Output to the Host (active low)    |
| 19   | PA9  | I2C1_SCL    | I²C Clock                                    |
| 20   | PA10 | I2C1_SDA    | I²C Data                                     |
| 21   | PA11 | USART1_CTS  | UART Clear-to-Send Output                    |
| 22   | PA12 | USART1_RTS  | UART Ready-to-Receive Input                  |
| 23   | PA13 | SWDIO       | SWD Progamming Data Input                    |
| 24   | PA14 | SWCLK       | SWD Programming Clock Input                  |
| 25   | PA15 | PS2_CLK0    | Keyboard Clock Input                         |
| 26   | PB3  | PS2_CLK1    | Mouse Clock Input                            |
| 27   | PB4  | PS2_DAT0    | Keyboard Data Input                          |
| 28   | PB5  | PS2_DAT1    | Mouse Data Input                             |
| 29   | PB6  | USART1_TX   | UART Transmit Output                         |
| 30   | PB7  | USART1_RX   | UART Receive Input                           |

Note that in the above table, the UART signals are wired as _Data Terminal Equipment (DTE)_ (i.e. like a PC, not like a Modem).

This design should also be pin-compatible with the following SoCs (although this firmware may need changes):

* STM32F042K4Tx
* STM32F042K6Tx
* STM32L071KBTx
* STM32L071KZTx
* STM32L072KZTx
* STM32L081KZTx
* STM32L082KZTx

Note that not all STM32 pins are 5V-tolerant, and the PS/2 protocol is a 5V open-collector system, so ensure that whichever part you pick has 5V-tolerant pins (marked `FT` or `FTt` in the datasheet) for the PS/2 signals. All of the parts above _should_ be OK, but they haven't been tested. Let us know if you try one!

## Communications Protocol - SPI Frames

The SPI interface runs in SPI mode 0 (clock line idles low, data sampled on rising edge) at up to 16 MHz (TBD). It uses frames made up of 8-bit words.

To communicate with the NBMC, first take the Chip Select line (`SPI1_nCS`) low, then send the appropriate number of bytes. SPI is a full-duplex system, but in this system only one side is actually transferring useful data at any time.

Once the appropriate number of bytes have been exchange, the Chip Select line must be raised for at least XXX microseconds, before another transfer is started.

```
+-----+-------------...-------------+----------------...---------------+
| CMD | CLEN | Command Bytes 0..n   | PADDING                          | Controller Out, Peripheral In (COPI)
+-----+------+------...-------------+-----+------+---------...---------+
| PADDING                           | RSP | RLEN | Response Bytes 0..n | Controller In, Peripheral Out (CIPO)
+----------------...----------------+-----+------+---------...---------+
```

The `CMD` byte is a command, given in the Table of Commands below. `RSP` is a response byte, given in the Table of Responses below.

Taking the Chip Select line low activates an Interrupt Routine. You must leave XXX microseconds (TBD) before starting the transfer in order to give the routine time to start. The various CMD bytes either read or write various registers in RAM. Once the Chip Select is raised, the system goes into its background processing, reading from the registers to set system outputs, and writing to registers based on system inputs.

## Communications Protocol - Commands and Responses

### Table of Commands

| Command Byte | Name  |
| ------------ | ----- |
| 0x00         | PING  |
| 0x01         | READ  |
| 0x02         | WRITE |

### Table of Responses

| Response Byte | Name            |
| ------------- | --------------- |
| 0x80          | OK              |
| 0x81          | Unknown Command |
| 0xFF          | Busy            |

### PING Command

This command just checks the NBMC is awake. There is no payload. A OK response is sent in return.

### READ Command

This command reads from an address in the NBMC. The payload is the 8-bit register address, followed by the 8-bit number of bytes to read. The `CLEN` must therefore be two.

### WRITE Command

This command writes to an address in the NBMC. The payload is the 8-bit register address, followed by the 8-bit number of bytes to write, followed by the bytes themselves (up to 253). The `CLEN` must therefore be 2 + the number of bytes written.

### Table of Registers

| Address | Name                                  | Type  | Contains                                                 | Length |
| ------- | ------------------------------------- | ----- | -------------------------------------------------------- | ------ |
| 0x00    | Firmware Version                      | RO    | The NBMC firmware version, as a null-padded UTF-8 string | 64     |
| 0x01    | Hardware Version                      | RO    | The NBMC firmware version, as a null-padded UTF-8 string | 64     |
| 0x02    | Interrupt Status                      | R/W1C | Which interrupts are currently active, as a bitmask.     | 2      |
| 0x02    | Interrupt Control                     | R/W   | Which interrupts are currently enabled, as a bitmask.    | 2      |
| 0x10    | UART Receive/Transmit Buffer          | FIFO  | Data received over the UART                              | max 64 |
| 0x11    | UART FIFO Control                     | R/W   | Settings for the UART FIFO                               | 1      |
| 0x12    | UART Control                          | R/W   | ...                                                      | 1      |
| 0x13    | UART Status                           | R/W   | ...                                                      | 1      |
| 0x14    | UART Baud Rate                        | R/W   | ...                                                      | 4      |
| 0x20    | PS/2 Keyboard Receive/Transmit Buffer | FIFO  | ...                                                      | max 16 |
| 0x22    | PS/2 Keyboard Control                 | R/W   | ...                                                      | 1      |
| 0x23    | PS/2 Keyboard Status                  | R/W1C | ...                                                      | 1      |
| 0x30    | PS/2 Mouse Receive/Transmit Buffer    | FIFO  | ...                                                      | max 16 |
| 0x32    | PS/2 Mouse Control                    | R/W   | ...                                                      | 1      |
| 0x33    | PS/2 Mouse Status                     | R/W1C | ...                                                      | 1      |
| 0x40    | LED Control                           | R/W   | ...                                                      | 1      |
| 0x41    | Button Status                         | RO    | ...                                                      | 1      |

The register types are:

* `RO` - read only register, where writes will return an error
* `R/W` - read/write register
* `R/W1C` - when writing a 1 bit clears that bit position and a 0 bit is ignored, reads are as normal.
* `FIFO` - a first-in, first-out buffer

## Build Requirements

1. rustup and Rust
   - see https://www.rust-lang.org
2. The `thumbv6m-none-eabi` target
   - run `rustup target add thumbv6m-none-eabi`
3. `probe-run`
   - run `cargo install probe-run` from your `$HOME` dir (not this folder!)
4. `flip-link`
   - run `cargo install flip-link` from your `$HOME` dir (not this folder!)

Then to build and flash, connect a probe supported by probe-rs (such as a SEGGER J-Link, or an ST-Link) and run:

```
$ cargo run --bin neotron-bmc --release
```

## Licence

This code is licenced under the GNU Public Licence version 3. See:

* [The LICENSE file](./LICENSE)
* [The GPL Website](http://www.gnu.org/licenses/gpl-3.0.html)

Our intent behind picking this licence is that you must ensure anyone who receives a programmed Neotron BMC processor, also receives:

* A note stating that the firmware is licenced under the GPL v3, and
* Access to Complete and Corresponding Source Code (e.g. in the form of the URL of this repository, or the source repository the firmware was built from if not this one).

For the avoidance of doubt, it is not our intention to:

* prevent you from selling PCBs containing a programmed Neotron BMC processor, or
* prevent you from making changes to the Neotron BMC source code.

It is however our intention to ensure that anyone who sells or distributes this firmware (or products that contain it):

* provides proper attribution, and
* gives the customers/recipients access to the source code, including any changes that were made.

Note that this firmware image incorporates a number of third-party modules. You should review the output of `cargo tree` and ensure that any licence terms for those modules are upheld. You should also be aware that this application was based on the Knurling Template at https://github.com/knurling-rs/app-template.
