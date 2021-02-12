# async-std-udp-test

example for [this issue](https://github.com/async-rs/async-std/issues/955)

A UDP socket sends messages to another UDP socket. The reading side polls a socket through a manual poll function. This works in async-std 1.6.2 (as pinned in Cargo.toml) but seems to fail when changin to async-std 1.9.

