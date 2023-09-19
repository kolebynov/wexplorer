use std::{error::Error};

use config::Config;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry, util::SubscriberInitExt, filter::LevelFilter, Layer};

pub fn init(config: &Config) -> Result<WorkerGuard, Box<dyn Error + Send + Sync>> {
    let file_appender = tracing_appender::rolling::hourly("temp/logs", "log.log");
    let (writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::Layer::default()
            .with_filter(LevelFilter::INFO))
        .with(tracing_subscriber::fmt::Layer::default()
            .with_ansi(false)
            .fmt_fields(tracing_subscriber::fmt::format::DefaultFields::new())
            .with_writer(writer)
            .with_filter(LevelFilter::INFO))
        .try_init()?;

    Ok(_guard)
}