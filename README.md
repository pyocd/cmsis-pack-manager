# cmsis-pack-manager
cmsis-pack-manager is a python module, Rust crate and command line utility for managing current device information that is stored in many CMSIS PACKs. Users of cmsis-pack-manager may query for information such as processor type, flash algorithm and memory layout information in a python program or through the command line utility, `pack-manager`, provided as part of this module.

# CI Status
[![Windows Build status](https://ci.appveyor.com/api/projects/status/tltovxvu20y4pma8?svg=true)](https://ci.appveyor.com/project/theotherjimmy/cmsis-pack-manager) [![Mac and Linux Build Status](https://travis-ci.org/ARMmbed/cmsis-pack-manager.svg?branch=master)](https://travis-ci.org/ARMmbed/cmsis-pack-manager)

## Wheels

The last step of CI uploads binary wheels to [this S3 bucket.](http://mbed-os.s3-website-eu-west-1.amazonaws.com/?prefix=builds/cmsis-pack-manager/dist/)

# DOCS!

They live here: https://pyocd.github.io/cmsis-pack-manager/

# Building

To build cmsis-pack-manager locally, Install a stable rust compiler.
See https://rustup.rs/ for details on installing `rustup`, the rust
toolchain updater. Afterwards, run `rustup update stable` to get the
most recent stable rust toolchain and build system.

After installing the rust toolchain and downloading a stable compiler,
run `python2 setup.py bdist_wheel` from the root of this repo to
generate a binary wheel (`.whl` file) in the same way as we release.

For testing purposes, there is a CLI written in Rust within the rust
workspace as the package `cmsis-cli`. For example From the `rust`
directory, `cargo run -p cmsis-cli -- update` builds this testing
CLI and runs the update command, for example.
