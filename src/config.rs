//! Small, version-tolerant XDG configuration, saved with an atomic rename.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: usize,
    /// Controls the launcher splash and playback interval, not third-party settings.
    pub speed: usize,
    pub splash: bool,
    pub startup: usize,
    pub favorites: BTreeSet<String>,
    pub recent: Vec<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            theme: 0,
            speed: 1,
            splash: true,
            startup: 0,
            favorites: BTreeSet::new(),
            recent: Vec::new(),
        }
    }
}
impl Config {
    pub fn path() -> Result<PathBuf> {
        Ok(dirs::config_dir()
            .context("Cannot locate configuration directory")?
            .join("terminal-art-launcher/config.toml"))
    }
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        let mut c: Self = match fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s)
                .with_context(|| format!("Invalid configuration: {}", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => return Err(e.into()),
        };
        c.theme %= 3;
        c.speed %= 3;
        c.startup %= 4;
        c.recent.truncate(20);
        Ok(c)
    }
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        fs::create_dir_all(path.parent().context("Invalid configuration path")?)?;
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temp, toml::to_string_pretty(self)?)?;
        fs::rename(temp, path)?;
        Ok(())
    }
    pub fn record(&mut self, id: &str) {
        self.recent.retain(|s| s != id);
        self.recent.insert(0, id.to_owned());
        self.recent.truncate(20);
    }
    pub fn seconds(&self) -> u64 {
        [30, 15, 7][self.speed]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_is_unique_and_bounded() {
        let mut c = Config::default();
        for i in 0..25 {
            c.record(&i.to_string());
        }
        c.record("12");
        assert_eq!(c.recent.len(), 20);
        assert_eq!(c.recent[0], "12");
        assert_eq!(c.recent.iter().filter(|s| *s == "12").count(), 1);
    }
    #[test]
    fn old_configuration_gets_defaults() {
        let c: Config = toml::from_str("splash = false").unwrap();
        assert!(!c.splash);
        assert_eq!(c.seconds(), 15);
    }
}
