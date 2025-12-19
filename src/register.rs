use std::{env, fs::File, io::Write};

use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

use anyhow::{Result, anyhow};
use chrono::Local;
use dotenvy::dotenv;

/// Returns the three key environment variables as:
///
/// (TARGET_DOMAIN, PUBLIC_ADDRESS, LOCAL_ADDRESS)
pub fn env() -> Result<(String, String, String)> {
    dotenv()
        .inspect_err(|_| {
            let mut new_env =
                File::create_new(".env").expect("Environment file exists but could not be read");
            new_env
                .write(
                    b"TARGET_DOMAIN=\"mc.yourdomain.com\"\n\
                    PUBLIC_ADDRESS=\"0.0.0.0:12345\"\n\
                    LOCAL_ADDRESS=\"127.0.0.1:25565\""
                )
                .expect("Failed to write to .env");

            eprintln!("I couldn't find a .env file so I made one for you with placeholder values!");
        })
        .map_err(|_| anyhow!("Failed to load .env"))?;

    Ok((
        env::var("TARGET_DOMAIN").expect("Failed to load TARGET_DOMAIN var"),
        env::var("PUBLIC_ADDRESS").expect("Failed to load PUBLIC_ADDRESS var"),
        env::var("LOCAL_ADDRESS").expect("Failed to load LOCAL_ADDRESS var"),
    ))
}

// Initializes the application-wide logger for use with log macros
pub fn logger() -> Result<()> {
    let date = Local::now().format("%Y-%m-%d");
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] - {m}{n}",
        )))
        .append(false)
        .build(format!("mirp-{}.log", date))?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(log::LevelFilter::Info),
        )?;
    log4rs::init_config(config)?;
    Ok(())
}
