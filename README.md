# cinit

Init program for UNIX processes. Inspired by
[scinit](https://github.com/vs-eth/scinit).

## Usage

Run `cargo doc --no-deps --open -p cinit` to view extensive documentation
for clients.

## Building

The project is compiled inside docker. As a user having access to a docker
daemon run `scripts/local/build`.

### Release Build

As a user with access to a docker daemon run

```bash
export CARGO_FLAGS="--release"
scripts/local/build
```

The self-contained binary is stored at
`target/x86_64-unknown-linux-musl/release/cinit`

