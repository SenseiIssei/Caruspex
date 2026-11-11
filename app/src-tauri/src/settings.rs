// Settings that are stored permanently (Settings.h/.cpp).
//
// The Qt version kept them in QSettings: on Windows the registry under
// HKCU\Software\Caruspex\Caruspex, on Linux ~/.config/Caruspex/Caruspex.conf.
// Here they live in settings.json in the app's config folder; on the first
// start the old values are read once and carried over.
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    // --- Connection ----------------------------------------------------------
    /// 0 means: try the usual baud rates.
    pub baud_override: i32,
    /// Order in which the protocols are tried. "kline" suits the A2, "can" the A4 B7.
    pub protocol_order: String,
    /// All timeouts are multiplied by this factor. Sluggish clones need more
    /// time, a good cable gets by with less.
    pub timeout_percent: i32,
    /// 0 standard (ATAT1), 1 faster on STN adapters only, 2 always ATAT2.
    pub adaptive_timing: i32,
    /// Last used port, so the selection doesn't start from scratch every time.
    pub last_port: String,
    pub auto_connect: bool,

    // --- Live values ---------------------------------------------------------
    /// Interval between two requests. Zero means: as fast as possible.
    pub poll_interval_ms: i32,
    /// How far back the charts reach.
    pub chart_window_seconds: i32,

    // --- Clearing ------------------------------------------------------------
    pub clear_attempts: i32,
    pub confirm_clear: bool,

    // --- K-Line --------------------------------------------------------------
    /// Session type for basic settings on KWP2000. Not publicly documented,
    /// hence selectable.
    pub basic_setting_session: i32,
    pub basic_setting_interval_ms: i32,
    pub kkl_ecu_address: i32,

    // --- Log -----------------------------------------------------------------
    pub log_raw: bool,
    pub log_to_file: bool,
    pub log_path: String,

    // --- Window --------------------------------------------------------------
    pub remember_geometry: bool,
    /// Ask GitHub for a newer version at start.
    pub check_updates: bool,

    // --- Language ------------------------------------------------------------
    /// "auto" follows the system language, otherwise a language code.
    pub language: String,
    /// "auto" follows the system's light or dark mode, otherwise "dark" or "light".
    pub appearance: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            baud_override: 0,
            protocol_order: "kline".into(),
            timeout_percent: 100,
            adaptive_timing: 0,
            last_port: String::new(),
            auto_connect: false,
            poll_interval_ms: 20,
            chart_window_seconds: 60,
            clear_attempts: 5,
            confirm_clear: true,
            basic_setting_session: 0x85,
            basic_setting_interval_ms: 180,
            kkl_ecu_address: 0x01,
            log_raw: true,
            log_to_file: false,
            log_path: String::new(),
            remember_geometry: true,
            check_updates: true,
            language: "auto".into(),
            appearance: "auto".into(),
        }
    }
}

impl AppSettings {
    /// Values from an older or hand-edited file could be nonsense. Better to
    /// clamp them than to divide by zero later.
    pub fn sanitized(mut self) -> Self {
        self.adaptive_timing = self.adaptive_timing.clamp(0, 2);
        if self.appearance != "dark" && self.appearance != "light" {
            self.appearance = "auto".into();
        }
        if !caruspex_i18n::languages()
            .iter()
            .any(|i| i.code == self.language)
        {
            self.language = "auto".into();
        }
        self.timeout_percent = self.timeout_percent.clamp(25, 500);
        self.poll_interval_ms = self.poll_interval_ms.clamp(0, 5000);
        self.chart_window_seconds = self.chart_window_seconds.clamp(10, 3600);
        self.clear_attempts = self.clear_attempts.clamp(1, 20);
        self.basic_setting_interval_ms = self.basic_setting_interval_ms.clamp(50, 2000);
        self
    }
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

/// Reads settings.json; on the very first start the values of the Qt version.
pub fn load_settings(config_dir: &Path) -> AppSettings {
    let path = settings_path(config_dir);
    if let Ok(text) = std::fs::read_to_string(&path) {
        return serde_json::from_str::<AppSettings>(&text)
            .unwrap_or_default()
            .sanitized();
    }
    let values = legacy::read().unwrap_or_default().sanitized();
    // Written right away, so the migration runs once.
    let _ = save_settings(config_dir, &values);
    values
}

pub fn save_settings(config_dir: &Path, values: &AppSettings) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(values).map_err(|e| e.to_string())?;
    write_atomically(&settings_path(config_dir), text.as_bytes())
}

