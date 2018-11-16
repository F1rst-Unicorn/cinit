use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::exit;

use config;
use config::process_tree::Config;

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


    Backoff,
}

pub struct Process {
    description: ProcessDescription,

    node_info: ProcessNode,
}

impl Process {
    pub fn start(&mut self) {
        self.description.start();
    }
}

pub struct ProcessDescription {
    name: String,

    path: String,

    args: Vec<String>,

    process_type: config::process_tree::ProcessType,

    uid: u32,

    gid: u32,

    emulate_pty: bool,

    capabilities: Vec<String>,

    env: HashMap<String, String>,

    state: ProcessState
}

impl ProcessDescription {
    pub fn from(config: &config::process_tree::ProcessConfig) -> ProcessDescription {
        ProcessDescription {
            name: config.name.to_owned(),
            path: config.path.to_owned(),
            args: config.args.to_owned(),
            process_type: config.process_type,
            uid: map_unix_name(&config.uid, &config.user, &config.name),
            gid: map_unix_name(&config.gid, &config.group, &config.name),
            emulate_pty: config.emulate_pty,
            capabilities: config.capabilities.to_owned(),
            env: convert_env(&config.env),
            state: ProcessState::Blocked,
        }
    }

    pub fn start(&mut self) {
        self.state = ProcessState::Running;
        info!("Starting {}", self.name);
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

fn convert_env(env: &HashMap<String, Option<String>>) -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();
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
    result
}

pub struct ProcessNode {
    after: Vec<usize>,

    predecessor_count: usize,
}

pub struct ProcessManager {
    processes: Vec<Process>,

    name_dict: HashMap<String, usize>,

    runnable: VecDeque<usize>,

    running_count: u32,
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
            runnable,
            running_count: 0,
        }
    }

    pub fn start(&mut self) {

        while self.running_count != 0 || ! self.runnable.is_empty() {
            self.kick_off_children();

            // do epoll stuff

        }

        info!("Nothing running and no processes left to start");
        info!("Exiting");
    }

    fn kick_off_children(&mut self) {
        while ! self.runnable.is_empty() {
            let child_index = self.runnable.pop_back().unwrap();
            self.processes[child_index].start();
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

        for i in 0..config.programs.len() {
            result.push(ProcessNode {
                after: Vec::new(),
                predecessor_count: 0,
            });
        }

        for process_config in &config.programs {
            let current_index = name_dict.get(&process_config.name).expect("Invalid index in name_dict").clone();
            {
                let mut current = result.get_mut(current_index).expect("Invalid index in name_dict");
                for successor in &process_config.after {
                    let dependant_index = name_dict.get(successor).expect("Invalid index in name_dict").clone();
                    current.after.push(dependant_index);
                }

                current.predecessor_count += process_config.before.len();

            }

            for successor in &process_config.after {
                let dependant_index = name_dict.get(successor).expect("Invalid index in name_dict").clone();
                let mut dependant = result.get_mut(dependant_index).expect("Invalid index in name_dict");
                dependant.predecessor_count += 1;
            }

            for predecessor in &process_config.before {
                let dependency_index = name_dict.get(predecessor).expect("Invalid index in name_dict").clone();
                let mut dependency = result.get_mut(dependency_index).expect("Invalid index in name_dict");
                dependency.after.push(current_index);
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
