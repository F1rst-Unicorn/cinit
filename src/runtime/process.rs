use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::exit;
use std::os::unix::io::RawFd;
use std::os::unix::io::AsRawFd;
use std::ffi::CString;

use config;
use config::process_tree::Config;

use nix::sys::epoll;
use nix::sys::signal;
use nix::sys::signalfd;
use nix::sys::wait;
use nix::unistd;
use nix::unistd::Pid;
use nix::unistd::read;
use nix::unistd::fork;

#[derive(Debug, PartialEq)]
enum ProcessState {

    /// The process cannot be started because of dependencies not having
    /// finished yet.
    Blocked,

    /// The process has no more dependencies and can be started
    Ready,

    /// The process is running
    Running,

    /// The process has finished successfully
    Done,

    /// The process has finished unsucessfully
    Crashed,
}

#[derive(Debug)]
pub struct Process {
    description: ProcessDescription,

    node_info: ProcessNode,
}

impl Process {
    pub fn start(&mut self) -> Pid {
        self.description.start()
    }
}

#[derive(Debug)]
pub struct ProcessDescription {
    name: String,

    path: String,

    args: Vec<CString>,

    process_type: config::process_tree::ProcessType,

    uid: u32,

    gid: u32,

    emulate_pty: bool,

    capabilities: Vec<String>,

    env: Vec<CString>,

    state: ProcessState,

    pid: Pid,
}

impl ProcessDescription {
    pub fn from(config: &config::process_tree::ProcessConfig) -> ProcessDescription {
        let mut result = ProcessDescription {
            name: config.name.to_owned(),
            path: config.path.to_owned(),
            args: Vec::new(),
            process_type: config.process_type,
            uid: map_unix_name(&config.uid, &config.user, &config.name),
            gid: map_unix_name(&config.gid, &config.group, &config.name),
            emulate_pty: config.emulate_pty,
            capabilities: config.capabilities.to_owned(),
            env: convert_env(&config.env),
            state: ProcessState::Blocked,
            pid: Pid::from_raw(0),
        };

        result.args.push(CString::new(result.path.clone()).unwrap());

        result.args.append(
                &mut config.args.iter().map(|x| CString::new(x.clone()).unwrap()).collect());

        result
    }

    pub fn start(&mut self) -> Pid {
        info!("Starting {}", self.name);

        let fork_result = fork();

        match fork_result {
            Ok(unistd::ForkResult::Parent {child: child_pid}) => {
                self.state = ProcessState::Running;
                self.pid = child_pid;
                child_pid
            },
            Ok(unistd::ForkResult::Child) => {
                self.setup_child()
            },
            _ => {
                error!("Forking failed");
                exit(4)
            }
        }
    }

    fn setup_child(&mut self) -> ! {
        unistd::setuid(unistd::Uid::from_raw(self.uid));
        unistd::setgid(unistd::Gid::from_raw(self.gid));

        let result = unistd::execve(&CString::new(self.path.to_owned()).unwrap(),
                                    self.args.as_slice(),
                                    self.env.as_slice());

        match result {
            Ok(_) => {
                assert!(false, "exec() was successful but did not replace program");
                exit(1);
            }
            Err(_) => {
                error!("Could not exec child {}", self.name);
                exit(0);
            }
        }
    }

}

/// Can be used to get either user id or group id
fn map_unix_name(id: &Option<u32>,
                 name: &Option<String>,
                 process: &String) -> u32 {

    if id.is_some() && name.is_some() {
        warn!("Both id and name set for {}, taking only id", process);
        id.unwrap()
    } else if id.is_some() && name.is_none() {
        id.unwrap()
    } else if id.is_none() && name.is_some() {
        // Depends on https://github.com/nix-rust/nix/pull/864
        panic!("name not supported as of now!");
    } else {
        warn!("Neither user nor id given for {}, using root (0)", process);
        0
    }
}

fn convert_env(env: &HashMap<String, Option<String>>) -> Vec<CString> {
    let mut result: HashMap<String, String> = HashMap::new();
    let default_env = ["HOME", "LANG", "LANGUAGE", "LOGNAME", "PATH",
                       "PWD", "SHELL", "TERM", "USER"];

    for key in default_env.iter() {
        match std::env::var(key) {
            Err(_) => {
                result.insert(key.to_string(), String::from(""));
            },
            Ok(real_value) => {
                result.insert(key.to_string(), real_value);
            }
        }
    }

    for (key, value) in env {
        match value {
            None => {
                match std::env::var(key) {
                    Err(_) => {
                        result.insert(key.to_string(), String::from(""));
                    },
                    Ok(real_value) => {
                        result.insert(key.to_string(), real_value);
                    }
                }
            },
            Some(real_value) => {
                result.insert(key.to_string(), real_value.to_string());
            }
        }
    }

    let mut ret: Vec<CString> = Vec::new();

    for (key, value) in result.iter() {
        let entry = key.to_owned() + "=" + value;
        ret.push(CString::new(entry).unwrap());
    }
    ret
}

