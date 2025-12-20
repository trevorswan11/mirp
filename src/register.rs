use std::{env, fs::File, io::Write};

use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

use anyhow::{Result, anyhow};
use chrono::Local;
use dotenvy::dotenv;

use crate::proxy::ProxyConfig;

pub fn env() -> Result<ProxyConfig> {
    dotenv()
        .inspect_err(|_| {
            File::create_new(".env")
                .unwrap()
                .write(
                    b"TARGET_DOMAIN=\"mc.yourdomain.com\"\n\
                    PUBLIC_ADDRESS=\"0.0.0.0:12345\"\n\
                    LOCAL_ADDRESS=\"127.0.0.1:25565\"\n\
                    BLACKLIST_PATH=\"mirp.blacklist\"",
                )
                .unwrap();

            eprintln!("I couldn't find a .env file so I made one for you with placeholder values!");
        })
        .map_err(|_| anyhow!("Failed to load .env"))?;

    Ok(ProxyConfig {
        target_domain: env::var("TARGET_DOMAIN")?,
        public_address: env::var("PUBLIC_ADDRESS")?,
        local_address: env::var("LOCAL_ADDRESS")?,
    })
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
