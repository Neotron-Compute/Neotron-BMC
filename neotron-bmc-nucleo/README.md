# Neotron-BMC-Nucleo

## Introduction
This folder is for Neotron BMC Nucleo.
TODO: more details?

## Hardware Interface
TODO: copy here from main Readme?

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