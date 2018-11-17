use nix::pty;

use libc;

ioctl_read_bad!(get_terminal_size, libc::TIOCGWINSZ, pty::Winsize);