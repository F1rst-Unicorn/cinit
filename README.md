# cinit

Init program for UNIX processes. Inspired by
[scinit](https://github.com/vs-eth/scinit).

## Usage

See [here](doc/README.md) for user documentation.

Run `cargo doc --no-deps --open -p cinit` to view internal developer
documentation.

## Building

The project is a normal cargo build.

System tests require to be run as root, which most easily works with docker. As
a user having access to a docker daemon run `scripts/local/build` which
executes all checks.

### Release Build

Simply run `cargo build --release`.

The self-contained binary is stored at
`target/x86_64-unknown-linux-musl/release/cinit`

Alternatively, as a user with access to a docker daemon run

```bash
export BUILD_FLAGS="--release"
scripts/local/build
```
