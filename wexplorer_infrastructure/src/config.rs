pub use config::*;

const ENVIRONMENT_KEY: &str = "rust_app_environment";
const DEFAULT_ENVIRONMENT: &str = "dev";

pub fn load_default_configuration() -> Result<Config, ConfigError> {
    let temp_config = Config::builder()
        .add_source(Environment::default())
        .build()?;

    let env = temp_config
        .get_string(ENVIRONMENT_KEY)
        .or_else(|err| if matches!(err, ConfigError::NotFound(_)) { Ok(DEFAULT_ENVIRONMENT.to_string()) } else { Err(err) })?;

    Config::builder()
        .add_source(File::with_name("app_settings").required(true))
        .add_source(File::with_name(&format!("app_settings.{}", env)).required(false))
        .add_source(Environment::default())
        .build()
}

#[cfg(test)]
mod tests {
    use std::env;

    use crate::config::ENVIRONMENT_KEY;

    use super::load_default_configuration;

    #[test]
    fn test() {
        let config1 = load_default_configuration().unwrap();

        env::set_var(ENVIRONMENT_KEY, "prod");

        let config2 = load_default_configuration().unwrap();

        println!("{:#?}", config1);
        println!("{:#?}", config2);
    }
}