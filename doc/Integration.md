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

## License

Copyright (C)  2019 The cinit developers.
Permission is granted to copy, distribute and/or modify this document
under the terms of the GNU Free Documentation License, Version 1.3
or any later version published by the Free Software Foundation;
with no Invariant Sections, no Front-Cover Texts, and no Back-Cover Texts.
A copy of the license is included alongside this document.

