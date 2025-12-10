# Linux Kernel Maintainer Service

This is the top-level background service that handles all orchestration and
issuance of credentials on behalf of the Linux Kernel Maintainers.

## Initial setup

1. Install Rust (> 1.88)
2. Clone this repository
3. Create the top-level DID and place the secrets in the `conf/config.json`
   - Typically use the [diwebvh-rs crate](https://crates.io/crates/diwebvh-rs) to
     create the DID and keys
4. Run the service `cargo run --release`

Run the following for more help:

```sh
cargo run --release -- --help
```
