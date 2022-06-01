# Neotron-BMC-Pico

## Introduction

This folder is for the Board Management Controller (BMC) on a [Neotron Pico](https://github.com/neotron-compute/neotron-pico).

## Hardware Interface

The NBMC firmware is designed to run on an ST Micro STM32F0 (STM32F031K6T6 or STM32F030K6T6) microcontroller

* 32-bit Arm Cortex-M0+ Core
* 3.3V I/O (5V tolerant)
* 32 KiB Flash
* 4 KiB SRAM
* LQFP-32 package (0.8mm pitch, 7mm x 7mm)


| Pin  | Name | Signal      | Function                                     |
| :--- | :--- | :---------- | :------------------------------------------- |
| 02   | PF0  | BUTTON_nPWR | Power Button Input (active low)              |
| 03   | PF1  | BUTTON_nRST | Reset Button Input (active low)              |
| 06   | PA0  | MON_3V3     | 3.3V rail monitor Input (1.65V nominal)      |
| 07   | PA1  | MON_5V      | 5.0V rail monitor Input (1.65V nominal)      |
| 08   | PA2  | nSYS_RESET  | System Reset Output (active low)             |
| 09   | PA3  | DC_ON       | PSU Enable Output (active high)              |
| 10   | PA4  | SPI1_nCS    | SPI Chip Select Input (active low)           |
| 11   | PA5  | SPI1_SCK    | SPI Clock Input                              |
| 12   | PA6  | SPI1_CIPO   | SPI Data Output                              |
| 13   | PA7  | SPI1_COPI   | SPI Data Input                               |
| 14   | PB0  | LED0        | Output for Power LED                         |
| 15   | PB1  | LED1        | Output for Status LED                        |
| 18   | PA8  | IRQ_nHOST   | Interrupt Output to the Host (active low)    |
| 19   | PA9  | USART1_TX   | UART Transmit Output                         |
| 20   | PA10 | USART1_RX   | UART Receive Input                           |
| 21   | PA11 | USART1_CTS  | UART Clear-to-Send Output                    |
| 22   | PA12 | USART1_RTS  | UART Ready-to-Receive Input                  |
| 23   | PA13 | SWDIO       | SWD Progamming Data Input                    |
| 24   | PA14 | SWCLK       | SWD Programming Clock Input                  |
| 25   | PA15 | PS2_CLK0    | Keyboard Clock Input                         |
| 26   | PB3  | PS2_CLK1    | Mouse Clock Input                            |
| 27   | PB4  | PS2_DAT0    | Keyboard Data Input                          |
| 28   | PB5  | PS2_DAT1    | Mouse Data Input                             |
| 29   | PB6  | I2C1_SCL    | I²C Clock                                    |
| 30   | PB7  | I2C1_SDA    | I²C Data                                     |

Note that in the above table, the UART signals are wired as _Data Terminal Equipment (DTE)_ (i.e. like a PC, not like a Modem). Connect the NMBC *UART Transmit Output* pin to the *Input* pin of something like an FTDI TTL-232R-3V3 cable.

This hardware design should also be pin-compatible with the following SoCs (although this firmware may need changes):

* STM32F042K4Tx
* STM32F042K6Tx
* STM32L071KBTx
* STM32L071KZTx
* STM32L072KZTx
* STM32L081KZTx
* STM32L082KZTx

Note that not all STM32 pins are 5V-tolerant, and the PS/2 protocol is a 5V open-collector system, so ensure that whichever part you pick has 5V-tolerant pins (marked `FT` or `FTt` in the datasheet) for the PS/2 signals. All of the parts above _should_ be OK, but they haven't been tested. Let us know if you try one!

## Build Requirements

1. rustup and Rust
   - see https://www.rust-lang.org
2. The `thumbv6m-none-eabi` target
   - run `rustup target add thumbv6m-none-eabi`
3. `probe-run`
   - run `cargo install probe-run`
4. `flip-link`
   - run `cargo install flip-link`

Then to build and flash for an STM32F031K6T6, connect a probe supported by probe-rs (such as a SEGGER J-Link, or an ST-Link) and run:

```
$ cargo run --release --features stm32f031
```

For an STM32F030K6T6, run:


```
$ cargo run --release --features stm32f030x6
```

## Licence

This source code as a whole is licensed under the GPL v3. Third-party crates are covered by their respective licences.

