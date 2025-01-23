use std::fmt::Display;

use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet};
use serde::{de::DeserializeOwned, Serialize};

pub fn update_config<T>(config: Config, key: &str, value: T)
where
    T: Serialize + Display + Clone,
{
    let config_set = config.set(key, value.clone());

    match config_set {
        Ok(_) => println!("Config variable for {key} was set to {value}"),
        Err(e) => eprintln!("Something went wrong setting {key} to {value}: {e}"),
    }

    let config_tx = config.transaction();
    let tx_result = config_tx.commit();

    match tx_result {
        Ok(_) => println!("Config transaction has been completed!"),
        Err(e) => eprintln!("Something with the config transaction when wrong: {e}"),
    }
}

pub fn load_config<T>(key: &str, config_vers: u64) -> (Option<T>, String)
where
    T: DeserializeOwned,
{
    let config = match Config::new("com.championpeak87.cosmic-classic-menu", config_vers) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Loading config file had an error: {e}");
            Config::system("com.championpeak87.cosmic-classic-menu", 1).unwrap()
        }
    };

    match config.get(key) {
        Ok(value) => (Some(value), "".to_owned()),
        Err(_e) => {
            update_config(config, key, "");
            (None, "Created config for key".to_owned())
        }
    }
}
