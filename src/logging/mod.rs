pub mod stderr;
pub mod stdout;

use log4rs;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;

use log::LevelFilter;

pub fn initialise(verbosity_level: u64) {
    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%dT%H:%M:%S.%3f)} {level} [cinit] {m}{n}",
        )))
        .build();

    let child_stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%dT%H:%M:%S.%3f)} {level} {m}{n}",
        )))
        .build();

    let child_stderr = log4rs::append::console::ConsoleAppender::builder()
        .target(log4rs::append::console::Target::Stderr)
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%dT%H:%M:%S.%3f)} {level} {m}{n}",
        )))
        .build();

    let level = match verbosity_level {
        2 => LevelFilter::Trace,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Info,
    };

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("stderr_child", Box::new(child_stderr)))
        .appender(Appender::builder().build("stdout_child", Box::new(child_stdout)))
        .logger(
            Logger::builder()
                .additive(false)
                .appender("stdout_child")
                .build("cinit::logging::stdout", LevelFilter::Info),
        )
        .logger(
            Logger::builder()
                .additive(false)
                .appender("stderr_child")
                .build("cinit::logging::stderr", LevelFilter::Info),
        )
        .build(Root::builder().appender("stdout").build(level))
        .expect("Could not configure logging");

    log4rs::init_config(config).expect("Could not apply log config");
}
