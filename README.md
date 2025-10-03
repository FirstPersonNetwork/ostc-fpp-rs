# Linux Kernel Maintainer Verification

[![Rust](https://img.shields.io/badge/rust-1.88.0%2B-blue.svg?maxAge=3600)](https://github.com/FirstPersonNetwork/lkmv)

## Rust Feature Flags

- `default`: [openpgp-card]
- `openpgp-card`: Enables support for openpgp-card compatible devices.

## Initial Setup

Install the app locally from source:

```bash
cargo install --path .
```

Run the setup wizard:

```bash
lkmv setup
```
