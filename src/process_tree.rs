use std::collections::HashMap;


#[derive(Debug, Deserialize)]
pub enum ProcessType {

    #[serde(rename = "oneshot")]
    Oneshot
}


#[derive(Debug, Deserialize)]
pub struct Process {
    name: String,

    path: String,
    args: Vec<String>,

    #[serde(rename = "type")]
    process_type: ProcessType,

    uid: Option<u32>,
    gid: Option<u32>,

    user: Option<String>,
    group: Option<String>,

    #[serde(default)]
    before: Vec<String>,
    #[serde(default)]
    after: Vec<String>,

    #[serde(rename = "pty")]
    #[serde(default)]
    emulate_pty: bool,

    capabilities: Vec<String>,

    #[serde(default)]
    env: HashMap<String, Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ProcessTree {
    programs: Vec<Process>,
}

impl ProcessTree {
    pub fn new() -> ProcessTree {
        ProcessTree {
            programs: Vec::new()
        }
    }

    pub fn merge(&mut self, other: ProcessTree) {
        for program in other.programs {
            self.programs.push(program);
        }
    }

    pub fn start(&self) {

    }
}

