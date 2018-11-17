use std::collections::HashMap;
use std::process::exit;
use std::ffi::CString;

use config;

use nix::unistd;
use nix::unistd::Pid;
use nix::unistd::fork;

#[derive(Debug, PartialEq)]
pub enum ProcessState {

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
    pub description: ProcessDescription,

    pub node_info: ProcessNode,
}

impl Process {
    pub fn start(&mut self) -> Pid {
        self.description.start()
    }
}

#[derive(Debug)]
pub struct ProcessDescription {
    pub name: String,

    pub path: String,

    pub args: Vec<CString>,

    pub process_type: config::config::ProcessType,

    pub uid: u32,

    pub gid: u32,

    pub emulate_pty: bool,

    pub capabilities: Vec<String>,

    pub env: Vec<CString>,

    pub state: ProcessState,

    pub pid: Pid,
}

impl ProcessDescription {
    pub fn from(config: &config::config::ProcessConfig) -> ProcessDescription {
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

/// Process information relevant for dependency resolution
/// via ongoing topological sorting
#[derive(Debug)]
pub struct ProcessNode {
    pub before: Vec<usize>,

    pub predecessor_count: usize,
}
