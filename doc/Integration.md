# cinit Integration

If you want to configure your program to be started by cinit, please refer to
`README.md`.

## Configuration

cinit can work with either a single configuration file or with a directory
recursively containing an arbitrary number of configuration files. The variant
with a directory should be preferred. 

This is achieved by passing the directory or file via CLI argument, e.g.
`cinit --config /etc/cinit.d`.

## Logging

Production setups SHOULD log without specifying any `--verbose` flag.
