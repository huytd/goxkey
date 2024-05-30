use std::collections::BTreeMap;
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
    is_macro_enabled: bool,
    macro_table: BTreeMap<String, String>,
    is_auto_toggle_enabled: bool,
    is_gox_mode_enabled: bool,
    allowed_words: Vec<String>,
}

fn parse_vec_string(line: String) -> Vec<String> {
    line.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_kv_string(line: &str) -> Option<(String, String)> {
    if let Some((left, right)) = line.split_once("\"=\"") {
        let left = left.strip_prefix("\"").map(|s| s.replace("\\\"", "\""));
        let right = right.strip_suffix("\"").map(|s| s.replace("\\\"", "\""));
        return left.zip(right);
    }
    return None;
}

fn build_kv_string(k: &str, v: &str) -> String {
    format!(
        "\"{}\"=\"{}\"",
        k.replace("\"", "\\\""),
        v.replace("\"", "\\\"")
    )
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
        writeln!(
            file,
            "{} = {}",
            ALLOWED_WORDS_CONFIG_KEY,
            self.allowed_words.join(",")
        )?;
        writeln!(
            file,
            "{} = {}",
            AUTOS_TOGGLE_ENABLED_CONFIG_KEY, self.is_auto_toggle_enabled
        )?;
        writeln!(
            file,
            "{} = {}",
            MACRO_ENABLED_CONFIG_KEY, self.is_macro_enabled
        )?;
        for (k, v) in self.macro_table.iter() {
            writeln!(file, "{} = {}", MACROS_CONFIG_KEY, build_kv_string(k, &v))?;
        }
        writeln!(
            file,
            "{} = {}",
            GOX_MODE_CONFIG_KEY, self.is_gox_mode_enabled
        )?;
        Ok(())
    }

    pub fn new() -> Self {
        let mut config = Self {
            hotkey: "ctrl+space".to_string(),
            method: "telex".to_string(),
            vn_apps: Vec::new(),
            en_apps: Vec::new(),
            is_macro_enabled: false,
            macro_table: BTreeMap::new(),
            is_auto_toggle_enabled: false,
            is_gox_mode_enabled: false,
            allowed_words: vec!["đc".to_string()],
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
                        ALLOWED_WORDS_CONFIG_KEY => {
                            config.allowed_words = parse_vec_string(right.to_string())
                        }
                        AUTOS_TOGGLE_ENABLED_CONFIG_KEY => {
                            config.is_auto_toggle_enabled = matches!(right.trim(), "true")
                        }
                        MACRO_ENABLED_CONFIG_KEY => {
                            config.is_macro_enabled = matches!(right.trim(), "true")
                        }
                        MACROS_CONFIG_KEY => {
                            if let Some((k, v)) = parse_kv_string(right) {
                                config.macro_table.insert(k, v);
                            }
                        }
                        GOX_MODE_CONFIG_KEY => {
                            config.is_gox_mode_enabled = matches!(right.trim(), "true")
                        }
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

    pub fn is_allowed_word(&self, word: &str) -> bool {
        self.allowed_words.contains(&word.to_string())
    }

    pub fn is_auto_toggle_enabled(&self) -> bool {
        self.is_auto_toggle_enabled
    }

    pub fn set_auto_toggle_enabled(&mut self, flag: bool) {
        self.is_auto_toggle_enabled = flag;
        self.save();
    }

    pub fn is_gox_mode_enabled(&self) -> bool {
        self.is_gox_mode_enabled
    }

    pub fn set_gox_mode_enabled(&mut self, flag: bool) {
        self.is_gox_mode_enabled = flag;
        self.save();
    }

    pub fn is_macro_enabled(&self) -> bool {
        self.is_macro_enabled
    }

    pub fn set_macro_enabled(&mut self, flag: bool) {
        self.is_macro_enabled = flag;
        self.save();
    }

    pub fn get_macro_table(&self) -> &BTreeMap<String, String> {
        &self.macro_table
    }

    pub fn add_macro(&mut self, from: String, to: String) {
        self.macro_table.insert(from, to);
        self.save();
    }

    pub fn delete_macro(&mut self, from: &String) {
        self.macro_table.remove(from);
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
const MACRO_ENABLED_CONFIG_KEY: &str = "is_macro_enabled";
const AUTOS_TOGGLE_ENABLED_CONFIG_KEY: &str = "is_auto_toggle_enabled";
const MACROS_CONFIG_KEY: &str = "macros";
const GOX_MODE_CONFIG_KEY: &str = "is_gox_mode_enabled";
const ALLOWED_WORDS_CONFIG_KEY: &str = "allowed_words";
