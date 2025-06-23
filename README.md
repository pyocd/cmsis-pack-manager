# cmsis-pack-manager

[![PyPI](https://img.shields.io/pypi/v/cmsis-pack-manager.svg)](https://pypi.python.org/pypi/cmsis-pack-manager)
[![Actions Status](https://github.com/pyocd/cmsis-pack-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/pyocd/cmsis-pack-manager/actions)

`cmsis-pack-manager` is a python module, Rust crate and command line utility
for managing current device information that is stored in many CMSIS PACKs.
Users of `cmsis-pack-manager `may query for information such as processor
type, flash algorithm and memory layout information in a python program or
through the command line utility, `pack-manager`, provided as part of this
module.

# DOCS!

They live here: https://pyocd.github.io/cmsis-pack-manager/

# Building

To build `cmsis-pack-manager` locally, Install a stable rust compiler. See
https://rustup.rs/ for details on installing `rustup`, the Rust toolchain
updater. Afterwards, run `rustup toolchain install` to get the Rust toolchain
and build system for building `cmsis-pack-manager`.

After installing the rust toolchain and downloading a stable compiler, run
`pip wheel .` from the root of this repo to generate a binary wheel (`.whl`
file). Alternatively you can run `pip install maturin cffi` and then
`maturin build` for a process closer to the way we build releases.

For testing purposes, there is a CLI written in Rust within the rust
workspace as the package `cmsis-cli`. For example From the `rust`
directory, `cargo run -p cmsis-cli -- update` builds this testing
CLI and runs the update command, for example.
