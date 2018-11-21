use std::collections::HashMap;
use std::ffi::CString;

use config;

use nix::unistd::Gid;
use nix::unistd::Pid;
use nix::unistd::Uid;

use super::process::{Process, ProcessState};

impl Process {
    pub fn from(config: &config::config::ProcessConfig) -> Process {
        let mut result = Process {
            name: config.name.to_owned(),
            path: config.path.to_owned(),
            args: Vec::new(),
            process_type: config.process_type,
            uid: Uid::from_raw(map_unix_name(&config.uid, &config.user, &config.name)),
            gid: Gid::from_raw(map_unix_name(&config.gid, &config.group, &config.name)),
            emulate_pty: config.emulate_pty,
            capabilities: config.capabilities.to_owned(),
            env: convert_env(&config.env),
            state: ProcessState::Blocked,
            pid: Pid::from_raw(0),
        };

        result.args.push(CString::new(result.path.clone()).unwrap());

        result.args.append(
            &mut config
                .args
                .iter()
                .map(|x| CString::new(x.clone()).unwrap())
                .collect(),
        );

        result
    }
}

/// Can be used to get either user id or group id
fn map_unix_name(id: &Option<u32>, name: &Option<String>, process: &String) -> u32 {
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

fn convert_env(env: &Vec<HashMap<String, Option<String>>>) -> Vec<CString> {
    let mut result = get_default_env();
    result = copy_from_config(env, result);
    flatten_to_strings(&mut result)
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
                    Err(_) => {
                        result.insert(key.to_string(), String::from(""));
                    }
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

fn flatten_to_strings(result: &mut HashMap<String, String>) -> Vec<CString> {
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
