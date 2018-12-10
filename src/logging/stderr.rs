use log::info;


pub fn log(child_name: &str, message: &str) {
    info!("[{}] {}", child_name, message);
}