/// Writes to a temporary file first, so a crash mid-write never leaves a
/// broken file behind (QSaveFile).
pub fn write_atomically(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// The QSettings keys of the Qt version: (group, key).
pub mod legacy {
    use super::AppSettings;

    /// A value as QSettings stored it: number, flag or text.
    #[derive(Clone, Debug, PartialEq)]
    pub enum Value {
        /// Only the Windows registry stores numbers as numbers; the Linux and
        /// macOS files of the Qt version hold everything as text.
        #[cfg_attr(not(windows), allow(dead_code))]
        Int(i64),
        Text(String),
    }

    impl Value {
        fn int(&self) -> Option<i32> {
            match self {
                Value::Int(v) => i32::try_from(*v).ok(),
                Value::Text(t) => t.trim().parse().ok(),
            }
        }
        fn flag(&self) -> Option<bool> {
            match self {
                Value::Int(v) => Some(*v != 0),
                Value::Text(t) => match t.trim() {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                },
            }
        }
        fn text(&self) -> String {
            match self {
                Value::Int(v) => v.to_string(),
                Value::Text(t) => t.clone(),
            }
        }
    }

    /// Builds the settings from a lookup (group "" is the top level).
    pub fn from_lookup(get: impl Fn(&str, &str) -> Option<Value>) -> AppSettings {
        let mut v = AppSettings::default();
        let int = |g: &str, k: &str, d: i32| get(g, k).and_then(|x| x.int()).unwrap_or(d);
        let flag = |g: &str, k: &str, d: bool| get(g, k).and_then(|x| x.flag()).unwrap_or(d);
        let text = |g: &str, k: &str, d: &str| {
            get(g, k).map(|x| x.text()).unwrap_or_else(|| d.to_string())
        };

        v.baud_override = int("verbindung", "baud", v.baud_override);
        v.protocol_order = text("verbindung", "protokoll", &v.protocol_order);
        v.timeout_percent = int("verbindung", "zeitlimit", v.timeout_percent);
        v.adaptive_timing = int("verbindung", "taktung", v.adaptive_timing);
        v.last_port = text("verbindung", "anschluss", &v.last_port);
        v.auto_connect = flag("verbindung", "autoverbinden", v.auto_connect);

        v.poll_interval_ms = int("messwerte", "takt", v.poll_interval_ms);
        v.chart_window_seconds = int("messwerte", "zeitfenster", v.chart_window_seconds);

        v.clear_attempts = int("loeschen", "versuche", v.clear_attempts);
        v.confirm_clear = flag("loeschen", "nachfragen", v.confirm_clear);

        v.basic_setting_session = int("kline", "sitzungsart", v.basic_setting_session);
        v.basic_setting_interval_ms = int("kline", "takt", v.basic_setting_interval_ms);
        v.kkl_ecu_address = int("kline", "steuergeraet", v.kkl_ecu_address);

        v.log_raw = flag("protokoll", "roh", v.log_raw);
        v.log_to_file = flag("protokoll", "datei", v.log_to_file);
        v.log_path = text("protokoll", "pfad", &v.log_path);

        v.remember_geometry = flag("fenster", "merken", v.remember_geometry);
        v.check_updates = flag("fenster", "updates", v.check_updates);

        v.language = text("", "sprache", &v.language);
        v.appearance = text("", "erscheinungsbild", &v.appearance);
        v
    }

    /// The old settings, if the Qt version ever stored any on this computer.
    pub fn read() -> Option<AppSettings> {
        platform::read()
    }

    /// Parses the INI file QSettings writes on Linux. Top level keys sit
    /// under [General].
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn parse_ini(text: &str) -> std::collections::HashMap<(String, String), String> {
        let mut out = std::collections::HashMap::new();
        let mut group = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                group = line[1..line.len() - 1].to_string();
                if group == "General" {
                    group.clear();
                }
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let mut value = v.trim().to_string();
                if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                    value = value[1..value.len() - 1]
                        .replace("\\\"", "\"")
                        .replace("\\\\", "\\");
                }
                out.insert((group.clone(), k.trim().to_string()), value);
            }
        }
        out
    }

    #[cfg(windows)]
    mod platform {
        use super::{AppSettings, Value, from_lookup};
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, RegType};

        pub fn read() -> Option<AppSettings> {
            let root = RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey("Software\\Caruspex\\Caruspex")
                .ok()?;
            Some(from_lookup(|group, key| {
                let node = if group.is_empty() {
                    root.open_subkey("").ok()?
                } else {
                    root.open_subkey(group).ok()?
                };
                let raw = node.get_raw_value(key).ok()?;
                match raw.vtype {
                    RegType::REG_DWORD => {
                        let b: [u8; 4] = raw.bytes.get(..4)?.try_into().ok()?;
                        Some(Value::Int(i64::from(u32::from_le_bytes(b))))
                    }
                    RegType::REG_QWORD => {
                        let b: [u8; 8] = raw.bytes.get(..8)?.try_into().ok()?;
                        Some(Value::Int(i64::from_le_bytes(b)))
                    }
                    RegType::REG_SZ | RegType::REG_EXPAND_SZ => {
                        node.get_value::<String, _>(key).ok().map(Value::Text)
                    }
                    _ => None,
                }
            }))
        }
    }

    #[cfg(target_os = "linux")]
    mod platform {
        use super::{AppSettings, Value, from_lookup, parse_ini};

        pub fn read() -> Option<AppSettings> {
            let base = std::env::var_os("XDG_CONFIG_HOME")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"))
                })?;
            let text = std::fs::read_to_string(base.join("Caruspex").join("Caruspex.conf")).ok()?;
            let map = parse_ini(&text);
            Some(from_lookup(|group, key| {
                map.get(&(group.to_string(), key.to_string()))
                    .cloned()
                    .map(Value::Text)
            }))
        }
    }

    // macOS kept them in a binary property list
    // (~/Library/Preferences/com.caruspex.Caruspex.plist), which is not read
    // here; the settings start from their defaults there.
    #[cfg(not(any(windows, target_os = "linux")))]
    mod platform {
        use super::AppSettings;

        pub fn read() -> Option<AppSettings> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::legacy::{Value, from_lookup, parse_ini};
    use super::*;

    #[test]
    fn defaults_match_the_interface() {
        let v = serde_json::to_value(AppSettings::default()).unwrap();
        assert_eq!(v["protocolOrder"], "kline");
        assert_eq!(v["basicSettingSession"], 0x85);
        assert_eq!(v["pollIntervalMs"], 20);
        // Missing fields take their default.
        let partial: AppSettings = serde_json::from_str(r#"{"pollIntervalMs": 50}"#).unwrap();
        assert_eq!(partial.poll_interval_ms, 50);
        assert_eq!(partial.clear_attempts, 5);
    }

    #[test]
    fn clamps_nonsense() {
        let v = AppSettings {
            timeout_percent: 5,
            poll_interval_ms: -3,
            language: "xx".into(),
            appearance: "pink".into(),
            ..AppSettings::default()
        }
        .sanitized();
        assert_eq!(v.timeout_percent, 25);
        assert_eq!(v.poll_interval_ms, 0);
        assert_eq!(v.language, "auto");
        assert_eq!(v.appearance, "auto");
    }

    #[test]
    fn reads_the_qt_ini_file() {
        let map = parse_ini(
            "[General]\nsprache=fr\nerscheinungsbild=light\n\n[verbindung]\nbaud=38400\nautoverbinden=true\nanschluss=/dev/ttyUSB0\n",
        );
        let v = from_lookup(|g, k| {
            map.get(&(g.to_string(), k.to_string()))
                .cloned()
                .map(Value::Text)
        });
        assert_eq!(v.language, "fr");
        assert_eq!(v.appearance, "light");
        assert_eq!(v.baud_override, 38400);
        assert!(v.auto_connect);
        assert_eq!(v.last_port, "/dev/ttyUSB0");
        assert_eq!(v.clear_attempts, 5);
    }

    #[test]
    fn json_round_trip() {
        let dir = std::env::temp_dir().join(format!("caruspex-settings-{}", std::process::id()));
        let values = AppSettings {
            last_port: "COM7".into(),
            ..AppSettings::default()
        };
        save_settings(&dir, &values).unwrap();
        assert_eq!(load_settings(&dir), values);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
