# cinit

Init program for UNIX processes. Original development was done
[here](https://github.com/vs-eth/scinit)

## Configuration

cinit takes its configuration via yaml files. They can look like this:

```yml
programs:
  - name: Some descriptive name

    # The path of the binary to run
    path: /usr/bin/echo

    # Arguments to pass (see below)
    args:
      - -e
      - "hello world\\n"
      # This will be "my_program_arg", see below
      - "{{ NAME }}_arg"

    # Set the current working directoy
    workdir: /some/path

    # See Program Types
    type:
      cronjob:
        timer: 1 2 3 4 5

    # If none or invalid is given, root is used
    uid: 0
    gid: 0
    user: root
    group: root

    # Specify dependencies, see below
    before:
      - other program name
    after:
      - other program name

    # Emulate a pseudo-terminal for this program
    pty: false

    # Give capabilities to this program
    capabilities:
      - CAP_NET_RAW

    # Pass environment variables
    env:
      - PWD: /home/user
        # If no value is given, it is forwarded from cinit
      - PASSWORD:
      - NAME: my_program
        # This will be rendered to "my_program_user"
      - PROGRAM_USER: {{ NAME }}_user
```

If a file is passed via command line it is the only file used. Passing a
directory makes cinit traverse it recursively and taking all found files as
configuration. If no path is given /etc/cinit.yml is used.

Many more examples can be found in the repository's `system-tests/usecases`.
Beware however that some of these examples test error situations. This is
usually indicated by the subdirectory names.

### Arguments

Pass arguments to the program to run. You SHOULD only pass one whitespace-
separated word per list item.

As with environment variables these strings support templating where you have
access to all environment variables.

### Program types

A program to be executed once is of type `oneshot`. The corresponding
representation in YAML is just `type: oneshot`. This is the default if none
is given.

A program which is called periodically is of type `cronjob`. The YAML
representation of this is nested:

```yaml
type:
  cronjob:
    timer: 1 2 3 4 5
```

See [`man cron`](https://manpages.debian.org/stretch/cron/crontab.5.en.html) for
a description of the time format. A cronjob must not have dependencies.

:::warning
The implementation of cron timer specifications deviates from the man page
linked above by not differentiating between `*` and the full range of valid
values given, e.g. `1-31` for the day. This is relevant for interactions
between date specifications and weekday specification.
:::

:::info
Special string specifications as `@monthly` are not supported.
:::

### Environment

By default the following environment variables will be forwarded from the
cinit process to the programs and are always present:

* `HOME`
* `LANG`
* `LANGUAGE`
* `LOGNAME`
* `PATH`
* `PWD`
* `SHELL`
* `TERM`
* `USER`

Additional parameters may be specified. If no value is given, cinit will
forward the value from its own environment. If the value is not present in
cinit's environment, no value will be passed (instead of an empty one).

The values of the variables support simple templating. Use `{{ VAR }}` to
refer to another variable in the environment. Note that `VAR` has to be
defined before it can be referenced!

### Capabilities

Processes can be restricted in what they are allowed to do. This can also
mean that non-root process get elevated capabilities. See
[here](http://man7.org/linux/man-pages/man7/capabilities.7.html)
for a list of all capabilities.

### Dependencies

Programs are allowed to depend on each other via the `before` and `after`
fields. Dendendant processes will only be started once all their
dependencies have terminated. Refer to other programs in the config via
their `name` field.

If the dependencies form a cycle, this is reported before any process is
started and cinit terminates.

## Usage

```text
cinit 0.1.0
init daemon for other programs, suitable for containers

USAGE:
    cinit [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Output information while running

OPTIONS:
    -c, --config <PATH>    The config file or directory to run with [default: /etc/cinit.yml]
```

## Logging

cinit combines the log output of children with its own. The log format is as
follows:

`<TIMESTAMP> <LEVEL> [<NAME>] <MESSAGE>`

* `TIMESTAMP`: This follows the pattern `YYYY-MM-DD'T'HH:MM:SS.mmm`.
  Example: `1970-11-23T13:25:44.567`.

* `LEVEL`: One of `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`.

* `NAME`: This is either the string `cinit` or the name of a child as defined
  in the `name` attribute in the YAML config.

* `MESSAGE`: The actual event being reported.

