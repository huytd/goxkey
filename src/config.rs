use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Result, Write},
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

use crate::platform::get_home_dir;
use druid::ExtEventSink;
use notify::{Event, EventKind, FsEventWatcher, Watcher};
use once_cell::sync::Lazy;

pub static CONFIG_MANAGER: Lazy<Mutex<ConfigStore>> = Lazy::new(|| Mutex::new(ConfigStore::new()));

pub struct ConfigStore {
    data: HashMap<String, String>,
}

impl ConfigStore {
    pub fn get_config_path() -> PathBuf {
        get_home_dir()
            .expect("Cannot read home directory!")
            .join(".goxkey")
    }

    fn load_config_data() -> Result<HashMap<String, String>> {
        let mut data = HashMap::new();
        let config_path = ConfigStore::get_config_path();
        let file = File::open(config_path.as_path());
        let mut buf = String::new();
        if let Ok(mut file) = file {
            file.read_to_string(&mut buf)?;
        } else {
            buf = format!(
                "{} = {}\n{} = {}",
                HOTKEY_CONFIG_KEY, "super+ctrl+space", TYPING_METHOD_CONFIG_KEY, "telex"
            );
        }
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

pub const HOTKEY_CONFIG_KEY: &str = "hotkey";
pub const TYPING_METHOD_CONFIG_KEY: &str = "method";

pub struct ConfigWatcher {
    watcher: FsEventWatcher,
}

impl ConfigWatcher {
    pub fn new(event_sink: ExtEventSink) -> Self {
        let watcher = notify::recommended_watcher(|res| match res {
            Ok(event) => {
                let event: Event = event;
                if let notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) = event.kind {
                    println!("Config file modified!");
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
            Err(_) => {}
        })
        .unwrap();
        Self { watcher }
    }

    pub fn start(&mut self) {
        let file_path = ConfigStore::get_config_path();
        _ = self
            .watcher
            .watch(&file_path.as_path(), notify::RecursiveMode::Recursive);
    }
}
