use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Result, Write},
    path::PathBuf,
};

use crate::{hotkey::Hotkey, input::TypingMethod, platform::get_home_dir};
use once_cell::sync::Lazy;

pub static HOTKEY_CONFIG: Lazy<Hotkey> = Lazy::new(|| Hotkey::from("super+ctrl+space"));

struct ConfigStore {
    data: HashMap<String, String>,
}

impl ConfigStore {
    fn get_config_path() -> PathBuf {
        get_home_dir()
            .expect("Cannot read home directory!")
            .join(".goxkey")
    }

    fn load_config_data() -> Result<HashMap<String, String>> {
        let mut data = HashMap::new();
        let config_path = ConfigStore::get_config_path();
        let mut file = File::open(config_path.as_path())?;
        let mut buf = String::new();
        file.read_to_string(&mut buf);
        buf.lines().for_each(|line| {
            if let Some((key, value)) = line.split_once('=') {
                data.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        });
        Ok(data)
    }

    fn write_config_data(data: &HashMap<String, String>) -> Result<()> {
        let config_path = ConfigStore::get_config_path();
        let mut file = File::create(config_path.as_path())?;
        let mut content = String::new();
        for (key, value) in data {
            content.push_str(&format!("{} = {}\n", key, value));
        }
        file.write_all(content.as_bytes())
    }

    pub fn new() -> Self {
        Self {
            data: ConfigStore::load_config_data().expect("Cannot read config file!"),
        }
    }

    pub fn read(&self, key: &str) -> String {
        return self.data.get(key).unwrap_or(&String::new()).to_string();
    }

    pub fn write(&mut self, key: &str, value: &str) {
        self.data.insert(key.to_string(), value.to_string());
        ConfigStore::write_config_data(&self.data).expect("Cannot write to config file!");
    }
}

const HOTKEY_CONFIG_KEY: &str = "hotkey";
const TYPING_METHOD_CONFIG_KEY: &str = "method";

pub struct ConfigManager {
    hotkey: Hotkey,
    typing_method: TypingMethod,
    config_store: ConfigStore,
}

impl ConfigManager {
    pub fn new() -> Self {
        let store = ConfigStore::new();
        let hotkey = Hotkey::from(&store.read(HOTKEY_CONFIG_KEY));
        let method = match store.read(TYPING_METHOD_CONFIG_KEY).to_lowercase().as_str() {
            "vni" => TypingMethod::VNI,
            _ => TypingMethod::Telex,
        };
        Self {
            hotkey,
            typing_method: method,
            config_store: store,
        }
    }

    pub fn get_hotkey(&self) -> &Hotkey {
        return &self.hotkey;
    }

    pub fn get_typing_method(&self) -> &TypingMethod {
        return &self.typing_method;
    }

    pub fn set_hotkey(&mut self, key_sequence: &str) {
        self.hotkey = Hotkey::from(key_sequence);
        self.config_store.write(HOTKEY_CONFIG_KEY, key_sequence);
    }

    pub fn set_typing_method(&mut self, method: TypingMethod) {
        self.typing_method = method;
        self.config_store
            .write(TYPING_METHOD_CONFIG_KEY, &method.to_string());
    }
}
