use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::process::exit;

use serde_yaml;

use config::config::Config;

pub fn parse_config(path: &str) -> Config {
    let raw_config = read_config(path);
    debug!(
        "Complete configuration:\n{}",
        raw_config
            .iter()
            .flat_map(|s| s.chars())
            .collect::<String>()
    );
    parse_raw_config(raw_config)
}

fn parse_raw_config(raw_config: Vec<String>) -> Config {
    raw_config
        .iter()
        .map(|s| serde_yaml::from_str(s).expect("Failed to parse config"))
        .fold(Config::new(), Config::merge)
}

pub fn read_config(path: &str) -> Vec<String> {
    let metadata_result = fs::metadata(path);

    if metadata_result.is_err() {
        error!(
            "Failed to read metadata of {}: {}",
            path,
            metadata_result.unwrap_err()
        );
        exit(1);
    }

    let mut result: Vec<String>;
    let metadata = metadata_result.unwrap();

    if metadata.file_type().is_dir() {
        let content = fs::read_dir(path);
        if content.is_err() {
            error!(
                "Failed to get directory content of {}: {}",
                path,
                content.unwrap_err()
            );
            exit(1);
        }

        result = Vec::new();

        for entry in content.unwrap() {
            if entry.is_err() {
                error!("Failed to read {}: {}", path, entry.unwrap_err());
                exit(1);
            }
            let entry_path = entry.unwrap().path();
            let entry_path_string = entry_path.to_str().unwrap();
            let content = read_config(entry_path_string);

            result.extend(content);
        }
    } else if metadata.file_type().is_file() {
        match read_file(path) {
            Err(error) => {
                error!("Failed to read file {}: {}", path, error);
                exit(1);
            }
            Ok(content) => {
                result = vec![content];
            }
        }
    } else {
        warn!("Ignoring file {}", path);
        result = Vec::new();
    }

    result
}

pub fn read_file(file_path: &str) -> Result<String, io::Error> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::super::config::ProcessType;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parse_single_program() {
        let mut expected_env = HashMap::new();
        expected_env.insert("key".to_owned(), Some("value".to_owned()));
        expected_env.insert("empty_key".to_owned(), None);

        let output = parse_raw_config(vec![FULL_CONFIG.to_owned()]);

        assert_eq!(1, output.programs.len());

        let program = &output.programs[0];
        assert_eq!("test", program.name);
        assert_eq!("/some/path", program.path);
        assert_eq!(Vec::new() as Vec<String>, program.args);
        assert_eq!(ProcessType::Oneshot, program.process_type);
        assert_eq!(Some(3), program.uid);
        assert_eq!(Some(1), program.gid);
        assert_eq!(Some("root".to_owned()), program.user);
        assert_eq!(Some("group".to_owned()), program.group);
        assert_eq!(vec!["ever"], program.before);
        assert_eq!(vec!["after"], program.after);
        assert_eq!(false, program.emulate_pty);
        assert_eq!(vec!["some"], program.capabilities);
        assert_eq!(expected_env, program.env);
    }

    #[test]
    fn parse_omitting_all_optional_values() {
        let expected_env = HashMap::new();

        let output = parse_raw_config(vec![MINIMAL_CONFIG.to_owned()]);

        assert_eq!(1, output.programs.len());

        let program = &output.programs[0];
        assert_eq!("test", program.name);
        assert_eq!("/path", program.path);
        assert_eq!(Vec::new() as Vec<String>, program.args);
        assert_eq!(ProcessType::Oneshot, program.process_type);
        assert_eq!(None, program.uid);
        assert_eq!(None, program.gid);
        assert_eq!(None, program.user);
        assert_eq!(None, program.group);
        assert_eq!(Vec::new() as Vec<String>, program.before);
        assert_eq!(Vec::new() as Vec<String>, program.after);
        assert_eq!(false, program.emulate_pty);
        assert_eq!(Vec::new() as Vec<String>, program.capabilities);
        assert_eq!(expected_env, program.env);
    }

    const MINIMAL_CONFIG: &str = "\
programs:
  - name: test
    path: /path
";

    const FULL_CONFIG: &str = "\
programs:
  - name: test
    path: /some/path
    args: []
    type: oneshot
    uid: 3
    gid: 1
    user: root
    group: group
    before:
      - ever
    after:
      - after
    emulate_pty: false
    capabilities:
      - some
    env:
      key: value
      empty_key:
";

}
