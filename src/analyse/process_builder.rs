use std::collections::HashMap;
use std::ffi::CString;
use std::fmt::Display;
use std::fmt::Error as FmtError;
use std::fmt::Formatter;
use std::path::PathBuf;

use config::config::{ProcessConfig, ProcessType};
use runtime::process::{Process, ProcessState};
use util::libc_helpers;

use nix::unistd::Gid;
use nix::unistd::Pid;
use nix::unistd::Uid;

#[derive(Debug)]
pub enum Error {
    CronjobDependency,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        let message = match self {
            Error::CronjobDependency => "Cronjobs may not have dependencies",
        };

        write!(f, "{}", message)
    }
}

impl Process {
    pub fn from(config: &ProcessConfig) -> Result<Process, Error> {
        let env = convert_env(&config.env);

        if let ProcessType::CronJob { .. } = &config.process_type {
            if !config.before.is_empty() || !config.after.is_empty() {
                return Err(Error::CronjobDependency);
            }
        }

        let mut result = Process {
            name: config.name.to_owned(),
            path: config.path.to_owned(),
            args: Vec::new(),
            workdir: PathBuf::from(match &config.workdir {
                None => ".",
                Some(path) => path,
            }),
            uid: Uid::from_raw(map_uid(&config.uid, &config.user, &config.name)),
            gid: Gid::from_raw(map_gid(&config.gid, &config.group, &config.name)),
            emulate_pty: config.emulate_pty,
            capabilities: config.capabilities.to_owned(),
            env: flatten_to_strings(&env),
            state: match config.process_type {
                ProcessType::Oneshot => ProcessState::Blocked,
                ProcessType::CronJob { .. } => ProcessState::Sleeping,
            },
            pid: Pid::from_raw(0),
        };

        result.args.push(CString::new(result.path.clone()).unwrap());

        result.args.append(
            &mut config
                .args
                .iter()
                .map(|x| render_template(&env, x).unwrap_or(x.clone()))
                .map(|x| CString::new(x.clone()).unwrap())
                .collect(),
        );

        Ok(result)
    }
}

fn map_uid(id: &Option<u32>, name: &Option<String>, process: &String) -> u32 {
    map_unix_name(id, name, process, &libc_helpers::user_to_uid)
}

fn map_gid(id: &Option<u32>, name: &Option<String>, process: &String) -> u32 {
    map_unix_name(id, name, process, &libc_helpers::group_to_gid)
}

/// Can be used to get either user id or group id
fn map_unix_name<T>(id: &Option<u32>, name: &Option<String>, process: &str, mapper: &T) -> u32
where
    T: Fn(&str) -> nix::Result<u32>,
{
    if id.is_some() && name.is_some() {
        warn!("Both id and name set for {}, taking only id", process);
        id.unwrap()
    } else if id.is_some() && name.is_none() {
        id.unwrap()
    } else if id.is_none() && name.is_some() {
        let mapped = mapper(name.as_ref().unwrap());
        match mapped {
            Ok(id) => id,
            Err(error) => {
                warn!(
                    "Name {} is not valid in program {}: {}",
                    name.as_ref().unwrap(),
                    process,
                    error
                );
                warn!("Using root(0)");
                0
            }
        }
    } else {
        warn!("Neither name nor id given for {}, using root (0)", process);
        0
    }
}

fn convert_env(env: &Vec<HashMap<String, Option<String>>>) -> HashMap<String, String> {
    let result = get_default_env();
    copy_from_config(env, result)
}

fn get_default_env() -> HashMap<String, String> {
    let mut result: HashMap<String, String> = HashMap::new();
    let default_env = [
        "HOME", "LANG", "LANGUAGE", "LOGNAME", "PATH", "PWD", "SHELL", "TERM", "USER",
    ];
    for key in default_env.iter() {
        match std::env::var(key) {
            Err(_) => {
                result.insert(key.to_string(), String::from(""));
            }
            Ok(real_value) => {
                result.insert(key.to_string(), real_value);
            }
        }
    }
    result
}

fn copy_from_config(
    env: &Vec<HashMap<String, Option<String>>>,
    mut result: HashMap<String, String>,
) -> HashMap<String, String> {
    for entry in env {
        for (key, value) in entry {
            match value {
                None => match std::env::var(key) {
                    Err(_) => {}
                    Ok(real_value) => {
                        result.insert(key.to_string(), real_value);
                    }
                },
                Some(raw_value) => {
                    let rendered_value =
                        render_template(&result, raw_value).unwrap_or(raw_value.to_string());
                    result.insert(key.to_string(), rendered_value);
                }
            }
        }
    }
    result
}

fn flatten_to_strings(result: &HashMap<String, String>) -> Vec<CString> {
    let mut ret: Vec<CString> = Vec::new();
    for (key, value) in result.iter() {
        let entry = key.to_owned() + "=" + value;
        ret.push(CString::new(entry).unwrap());
    }
    ret
}

fn render_template(context: &HashMap<String, String>, raw_value: &str) -> Result<String, ()> {
    let mut tera: tera::Tera = Default::default();
    let mut internal_context = tera::Context::new();
    let name = "name";
    tera.add_raw_template(name, raw_value).map_err(|_| ())?;
    for (key, value) in context {
        internal_context.insert(key, value);
    }
    tera.render(name, &context).map_err(|_| ())
}
