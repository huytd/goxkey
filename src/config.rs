use std::io::BufRead;
use std::{
    fs::File,
    io,
    io::{Result, Write},
    path::PathBuf,
    sync::Mutex,
};

use once_cell::sync::Lazy;

use crate::platform::get_home_dir;

pub static CONFIG_MANAGER: Lazy<Mutex<ConfigStore>> = Lazy::new(|| Mutex::new(ConfigStore::new()));

pub struct ConfigStore {
    hotkey: String,
    method: String,
    vn_apps: Vec<String>,
    en_apps: Vec<String>,
}

fn parse_vec_string(line: String) -> Vec<String> {
    line.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

impl ConfigStore {
    fn get_config_path() -> PathBuf {
        get_home_dir()
            .expect("Cannot read home directory!")
            .join(".goxkey")
    }

    fn write_config_data(&mut self) -> Result<()> {
        let mut file = File::create(ConfigStore::get_config_path())?;

        writeln!(file, "{} = {}", HOTKEY_CONFIG_KEY, self.hotkey)?;
        writeln!(file, "{} = {}", TYPING_METHOD_CONFIG_KEY, self.method)?;
        writeln!(file, "{} = {}", VN_APPS_CONFIG_KEY, self.vn_apps.join(","))?;
        writeln!(file, "{} = {}", EN_APPS_CONFIG_KEY, self.en_apps.join(","))?;

        Ok(())
    }

    pub fn new() -> Self {
        let mut config = Self {
            hotkey: "ctrl+space".to_string(),
            method: "telex".to_string(),
            vn_apps: Vec::new(),
            en_apps: Vec::new(),
        };

        let config_path = ConfigStore::get_config_path();

        if let Ok(file) = File::open(config_path) {
            let reader = io::BufReader::new(file);
            for line in reader.lines() {
                if let Some((left, right)) = line.unwrap_or_default().split_once(" = ") {
                    match left {
                        HOTKEY_CONFIG_KEY => config.hotkey = right.to_string(),
                        TYPING_METHOD_CONFIG_KEY => config.method = right.to_string(),
                        VN_APPS_CONFIG_KEY => config.vn_apps = parse_vec_string(right.to_string()),
                        EN_APPS_CONFIG_KEY => config.en_apps = parse_vec_string(right.to_string()),
                        _ => {}
                    }
                }
            }
        }

        config
    }

    // Hotkey
    pub fn get_hotkey(&self) -> &str {
        &self.hotkey
    }

    pub fn set_hotkey(&mut self, hotkey: &str) {
        self.hotkey = hotkey.to_string();
        self.save();
    }

    // Method
    pub fn get_method(&self) -> &str {
        &self.method
    }

    pub fn set_method(&mut self, method: &str) {
        self.method = method.to_string();
        self.save();
    }

    pub fn is_vietnamese_app(&self, app_name: &str) -> bool {
        self.vn_apps.contains(&app_name.to_string())
    }

    pub fn is_english_app(&self, app_name: &str) -> bool {
        self.en_apps.contains(&app_name.to_string())
    }

    pub fn add_vietnamese_app(&mut self, app_name: &str) {
        if self.is_english_app(app_name) {
            // Remove from english apps
            self.en_apps.retain(|x| x != app_name);
        }
        self.vn_apps.push(app_name.to_string());
        self.save();
    }

    pub fn add_english_app(&mut self, app_name: &str) {
        if self.is_vietnamese_app(app_name) {
            // Remove from vietnamese apps
            self.vn_apps.retain(|x| x != app_name);
        }
        self.en_apps.push(app_name.to_string());
        self.save();
    }

    // Save config to file
    fn save(&mut self) {
        self.write_config_data().expect("Failed to write config");
    }
}

const HOTKEY_CONFIG_KEY: &str = "hotkey";
const TYPING_METHOD_CONFIG_KEY: &str = "method";
const VN_APPS_CONFIG_KEY: &str = "vn-apps";
const EN_APPS_CONFIG_KEY: &str = "en-apps";
