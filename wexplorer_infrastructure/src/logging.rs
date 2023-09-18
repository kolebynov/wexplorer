use std::error::Error;

use config::Config;
use tracing::Level;
use tracing_log::LogTracer;

pub fn init(config: &Config) -> Result<(), Box<dyn Error>> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt().with_thread_ids(true).with_target(false).with_max_level(Level::INFO).finish())?;
    LogTracer::init()?;
    Ok(())
}