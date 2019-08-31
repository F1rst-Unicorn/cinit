# cinit

Init program for UNIX processes. Original development was done
[here](https://github.com/vs-eth/scinit).

This is the user manual for developers wanting to start their programs in a
cinit enabled container. See `Integration.md` for information about building a
cinit enabled container.

## Configuration

cinit takes its configuration via [YAML](https://yaml.org/) files. The minimal
configuration to start a program is:

```yml
programs:
  - name: myprogram
    path: /the/path
```

The full list of available options is:

```yml
programs:
  - name: Some descriptive name

    # The path of the binary to run
    path: /the/path

    # Arguments to pass (see below)
    args:
      - "-e"
      - "hello world\\n"

      # This will be "my_program_arg", see below
      - "{{ NAME }}_arg"

    # Set the current working directoy
    workdir: /some/path

    # See Program Types
    type:
      cronjob:
        timer: 1 2 3 4 5

    # If none is given, root is used
    uid: 0
    gid: 0
    user: root
    group: root

    # Specify dependencies, see below
    before:
      - other-program
    after:
      - other-program

    # Emulate a pseudo-terminal for this program
    pty: false

    # Give capabilities to this program
    capabilities: []

    # Pass environment variables
    env:
      - PWD: "/home/user"
        # If no value is given, it is forwarded from cinit
      - PASSWORD:
      - NAME: "my_program"
        # This will be rendered to "my_program_user"
      - PROGRAM_USER: "{{ NAME }}_user"
```

If a file is passed via command line it is the only file used. Passing a
directory makes cinit traverse it recursively and taking all found files as
configuration. If no path is given /etc/cinit.yml is used.

Many more examples can be found in the repository's `system-tests/usecases`.
Beware however that some of these examples test error situations. This is
usually indicated by the subdirectory names.

### Arguments

Pass arguments to the program to run. You SHOULD only pass one whitespace-
separated word per list item. You SHOULD quote all arguments as they might
contain characters interpreted by the YAML parser.

As with environment variables these strings support templating. You have access
to all environment variables listed in the current program's `env` list.

### Program types

#### Oneshot

A program to be executed once is of type `oneshot`. The corresponding
representation in YAML is just `type: oneshot`. This is the default if none
is given.

#### Cronjob

A program which is called periodically is of type `cronjob`. The YAML
representation of this is nested:

```yaml
type:
  cronjob:
    timer: 1 2 3 4 5
```

See [`man cron`](https://manpages.debian.org/stretch/cron/crontab.5.en.html) for
a description of the time format. A cronjob MUST NOT have dependencies.

Cronjobs are not reentrant. This means if the timer specification wants the job
to be executed at a certain time while simultaneously the job is still running
from a previous run, it won't be executed twice. Instead it will be rescheduled
to the next time according to the timer specification.

The implementation of cron timer specifications deviates from the man page
linked above by not differentiating between `*` and the full range of valid
values given, e.g. `1-31` for the day. This is relevant for interactions
between date specifications and weekday specification.

Special string specifications as `@monthly` are not supported.

### User / Group

Specify a UNIX user and group under which to run the program. If none is given,
root is used. If an invalid name is given (e.g. because it doesn't exist), this
is reported by cinit before any program is run. cinit does not create or
otherwise manage users or groups.

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
fields. Dependant processes will only be started once all their
dependencies have terminated. Refer to other programs in the config via
their `name` field.

The direction of a dependency is taken from the view of the program currently
being configured. E.g. the configuration

```yaml
  - name: current
    before:
      - other
```

reads as "`current` runs before `other`". Similarly

```yaml
  - name: current
    after:
      - other
```

reads as "`current` runs after `other`"

If the dependencies form a cycle, this is reported before any process is
started and cinit terminates.

## Advanced Features

### Merging configuration

Each program is identified by a name. Several programs containing the same name
will be merged into a single program configuration. This is especially useful if
cinit is configured to read the configuration from a directory. The fields which
are allowed in more than only one place are listed below. All other fields are
only permitted in one location.

#### `env`

All list entries will be merged into a single list. The order of the sublists is
not defined. The order of entries within one list will be preserved. This has
some implications:

* When specifying duplicate keys, the value is taken from one of the duplicates
  but it's not defined, which one.

* When using a key from one location in the template of a different key, the
  result can be either the template without variable substitution or a
  successful substitution. It is not guaranteed to be consistent.

#### `before` and `after`

All list entries will be merged into a single list. The dependencies will be
treated according to this merged list. Duplicates are handled by cinit.

#### `capabilities`

All list entries will be merged into a single list. Capabilities will be granted
according to this merged list. Duplicates are handled by cinit.

#### `args`

All list entries will be merged into a single list. The list from the location
containing a `path` entry is always put first. Apart from that there are no
guarantees on the order of the lists. The order of entries within one sublist
will be preserved. Arguments can use environment variables from any location
specifying environment variables. The same implications about duplicate
environment variables as in `env` do apply.

#### `type`

Setting a type other than `oneshot` is only allowed in the list entry which also
contains the `path` (the so called primary list entry). If the primary is set to
a different value than `oneshot` it is not possible to change it from a
different list entry.

#### `pty`

The logical disjunction of all flags is computed, with `false` as the default if
none is given.

## Usage

```text
cinit 1.3.2
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

cinit combines the log output of children with its own. Child output is always
logged at INFO level. The log format is as follows:

`<TIMESTAMP> <LEVEL> [<NAME>] <MESSAGE>`

* `TIMESTAMP`: This follows the pattern `YYYY-MM-DD'T'HH:MM:SS.mmm`.
  Example: `1970-11-23T13:25:44.567`.

* `LEVEL`: One of `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`.

* `NAME`: This is either the string `cinit` or the name of a child as defined
  in the `name` attribute in the YAML config.

* `MESSAGE`: The actual event being reported.

Programs SHOULD log to the standard file descriptors and SHOULD NOT log own
timestamps.

Specifying `-v` twice gives messages up to the `TRACE` level. Tracing messages
are considered part of the public API. Specifying `-v` once gives messages up to
the `DEBUG` level. This is the expected level for bug reports. Production
installations should not specify the `-v` flag which gives messages up to the
`INFO` level.

## Status Reporting

While running cinit holds an open UNIX Domain Socket at `/run/cinit.socket`.
When connecting to the socket cinit reports information about its current
runtime status. The output format is similar to the configuration file format.

A program consists of the following runtime keys:

* `name`: The name of the program as given in the configuration file.

* `state`: The current status of the program. One of:

  * `blocked`: The program is currently waiting for its dependencies to
    terminate.
  * `sleeping`: The program is a cronjob waiting for the next scheduled
    execution.
  * `running`: The program is currently running.
  * `done`: The program has terminated successfully.
  * `crashed`: The program has terminated with an error code.

* `pid`: Optional. The process id of a running child.

* `scheduled_at`: Optional. Next execution time of a cronjob.

* `exit_code`: Optional. The returned exit code of a child that is `done` or
  `crashed`.

An example report looks like this:

```yaml
programs:
  - name: 'program 1'
    state: 'done'
    exit_code: 0
  - name: 'program 2'
    state: 'running'
    pid: 1409
  - name: 'cronjob'
    state: 'sleeping'
    scheduled_at: '2019-01-03T17:41:39'
```

## License

Copyright (C)  2019 The cinit developers.
Permission is granted to copy, distribute and/or modify this document
under the terms of the GNU Free Documentation License, Version 1.3
or any later version published by the Free Software Foundation;
with no Invariant Sections, no Front-Cover Texts, and no Back-Cover Texts.
A copy of the license is included alongside this document.
