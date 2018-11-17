use std::collections::HashMap;


#[derive(Debug, Deserialize, Clone, Copy)]
pub enum ProcessType {

    #[serde(rename = "oneshot")]
    Oneshot
}


#[derive(Debug, Deserialize)]
pub struct ProcessConfig {
    pub name: String,

    pub path: String,
    pub args: Vec<String>,

    #[serde(rename = "type")]
    pub process_type: ProcessType,

    pub uid: Option<u32>,
    pub gid: Option<u32>,

    pub user: Option<String>,
    pub group: Option<String>,

    #[serde(default)]
    pub before: Vec<String>,
    #[serde(default)]
    pub after: Vec<String>,

    #[serde(rename = "pty")]
    #[serde(default)]
    pub emulate_pty: bool,

    pub capabilities: Vec<String>,

    #[serde(default)]
    pub env: HashMap<String, Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub programs: Vec<ProcessConfig>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            programs: Vec::new()
        }
    }

    pub fn merge(&mut self, other: Config) {
        for program in other.programs {
            self.programs.push(program);
        }
    }
}

