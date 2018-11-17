use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::exit;
use std::os::unix::io::RawFd;
use std::os::unix::io::AsRawFd;

use runtime::process::{Process, ProcessState};

use nix::sys::epoll;
use nix::sys::signal;
use nix::sys::signalfd;
use nix::sys::wait;
use nix::unistd::Pid;
use nix::unistd::read;

#[derive(Debug)]
pub struct ProcessManager {
    pub processes: Vec<Process>,

    pub name_dict: HashMap<String, usize>,

    pub fd_dict: HashMap<RawFd, usize>,

    pub pid_dict: HashMap<Pid, usize>,

    pub keep_running: bool,

    pub runnable: VecDeque<usize>,

    pub running_count: u32,

    pub epoll_file: RawFd,

    pub signal_fd: signalfd::SignalFd,
}

impl ProcessManager {
    pub fn start(&mut self) {

        match self.setup() {
            Err(content) => {
                error!("Failed to register with epoll: {}", content);
                exit(3);
            },
            _ => {}
        }

        debug!("Entering poll loop");
        while self.keep_running &&
                    (self.running_count != 0 ||
                    self.runnable.len() != 0) {

            self.kick_off_children();
            self.dispatch_epoll();
            self.handle_finished_children();
        }

        self.handle_finished_children();
        info!("Exiting");
    }

    fn handle_finished_children(&mut self) {
        let mut wait_args = wait::WaitPidFlag::empty();
        wait_args.insert(wait::WaitPidFlag::WNOHANG);
        while let Ok(status) = wait::waitpid(Pid::from_raw(0), Some(wait_args)) {
            debug!("Got signal from child: {:?}", status);
            match status {
                wait::WaitStatus::Exited(pid, rc) => {
                    self.handle_finished_child(&pid, rc)
                },
                wait::WaitStatus::StillAlive => {
                    break;
                }
                _ => {}
            }
        }
    }

    fn handle_finished_child(&mut self, pid: &Pid, rc: i32) {
        self.running_count -= 1;
        let child_index = *self.pid_dict.get(&pid).expect("PID not found");
        {
            let child = &mut self.processes[child_index];
            info!("Child {} exited with {}", child.description.name, rc);
            child.description.state = if rc == 0 {
                ProcessState::Done
            } else {
                ProcessState::Crashed
            }
        }

        for successor_index in self.processes[child_index].node_info.before.clone() {
            let mut successor = &mut self.processes[successor_index];
            successor.node_info.predecessor_count -= 1;
            if successor.node_info.predecessor_count == 0 {
                self.runnable.push_back(successor_index);
            }
        }
    }

    fn dispatch_epoll(&mut self) {
        let mut event_buffer = [epoll::EpollEvent::empty(); 10];
        let epoll_result = epoll::epoll_wait(self.epoll_file, &mut event_buffer, 1000);
        match epoll_result {
            Ok(count) => {
                debug!("Got {} events", count);
                for i in 0..count {
                    let event = event_buffer[i];
                    self.handle_event(event);
                }
            },
            Err(_) => {}
        }
    }

    pub fn setup(&mut self) -> Result<(), nix::Error> {
        self.signal_fd = ProcessManager::setup_signal_handler()?;
        self.epoll_file = self.setup_epoll_fd()?;
        Ok(())
    }

    fn setup_signal_handler() -> Result<signalfd::SignalFd, nix::Error> {
        let mut signals = signalfd::SigSet::empty();
        signals.add(signalfd::signal::SIGCHLD);
        signals.add(signalfd::signal::SIGINT);
        signals.add(signalfd::signal::SIGTERM);
        signals.add(signalfd::signal::SIGQUIT);
        signal::sigprocmask(signal::SigmaskHow::SIG_BLOCK, Some(&signals), None)?;
        signalfd::SignalFd::with_flags(&signals, signalfd::SfdFlags::SFD_CLOEXEC)
    }

    fn setup_epoll_fd(&mut self) -> Result<RawFd, nix::Error> {
        let epoll_fd = epoll::epoll_create1(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        epoll::epoll_ctl(epoll_fd,
                         epoll::EpollOp::EpollCtlAdd,
                         self.signal_fd.as_raw_fd(),
                         &mut epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, self.signal_fd.as_raw_fd() as u64))?;
        Ok(epoll_fd)
    }

    pub fn handle_event(&mut self, event: epoll::EpollEvent) {
        if event.events().contains(epoll::EpollFlags::EPOLLIN) {
            let fd = event.data() as RawFd;
            if fd == self.signal_fd.as_raw_fd() {
                self.handle_signal();
            } else {
                self.handle_child_output(fd);
            }
        } else if event.events().contains(epoll::EpollFlags::EPOLLHUP) {
            self.deregister_fd(event);
        } else {
            warn!("Received unknown event");
        }
    }

    pub fn handle_signal(&mut self) {
        match self.signal_fd.read_signal() {
            Ok(Some(signal)) => {
                match signal::Signal::from_c_int(signal.ssi_signo as i32).unwrap() {
                    signal::SIGINT |
                    signal::SIGTERM |
                    signal::SIGQUIT => {
                        info!("Received termination signal, killing children");
                        self.keep_running = false;
                        for child in &self.processes {
                            if child.description.state == ProcessState::Running {
                                signal::kill(child.description.pid, signal::SIGTERM);
                            }
                        }
                    },
                    _ => {},
                }
            },
            _ => {}
        }
    }

    fn deregister_fd(&mut self, event: epoll::EpollEvent) {
        info!("client has closed fd");
        let fd = event.data();
        let epoll_result = epoll::epoll_ctl(self.epoll_file,
                                            epoll::EpollOp::EpollCtlDel,
                                            fd as RawFd,
                                            &mut epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, self.signal_fd.as_raw_fd() as u64));
        if epoll_result.is_err() {
            warn!("Could not unregister fd from epoll");
        }
        self.fd_dict.remove(&(fd as RawFd));
    }

    pub fn handle_child_output(&mut self, fd: RawFd) {
        let mut buffer = [0 as u8; 4096];
        read(fd, &mut buffer);

        let output = String::from_utf8_lossy(&buffer);
        let child_name = &self.processes[*self.fd_dict.get(&fd).expect("Invalid fd found")].description.name;

        info!("Child {}: {}", child_name, output);
    }

    fn kick_off_children(&mut self) {
        while ! self.runnable.is_empty() {
            let child_index = self.runnable.pop_back().unwrap();
            let child_pid = self.processes[child_index].start();
            self.pid_dict.insert(child_pid, child_index);
            self.running_count += 1;
        }
    }
}
