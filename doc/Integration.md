# cinit Integration

If you want to configure your program to be started by cinit, please refer to
[README.md](README.md).

## Configuration

cinit can work with either a single configuration file or with a directory
recursively containing an arbitrary number of configuration files. The variant
with a directory should be preferred.

This is achieved by passing the directory or file via CLI argument, e.g.
`cinit --config /etc/cinit.d`.

## Logging

Production setups SHOULD log without specifying any `--verbose` flag.

## Runtime

### Exit Codes

In case of error cinit returns with an exit code. The following exit codes have
a defined meaning:

* 0: No error

* 1: Invalid configuration (file permissions, syntax error, etc.)

* 2: Invalid configuration (more semantically, e.g. cyclic dependencies)

* 3: Runtime setup failed (epoll(), signal masking)

* 4: Child startup failed (forking, capabilities, etc.)

* 5: Precondition failed

  * Running as non-root user

  * Running on an unsupported kernel

* 6: Child process exitted with non-zero exit code

## License

Copyright (C)  2019 The cinit developers.
Permission is granted to copy, distribute and/or modify this document
under the terms of the GNU Free Documentation License, Version 1.3
or any later version published by the Free Software Foundation;
with no Invariant Sections, no Front-Cover Texts, and no Back-Cover Texts.
A copy of the license is included alongside this document.

