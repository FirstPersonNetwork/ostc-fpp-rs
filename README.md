# Linux Kernel Maintainer Verification

[![Rust](https://img.shields.io/badge/rust-1.88.0%2B-blue.svg?maxAge=3600)](https://github.com/FirstPersonNetwork/lkmv)

## Rust Feature Flags

- `default`: [openpgp-card]
  - To disable default features, use `--no-default-features` on all `cargo` commands.
- `openpgp-card`: Enables support for openpgp-card compatible devices (included in
  default).

## Initial Setup

Install the app locally from source:

```bash
cargo install --path .
```

**_NOTE:_** _This will in the future be replaced in the future when this is published
on [crates.io]_

Run the setup wizard:

```bash
lkmv setup
```
