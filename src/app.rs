use crate::{
    catalog::{self, CATEGORIES, ENTRIES},
    config::Config,
    runner,
};
use rand::seq::SliceRandom;
use std::collections::HashSet;

pub struct App {
    pub presets: Vec<crate::presets::Preset>,
    pub preset_open: bool,
    pub preset_selected: usize,
    pub preset_name: Option<String>,
    pub config: Config,
    pub category: usize,
    pub selected: usize,
    pub query: String,
    pub searching: bool,
    pub settings: bool,
    pub setting: usize,
    pub installed: Vec<bool>,
    pub usable: Vec<bool>,
    pub failed_installs: HashSet<usize>,
    pub featured: usize,
    pub message: String,
    pub tick: usize,
}
impl App {
    pub fn new(config: Config) -> Self {
        let category = match config.startup {
            1 => 1,
            2 => 2,
            _ => 0,
        };
        let mut app = Self {
            presets: vec![],
            preset_open: false,
            preset_selected: 0,
            preset_name: None,
            config,
            category,
            selected: 0,
            query: String::new(),
            searching: false,
            settings: false,
            setting: 0,
            installed: vec![],
            usable: vec![],
            failed_installs: HashSet::new(),
            featured: 0,
            message: "Welcome to your terminal gallery.".into(),
            tick: 0,
        };
        app.refresh();
        let pool: Vec<_> = (0..ENTRIES.len()).filter(|i| app.usable[*i]).collect();
        app.featured = *pool.choose(&mut rand::thread_rng()).unwrap_or(&0);
        app
    }
    pub fn refresh(&mut self) {
        self.installed = ENTRIES
            .iter()
            .map(|e| catalog::which(e.executable).is_some())
            .collect();
        self.usable = ENTRIES.iter().map(catalog::usable).collect();
        self.failed_installs.clear();
    }
    pub fn visible(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        let mut entries: Vec<_> = ENTRIES
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let category = match self.category {
                    0 => true,
                    1 => self.config.favorites.contains(e.id),
                    2 => self.config.recent.iter().any(|s| s == e.id),
                    _ => e.category == CATEGORIES[self.category],
                };
                category
                    && format!(
                        "{} {} {} {} {}",
                        e.title, e.executable, e.description, e.package, e.category
                    )
                    .to_lowercase()
                    .contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        if self.category == 2 {
            entries.sort_by_key(|i| self.config.recent.iter().position(|s| s == ENTRIES[*i].id));
        }
        entries
    }
    pub fn current(&self) -> Option<usize> {
        self.visible().get(self.selected).copied()
    }
    pub fn move_selection(&mut self, delta: isize) {
        let len = self.visible().len();
        self.selected = if len == 0 {
            0
        } else {
            (self.selected as isize + delta).rem_euclid(len as isize) as usize
        };
    }
    pub fn favorite(&mut self) {
        if let Some(i) = self.current() {
            if !self.config.favorites.remove(ENTRIES[i].id) {
                self.config.favorites.insert(ENTRIES[i].id.into());
            }
            self.selected = self.selected.min(self.visible().len().saturating_sub(1));
            self.save();
        }
    }
    pub fn save(&mut self) {
        if let Err(e) = self.config.save() {
            self.message = format!("Could not save settings: {e}");
        }
    }
    pub fn pool(&self, mode: char) -> Vec<usize> {
        ENTRIES
            .iter()
            .enumerate()
            .filter(|(i, e)| {
                self.usable[*i]
                    && match mode {
                        'a' => matches!(e.category, "Animations" | "Space") && e.cycle,
                        'm' => e.matrix,
                        's' => e.cycle,
                        _ => true,
                    }
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn can_install(&self, i: usize) -> bool {
        !self.installed[i]
            && ENTRIES[i].install_hint.is_none()
            && !self.failed_installs.contains(&i)
            && runner::manager().is_some()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn space_search_and_playback_exclude_static_constellation_cards() {
        let mut app = App::new(Config::default());
        app.usable.fill(true);
        app.query = "space".into();
        let space: Vec<_> = app.visible().into_iter().map(|i| ENTRIES[i].id).collect();
        assert_eq!(space, ["astroterm", "globe", "starfetch", "terrascope"]);
        for mode in ['a', 's'] {
            let pool: Vec<_> = app.pool(mode).into_iter().map(|i| ENTRIES[i].id).collect();
            assert!(pool.contains(&"astroterm"));
            assert!(pool.contains(&"globe"));
            assert!(!pool.contains(&"starfetch"));
            assert!(!pool.contains(&"terrascope"));
        }
        let starfetch = ENTRIES.iter().position(|e| e.id == "starfetch").unwrap();
        app.installed[starfetch] = false;
        assert!(!app.can_install(starfetch));
    }
    #[test]
    fn filtering_history_and_failed_install_state() {
        let mut a = App::new(Config::default());
        a.query = "digital rain".into();
        assert_eq!(a.visible().len(), 2);
        a.query.clear();
        a.config.record("cowsay");
        a.config.record("figlet");
        a.category = 2;
        assert_eq!(ENTRIES[a.visible()[0]].id, "figlet");
        a.usable.fill(false);
        assert!(a.pool('s').is_empty());
        a.usable[1] = true;
        assert_eq!(a.pool('m'), vec![1]);
        a.failed_installs.insert(0);
        assert!(!a.can_install(0));
    }
}