#[derive(Debug)]
pub struct ProcessNode {
    before: Vec<usize>,

    predecessor_count: usize,
}

#[derive(Debug)]
pub struct ProcessManager {
    processes: Vec<Process>,

    name_dict: HashMap<String, usize>,

    fd_dict: HashMap<RawFd, usize>,

    pid_dict: HashMap<Pid, usize>,

    keep_running: bool,

    runnable: VecDeque<usize>,

    running_count: u32,

    epoll_file: RawFd,

    signal_fd: signalfd::SignalFd,
}

impl ProcessManager {
    pub fn from(config: Config) -> ProcessManager {

        let descriptions = ProcessManager::copy_processes(&config);

        let name_dict = ProcessManager::build_name_dict(&descriptions);

        let nodes = ProcessManager::build_dependencies(&config, &name_dict);

        let mut processes = ProcessManager::merge(descriptions, nodes);

        let runnable = ProcessManager::find_runnables(&mut processes);

        if runnable.len() == 0 {
            error!("No runnable processes found, check for cycles");
            exit(2);
        }

        ProcessManager {
            processes,
            name_dict,
            fd_dict: HashMap::new(),
            pid_dict: HashMap::new(),
            keep_running: true,
            runnable,
            running_count: 0,
            epoll_file: -1,
            signal_fd: signalfd::SignalFd::new(&signalfd::SigSet::empty()).unwrap(),
        }
    }

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



    fn copy_processes(config: &Config) -> Vec<ProcessDescription> {
        let mut result = Vec::with_capacity(config.programs.len());
        for program in &config.programs {
            result.push(ProcessDescription::from(program));
        }
        result
    }

    fn build_name_dict(descriptions: &Vec<ProcessDescription>) -> HashMap<String, usize> {
        let mut result = HashMap::new();

        for (i, desc) in descriptions.into_iter().enumerate() {
            if result.contains_key(&desc.name) {
                error!("Duplicate program found for name {}", &desc.name);
                exit(2);
            } else {
                result.insert(desc.name.to_owned(), i);
            }
        }

        result
    }

    fn build_dependencies(config: &Config, name_dict: &HashMap<String, usize>) -> Vec<ProcessNode> {
        let mut result = Vec::with_capacity(config.programs.len());

        for _ in 0..config.programs.len() {
            result.push(ProcessNode {
                before: Vec::new(),
                predecessor_count: 0,
            });
        }

        for process_config in &config.programs {
            let current_index = name_dict.get(&process_config.name).expect("Invalid index in name_dict").clone();
            {
                let mut current = result.get_mut(current_index).expect("Invalid index in name_dict");
                for predecessor_name in &process_config.before {
                    let predecessor_index = name_dict.get(predecessor_name).expect("Invalid index in name_dict").clone();
                    current.before.push(predecessor_index);
                }

                current.predecessor_count += process_config.after.len();

            }

            for predecessor_name in &process_config.before {
                let predecessor_index = name_dict.get(predecessor_name).expect("Invalid index in name_dict").clone();
                let mut predecessor = result.get_mut(predecessor_index).expect("Invalid index in name_dict");
                predecessor.predecessor_count += 1;
            }

            for predecessor in &process_config.after {
                let dependency_index = name_dict.get(predecessor).expect("Invalid index in name_dict").clone();
                let mut dependency = result.get_mut(dependency_index).expect("Invalid index in name_dict");
                dependency.before.push(current_index);
            }
        }

        result
    }

    fn find_runnables(processes: &mut Vec<Process>) -> VecDeque<usize> {
        let mut result = VecDeque::new();
        for (i, process) in processes.iter_mut().enumerate() {
            if process.node_info.predecessor_count == 0 {
                result.push_back(i);
                process.description.state = ProcessState::Ready;
            }
        }
        result
    }

    fn merge(descriptions: Vec<ProcessDescription>, nodes: Vec<ProcessNode>) -> Vec<Process> {
        let mut result = Vec::with_capacity(descriptions.len());
        for (desc, node) in descriptions.into_iter().zip(nodes.into_iter()) {
            result.push(Process {
                description: desc,
                node_info: node,
            })
        }
        result
    }
}
