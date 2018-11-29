use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum ProcessType {
    #[serde(rename = "oneshot")]
    Oneshot,

    #[serde(rename = "service")]
    Service,

    #[serde(rename = "cronjob")]
    CronJob {timer: String},
}

fn default_process_type() -> ProcessType {
    ProcessType::Oneshot
}

#[derive(Debug, Deserialize)]
pub struct ProcessConfig {
    pub name: String,

    pub path: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default)]
    pub workdir: Option<String>,

    #[serde(rename = "type")]
    #[serde(default = "default_process_type")]
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

    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub env: Vec<HashMap<String, Option<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub programs: Vec<ProcessConfig>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            programs: Vec::new(),
        }
    }

    pub fn merge(mut self, mut other: Self) -> Self {
        self.programs.append(&mut other.programs);
        self
    }
}
