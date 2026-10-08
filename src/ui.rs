//! Responsive gallery layout with themed accents and a focused detail pane.
use crate::{
    app::App,
    catalog::{CATEGORIES, ENTRIES},
};
use ratatui::{prelude::*, widgets::*};
pub const THEMES: [&str; 3] = ["Aurora", "Ember", "Ocean"];
pub fn accent(app: &App) -> Color {
    [
        Color::Rgb(97, 239, 195),
        Color::Rgb(255, 180, 105),
        Color::Rgb(105, 185, 255),
    ][app.config.theme]
}
fn block(title: &str, color: Color) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(color))
}
pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let color = accent(app);
    let bg = Color::Rgb(13, 17, 27);
    f.render_widget(
        Block::default().style(Style::default().bg(bg).fg(Color::Rgb(220, 228, 242))),
        area,
    );
    if area.width < 64 || area.height < 18 {
        f.render_widget(
            Paragraph::new("ART • terminal gallery\n\nResize to at least 64 × 18.\nEsc quits.")
                .alignment(Alignment::Center),
            area,
        );
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(area);
    let count = app.installed.iter().filter(|s| **s).count();
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ✦ A R T  ", Style::default().fg(color).bold()),
            Span::raw(" / terminal gallery"),
            Span::styled(
                format!("    {count}/{} installed", ENTRIES.len()),
                Style::default().fg(Color::DarkGray),
            ),
        ]))
        .block(block("", color)),
        rows[0],
    );
    let feature = &ENTRIES[app.featured];
    let banner = if app.searching || !app.query.is_empty() {
        format!("  / {}{}", app.query, if app.searching { "▏" } else { "" })
    } else {
        format!(
            "  {} Featured · {}   —   {}",
            ["◇", "◈", "◆", "◈"][app.tick / 5 % 4],
            feature.title,
            feature.category
        )
    };
    f.render_widget(
        Paragraph::new(banner)
            .style(Style::default().fg(color))
            .block(block(
                if app.searching {
                    " Search · Enter accepts · Esc clears "
                } else {
                    " Discover "
                },
                Color::DarkGray,
            )),
        rows[1],
    );
    let columns = Layout::horizontal([
        Constraint::Length(17),
        Constraint::Percentage(39),
        Constraint::Min(20),
    ])
    .split(rows[2]);
    let icons = ["✦", "♥", "◷", "✺", "▣", "≡", "♫", "⚙", "☄"];
    let categories: Vec<_> = CATEGORIES
        .iter()
        .enumerate()
        .map(|(i, c)| ListItem::new(format!("{} {}", icons[i], c)))
        .collect();
    let mut state = ListState::default().with_selected(Some(app.category));
    f.render_stateful_widget(
        List::new(categories)
            .block(block(" Collections ", Color::DarkGray))
            .highlight_style(Style::default().fg(color).bg(Color::Rgb(29, 42, 54)).bold())
            .highlight_symbol("▸"),
        columns[0],
        &mut state,
    );
    let visible = app.visible();
    let entries: Vec<_> = visible
        .iter()
        .map(|i| {
            let e = &ENTRIES[*i];
            let star = if app.config.favorites.contains(e.id) {
                "♥"
            } else {
                " "
            };
            let status = if app.installed[*i] {
                "● Installed"
            } else {
                "○ Not Installed"
            };
            ListItem::new(vec![
                Line::from(vec![Span::styled(
                    format!("{star} {}", e.title),
                    Style::default().bold(),
                )]),
                Line::from(Span::styled(
                    format!("  {status}"),
                    Style::default().fg(if app.installed[*i] {
                        color
                    } else {
                        Color::DarkGray
                    }),
                )),
            ])
        })
        .collect();
    let mut state = ListState::default().with_selected(if visible.is_empty() {
        None
    } else {
        Some(app.selected)
    });
    f.render_stateful_widget(
        List::new(entries)
            .block(block(" Programs ", color))
            .highlight_style(Style::default().bg(Color::Rgb(34, 45, 66)))
            .highlight_symbol("▸ "),
        columns[1],
        &mut state,
    );
    if visible.is_empty() {
        f.render_widget(
            Paragraph::new("No matches.\nTry another collection\nor clear your search.")
                .wrap(Wrap { trim: true })
                .block(block(" Programs ", color)),
            columns[1],
        );
    }
    if let Some(i) = app.current() {
        let e = &ENTRIES[i];
        let status = if !app.installed[i] {
            "Not Installed"
        } else if !app.usable[i] {
            "Installed · needs DISPLAY"
        } else {
            "Installed · ready"
        };
        let mut text = vec![
            Line::from(Span::styled(
                format!("✦ {}", e.title),
                Style::default().fg(color).bold(),
            )),
            Line::from(e.category),
            Line::from(""),
            Line::from(e.description),
            Line::from(""),
            Line::from(format!("Executable: {}", e.executable)),
            Line::from(format!("Package: {}", e.package)),
            Line::from(""),
            Line::from(Span::styled(status, Style::default().fg(color))),
        ];
        if app.can_install(i) {
            text.push(Line::from("Press I to install."));
        } else if app.failed_installs.contains(&i) {
            text.push(Line::from("Install unavailable. U refreshes / retries."));
        } else if !app.installed[i] && e.install_hint.is_some() {
            text.push(Line::from(e.install_hint.unwrap_or_default()));
        } else if !app.installed[i] {
            text.push(Line::from("Install manually; no supported manager found."));
        }
        if app.usable[i] {
            text.push(Line::from("Enter opens · F favorites"));
        }
        f.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: true })
                .block(block(" Preview / about ", Color::DarkGray)),
            columns[2],
        );
    }
    f.render_widget(
        Paragraph::new(format!("  {}", app.message))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(color)),
        rows[3],
    );
    let mut shortcuts = " ↑↓ select  ←→ category  Enter run  / search  F ♥  R random  I install  U refresh\n A sequence  M matrix  S screensaver  C config  P presets  Esc quit".to_owned();
    if !app.current().is_some_and(|i| app.can_install(i)) {
        shortcuts = shortcuts.replace("I install  ", "");
    }
    if app.pool('r').is_empty() {
        shortcuts = shortcuts.replace("R random  ", "");
    }
    if app.pool('a').is_empty() {
        shortcuts = shortcuts.replace("A sequence  ", "");
    }
    if app.pool('m').is_empty() {
        shortcuts = shortcuts.replace("M matrix  ", "");
    }
    if app.pool('s').is_empty() {
        shortcuts = shortcuts.replace("S screensaver  ", "");
    }
    f.render_widget(
        Paragraph::new(shortcuts).style(Style::default().fg(Color::Gray)),
        rows[4],
    );
    if app.settings {
        settings(f, app);
    }
    if app.preset_open {
        presets(f, app);
    }
}
fn presets(f: &mut Frame, app: &App) {
    let area = f.area();
    let width = area.width.saturating_sub(4).min(88);
    let height = area.height.saturating_sub(2).min(24);
    let rect = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    f.render_widget(Clear, rect);
    let outer = block(" Workspace presets ", accent(app))
        .style(Style::default().bg(Color::Rgb(20, 26, 40)));
    let inner = outer.inner(rect);
    f.render_widget(outer, rect);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(5),
        Constraint::Length(3),
    ])
    .split(inner);
    f.render_widget(
        Paragraph::new("↑↓ select · Enter restore · N save current workspace · Esc close")
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(accent(app))),
        rows[0],
    );
    let items: Vec<_> = app
        .presets
        .iter()
        .map(|p| {
            ListItem::new(format!(
                "▦ {}  · {} windows · workspace {}",
                p.name,
                p.windows.len(),
                p.workspace
            ))
        })
        .collect();
    let mut state = ListState::default().with_selected(if items.is_empty() {
        None
    } else {
        Some(app.preset_selected)
    });
    f.render_stateful_widget(
        List::new(items)
            .highlight_style(Style::default().fg(accent(app)).bold())
            .highlight_symbol("▸ "),
        rows[1],
        &mut state,
    );
    let detail = if let Some(p) = app.presets.get(app.preset_selected) {
        format!(
            "{}\nMonitor: {} · {} × {}\nReuses matching windows; launches missing programs.\n{}",
            p.windows
                .iter()
                .map(|w| w.label.as_str())
                .collect::<Vec<_>>()
                .join(" · "),
            p.monitor.name,
            p.monitor.size[0],
            p.monitor.size[1],
            p.notes.join("; ")
        )
    } else {
        "No presets yet. Press N to save this workspace.\nCaptures recognized art programs in Kitty and desktop window positions. Requires Hyprland.".into()
    };
    f.render_widget(Paragraph::new(detail).wrap(Wrap { trim: true }), rows[2]);
    let status = app
        .preset_name
        .as_ref()
        .map(|name| format!("Save current workspace as: {name}▏\nEnter saves · Esc cancels"))
        .unwrap_or_else(|| app.message.clone());
    f.render_widget(
        Paragraph::new(status)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(accent(app))),
        rows[3],
    );
}
fn settings(f: &mut Frame, app: &App) {
    let area = f.area();
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(58)) / 2,
        area.y + (area.height.saturating_sub(14)) / 2,
        58.min(area.width),
        14.min(area.height),
    );
    f.render_widget(Clear, rect);
    let values = [
        format!("Colors              {}", THEMES[app.config.theme]),
        format!(
            "Animation speed     {} ({}s / effect)",
            ["Slow", "Normal", "Fast"][app.config.speed],
            app.config.seconds()
        ),
        format!(
            "Startup splash      {}",
            if app.config.splash { "On" } else { "Off" }
        ),
        format!(
            "Startup behavior    {}",
            [
                "All programs",
                "Favorites",
                "Recent history",
                "Random program"
            ][app.config.startup]
        ),
        "Manage favorites    Open Favorites collection".into(),
        "Clear favorites     Press Enter to clear".into(),
    ];
    let mut lines = vec![
        Line::from("↑↓ select · ←→ / Enter change · Esc save & close"),
        Line::from(""),
    ];
    for (i, v) in values.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!("{} {v}", if i == app.setting { "▸" } else { " " }),
            Style::default().fg(if i == app.setting {
                accent(app)
            } else {
                Color::Gray
            }),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Speed controls splash and automatic playback timing.",
    ));
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(block(" Configuration ", accent(app)))
            .style(Style::default().bg(Color::Rgb(20, 26, 40))),
        rect,
    );
}
pub fn splash(f: &mut Frame, phase: usize, color: Color) {
    let area = f.area();
    let lines = vec![
        Line::from(""),
        Line::from("     ▄▀█  █▀█  ▀█▀"),
        Line::from("     █▀█  █▀▄   █ "),
        Line::from(""),
        Line::from("T E R M I N A L   G A L L E R Y"),
        Line::from(""),
        Line::from(format!(
            "{}  discovering your canvas",
            ["◐", "◓", "◑", "◒"][phase % 4]
        )),
    ];
    let rect = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(9) / 2,
        area.width,
        9.min(area.height),
    );
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(13, 17, 27))),
        area,
    );
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color)),
        rect,
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_small_and_large_terminals() {
        for (w, h) in [(40, 10), (64, 18), (100, 30), (160, 50)] {
            let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            let mut app = App::new(crate::config::Config::default());
            terminal.draw(|f| draw(f, &app)).unwrap();
            app.settings = true;
            terminal.draw(|f| draw(f, &app)).unwrap();
        }
    }
}
