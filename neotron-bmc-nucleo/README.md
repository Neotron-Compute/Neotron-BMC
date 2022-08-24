# Neotron-BMC-Nucleo

## Introduction

This folder is for the Board Management Controller (BMC) when running on an STM32F4 Nucleo board.

## Hardware Interface

This firmware runs on an ST Nucleo-F401RE, for development and debugging purposes. The STM32F401RET6U MCU has:

* 32-bit Arm Cortex-M4 Core
* 3.3V I/O (5V tolerant)
* 512 KiB Flash
* 96 KiB SRAM
* LQFP64 package (10 * 10 mm)

| CPU Pin | Nucleo-64 Pin | Name | Signal      | Function                                  |
| ------- | ------------- | ---- | ----------- | ----------------------------------------- |
| 2       | CN7 23        | PC13 | BUTTON_nPWR | Power Button Input (active low)           |
| 33      | CN10 16       | PB12 | BUTTON_nRST | Reset Button Input (active low)           |
| 34      | CN10 30       | PB13 | MON_3V3     | 3.3V rail monitor Input (1.65V nominal)   |
| 35      | CN10 28       | PB14 | MON_5V      | 5.0V rail monitor Input (1.65V nominal)   |
| 36      | CN10 26       | PB15 | nSYS_RESET  | System Reset Output (active low)          |
| 8       | CN8 6         | PC0  | DC_ON       | PSU Enable Output (active high)           |
| 20      | CN8 3         | PA4  | SPI1_NSS    | SPI Chip Select Input (active low)  ??    |
| 21      | CN5 6         | PA5  | SPI1_SCK    | SPI Clock Input                           |
| 22      | CN5 5         | PA6  | SPI1_MISO   | SPI Data Output                           |
| 23      | CN5 4         | PA7  | SPI1_MOSI   | SPI Data Input                            |
| 9       | CN6 5         | PC1  | POWER_LED   | Output for Power LED                      |
| 10      | CN7 35        | PC2  | STATUS_LED  | Output for Status LED                     |
| 11      | CN7 37        | PC3  | IRQ_nHOST   | Interrupt Output to the Host (active low) |
| 42      | CN10 21       | PA9  | USART1_TX   | UART Transmit Output                      |
| 43      | CN10 33       | PA10 | USART1_RX   | UART Receive Input                        |
| 44      | CN10 14       | PA11 | USART1_CTS  | UART Clear-to-Send Output                 |
| 45      | CN10 12       | PA12 | USART1_RTS  | UART Ready-to-Receive Input               |
| 46      | CN7 13        | PA13 | SWDIO       | SWD Progamming Data Input                 |
| 49      | CN7 15        | PA14 | SWCLK       | SWD Programming Clock Input               |
| 25      | CN10 6        | PC5  | PS2_CLK0    | Keyboard Clock Input                      |
| 26      | CN8 4         | PB0  | PS2_CLK1    | Mouse Clock Input                         |
| 27      | CN10 24       | PB1  | PS2_DAT0    | Keyboard Data Input                       |
| 28      | CN10 22       | PB2  | PS2_DAT1    | Mouse Data Input                          |
| 58      | CN10 17       | PB6  | I2C1_SCL    | I²C Clock                                 |
| 59      | CN7 21        | PB7  | I2C1_SDA    | I²C Data                                  |

## Build Requirements

1. rustup and Rust
   - see https://www.rust-lang.org
2. The `thumbv7em-none-eabi` target
   - run `rustup target add target=thumbv7em-none-eabi`
3. `probe-run`
   - run `cargo install probe-run` from your `$HOME` dir (not this folder!)
4. `flip-link`
   - run `cargo install flip-link` from your `$HOME` dir (not this folder!)

Then to build and flash, connect a probe supported by probe-rs (such as a SEGGER J-Link, or an ST-Link) and run:

```
$ cargo run --release
```