use std::process::exit;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io;

use serde_yaml;

use config::process_tree::Config;


pub fn parse_config(path: &str) -> Config {
    debug!("Complete configuration:");
    read_config(path)
}


pub fn read_config(path: &str) -> Config {
    let metadata_result = fs::metadata(path);

    if metadata_result.is_err() {
        error!("Failed to read metadata of {}: {}",
                path,
                metadata_result.unwrap_err());
        exit(1);
    }

    let mut configuration: Config;
    let metadata = metadata_result.unwrap();

    if metadata.file_type().is_dir() {

        let content = fs::read_dir(path);
        if content.is_err() {
            error!("Failed to get directory content of {}: {}",
                    path,
                    content.unwrap_err());
            exit(1);
        }

        configuration = Config::new();

        for entry in content.unwrap() {
            if entry.is_err() {
                error!("Failed to read {}: {}",
                        path,
                        entry.unwrap_err());
                exit(1);
            }
            let entry_path = entry.unwrap().path();
            let entry_path_string = entry_path.to_str().unwrap();
            configuration.merge(read_config(entry_path_string));
        }

    } else if metadata.file_type().is_file() {
        match read_file(path) {
            Err(error) => {
                error!("Failed to read file {}: {}", path, error);
                exit(1);
            },
            Ok(result) => {
                configuration = result;
            }
        }
    } else {
        warn!("Ignoring file {}", path);
        configuration = Config::new();
    }

    configuration
}

pub fn read_file(file_path: &str) -> Result<Config, io::Error> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    debug!("\n{}", content);
    let result = serde_yaml::from_str(&content)
            .expect(&format!("Failed to parse {}", file_path));
    Ok(result)
}
