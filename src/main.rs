//! `art`: a Linux terminal-art gallery and launcher.
mod app;
mod catalog;
mod config;
mod presets;
mod runner;
mod ui;
use anyhow::{bail, Context, Result};
use app::App;
use catalog::ENTRIES;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self, IsTerminal},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
type Screen = Terminal<CrosstermBackend<io::Stdout>>;
struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}
fn restore() -> Result<()> {
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)?;
    Ok(())
}
fn enter(screen: &mut Screen) -> Result<()> {
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
    screen.clear()?;
    Ok(())
}
fn run_entry(
    screen: &mut Screen,
    app: &mut App,
    i: usize,
    install: bool,
    terminated: &AtomicBool,
) -> Result<()> {
    restore()?;
    let result = if install {
        runner::install(&ENTRIES[i], terminated)
    } else {
        runner::launch(&ENTRIES[i], terminated)
    };
    enter(screen)?;
    match result {
        Ok(()) => {
            app.message = format!(
                "{} {}",
                ENTRIES[i].title,
                if install { "installed." } else { "finished." }
            );
            if !install {
                app.config.record(ENTRIES[i].id);
                app.save();
            }
        }
        Err(e) => {
            app.message = e.to_string();
            if install {
                app.failed_installs.insert(i);
            }
        }
    }
    app.installed[i] = catalog::which(ENTRIES[i].executable).is_some();
    app.usable[i] = catalog::usable(&ENTRIES[i]);
    Ok(())
}
fn playback(screen: &mut Screen, app: &mut App, mode: char, terminated: &AtomicBool) -> Result<()> {
    let mut pool = app.pool(mode);
    if pool.is_empty() {
        app.message = "No compatible installed effects. Install one first.".into();
        return Ok(());
    }
    restore()?;
    // Raw input is reserved for the playback controller. Effects get no stdin.
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
    let result = (|| -> Result<()> {
        let mut index = 0;
        let mut previous = None;
        loop {
            if pool.is_empty() {
                break;
            }
            let i = if mode == 'a' {
                if index >= pool.len() {
                    break;
                }
                pool[index]
            } else {
                let choices: Vec<_> = pool
                    .iter()
                    .copied()
                    .filter(|i| Some(*i) != previous || pool.len() == 1)
                    .collect();
                *choices.choose(&mut rand::thread_rng()).unwrap()
            };
            terminal::enable_raw_mode()?;
            execute!(
                io::stdout(),
                terminal::Clear(terminal::ClearType::All),
                cursor::MoveTo(0, 0)
            )?;
            println!("ART · {} · Esc stops · Space / → skips\r", ENTRIES[i].title);
            app.config.record(ENTRIES[i].id);
            match runner::effect(&ENTRIES[i], app.config.seconds(), terminated) {
                Ok(false) => break,
                Ok(true) => {
                    index += 1;
                    previous = Some(i);
                }
                Err(e) => {
                    app.message = format!("Effect removed from playback: {e}");
                    pool.retain(|j| *j != i);
                }
            }
            // Bound fast-exiting programs so a missing audio backend cannot spin.
            std::thread::sleep(Duration::from_millis(150));
        }
        Ok(())
    })();
    restore()?;
    enter(screen)?;
    app.save();
    result
}
fn settings_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('c') => {
            app.settings = false;
            app.save();
        }
        KeyCode::Up => app.setting = (app.setting + 5) % 6,
        KeyCode::Down => app.setting = (app.setting + 1) % 6,
        KeyCode::Left | KeyCode::Right | KeyCode::Enter => {
            let delta = if key == KeyCode::Left { -1_isize } else { 1 };
            match app.setting {
                0 => app.config.theme = (app.config.theme as isize + delta).rem_euclid(3) as usize,
                1 => app.config.speed = (app.config.speed as isize + delta).rem_euclid(3) as usize,
                2 => app.config.splash = !app.config.splash,
                3 => {
                    app.config.startup =
                        (app.config.startup as isize + delta).rem_euclid(4) as usize
                }
                4 => {
                    app.category = 1;
                    app.query.clear();
                    app.selected = 0;
                    app.settings = false;
                }
                5 if key == KeyCode::Enter => {
                    app.config.favorites.clear();
                    app.message = "Favorites cleared.".into();
                    app.selected = 0;
                }
                _ => {}
            }
            app.save();
        }
        _ => {}
    }
}
fn preset_key(screen: &mut Screen, app: &mut App, key: KeyCode) -> Result<()> {
    if let Some(name) = &mut app.preset_name {
        match key {
            KeyCode::Esc => app.preset_name = None,
            KeyCode::Backspace => {
                name.pop();
            }
            KeyCode::Char(c) if name.len() < 80 => name.push(c),
            KeyCode::Enter => {
                let name = name.clone();
                let result = presets::capture(&name, None).and_then(|p| p.save(false));
                match result {
                    Ok(_) => {
                        app.message = format!("Saved preset {name}.");
                        app.presets = presets::list()?;
                        app.preset_name = None;
                    }
                    Err(e) => app.message = format!("Could not save preset: {e:#}"),
                }
            }
            _ => {}
        }
        return Ok(());
    }
    let len = app.presets.len();
    match key {
        KeyCode::Esc | KeyCode::Char('p') => app.preset_open = false,
        KeyCode::Char('n') | KeyCode::Char('N') => app.preset_name = Some(String::new()),
        KeyCode::Up if len > 0 => app.preset_selected = (app.preset_selected + len - 1) % len,
        KeyCode::Down if len > 0 => app.preset_selected = (app.preset_selected + 1) % len,
        KeyCode::Enter => {
            if let Some(p) = app.presets.get(app.preset_selected).cloned() {
                app.message = format!("Restoring {}…", p.name);
                screen.draw(|f| ui::draw(f, app))?;
                app.message = match presets::launch(&p, None) {
                    Ok(message) => message,
                    Err(e) => format!("Preset restore failed: {e:#}"),
                };
            }
        }
        _ => {}
    }
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "install") {
        return runner::install_selection(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "__preset-exec") {
        return presets::preset_exec(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "preset") {
        return presets::cli(&args[1..]);
    }
    if args.first().is_some_and(|s| s == "--preset") {
        let name = args.get(1).context("Usage: art --preset NAME")?;
        println!("{}", presets::launch(&presets::load(name)?, None)?);
        return Ok(());
    }
    if !args.is_empty() {
        match args[0].as_str() {
            "--help"|"-h"=>println!("art — terminal art launcher\n\nUsage: art [--list | --no-splash | --version | --help]\n\n↑↓ navigate · ←→ categories · Enter launch · / search · Esc quit\nI install · F favorite · R random · A animations · M matrix\nS screensaver · C configuration · P presets · U refresh\n\nPresets: art preset list | save NAME --workspace N | launch NAME\nShortcut: art --preset NAME\nInstall: art install [--all | CATEGORY | PROGRAM] [--dry-run]\n\nPlayback: Esc / Q / Ctrl-C stops; Space / → skips."),
            "--version"|"-V"=>println!("art {}",env!("CARGO_PKG_VERSION")),
            "--list"=>for e in ENTRIES {println!("{:<14} {:<12} {:<15} {:<15} {}",e.title,e.category,if catalog::which(e.executable).is_some(){"Installed"}else{"Not Installed"},e.package,e.description);},
            "--no-splash"=>{},
            other=>bail!("Unknown option {other}; use --help"),
        }
        if args[0] != "--no-splash" {
            return Ok(());
        }
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("art needs an interactive terminal. Use --list for plain output.");
    }
    let mut app = App::new(config::Config::load()?);
    let interrupted = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&interrupted))?;
    let terminated = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&terminated))?;
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        old_hook(info);
    }));
    let _guard = TerminalGuard;
    let mut screen = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    enter(&mut screen)?;
    if app.config.splash && args.is_empty() {
        for phase in 0..12 {
            screen.draw(|f| ui::splash(f, phase, ui::accent(&app)))?;
            if event::poll(Duration::from_millis([100, 65, 35][app.config.speed]))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Esc {
                        return Ok(());
                    }
                    break;
                }
            }
        }
    }
    if app.config.startup == 3 {
        if let Some(&i) = app.pool('r').choose(&mut rand::thread_rng()) {
            run_entry(&mut screen, &mut app, i, false, &terminated)?;
        }
    }
    loop {
        if terminated.load(Ordering::Relaxed) {
            break;
        }
        interrupted.store(false, Ordering::Relaxed);
        app.selected = app.selected.min(app.visible().len().saturating_sub(1));
        screen.draw(|f| ui::draw(f, &app))?;
        if !event::poll(Duration::from_millis(100))? {
            app.tick = app.tick.wrapping_add(1);
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            break;
        }
        if app.preset_open {
            preset_key(&mut screen, &mut app, key.code)?;
            continue;
        }
        if app.settings {
            settings_key(&mut app, key.code);
            continue;
        }
        if app.searching {
            match key.code {
                KeyCode::Esc => {
                    app.searching = false;
                    app.query.clear();
                }
                KeyCode::Enter => app.searching = false,
                KeyCode::Backspace => {
                    app.query.pop();
                    app.selected = 0;
                }
                KeyCode::Char(c) => {
                    app.query.push(c);
                    app.selected = 0;
                }
                _ => {}
            }
            continue;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => break,
            KeyCode::Up => app.move_selection(-1),
            KeyCode::Down => app.move_selection(1),
            KeyCode::Left => {
                app.category =
                    (app.category + catalog::CATEGORIES.len() - 1) % catalog::CATEGORIES.len();
                app.selected = 0;
            }
            KeyCode::Right | KeyCode::Tab => {
                app.category = (app.category + 1) % catalog::CATEGORIES.len();
                app.selected = 0;
            }
            KeyCode::Char('/') => app.searching = true,
            KeyCode::Enter => {
                if let Some(i) = app.current() {
                    if app.usable[i] {
                        run_entry(&mut screen, &mut app, i, false, &terminated)?;
                    } else {
                        app.message="Program unavailable. See the preview for installation or requirements.".into();
                    }
                }
            }
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                'f' => app.favorite(),
                'c' => app.settings = true,
                'p' => match presets::list() {
                    Ok(items) => {
                        app.presets = items;
                        app.preset_selected = 0;
                        app.preset_open = true;
                    }
                    Err(e) => app.message = format!("Presets: {e:#}"),
                },
                'u' => {
                    app.refresh();
                    app.message =
                        "Discovery refreshed. Failed installations can be retried.".into();
                }
                'i' => {
                    if let Some(i) = app.current() {
                        if app.can_install(i) {
                            run_entry(&mut screen, &mut app, i, true, &terminated)?;
                        }
                    }
                }
                'r' => {
                    if let Some(&i) = app.pool('r').choose(&mut rand::thread_rng()) {
                        run_entry(&mut screen, &mut app, i, false, &terminated)?;
                    } else {
                        app.message = "Install a program to use random mode.".into();
                    }
                }
                mode @ ('a' | 'm' | 's') => playback(&mut screen, &mut app, mode, &terminated)?,
                _ => {}
            },
            _ => {}
        }
    }
    app.config.save()?;
    Ok(())
}
