//! Curated launch recipes. Commands and packages never originate in user input.
use std::{env, os::unix::fs::PermissionsExt, path::PathBuf};

#[derive(Debug)]
pub struct Entry {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub executable: &'static str,
    pub package: &'static str,
    /// A manual source-build hint when no verified Arch package exists.
    pub install_hint: Option<&'static str>,
    pub description: &'static str,
    pub args: &'static [&'static str],
    pub input: Option<&'static str>,
    pub continuous: bool,
    pub cycle: bool,
    pub matrix: bool,
}
macro_rules! entry {
    ($id:literal,$cat:literal,$exe:literal,$pkg:literal,$desc:literal,$args:expr,$input:expr,$continuous:expr,$cycle:expr,$matrix:expr) => {
        Entry {
            id: $id,
            title: $id,
            category: $cat,
            executable: $exe,
            package: $pkg,
            install_hint: None,
            description: $desc,
            args: $args,
            input: $input,
            continuous: $continuous,
            cycle: $cycle,
            matrix: $matrix,
        }
    };
}
pub const ENTRIES: &[Entry] = &[
    entry!("cbonsai","Animations","cbonsai","cbonsai","Grow a living ASCII bonsai. Watch branches unfold in a tiny terminal garden.",&["-l"],None,true,true,false),
    entry!("cmatrix","Animations","cmatrix","cmatrix","The classic green digital rain. A quiet portal into the Matrix.",&["-b"],None,true,true,true),
    entry!("unimatrix","Animations","unimatrix","unimatrix-git","Unicode digital rain with a rich mix of glyphs and cascading trails.",&[],None,true,true,true),
    entry!("pipes.sh","Animations","pipes.sh","pipes.sh","Endlessly winding colorful pipes, drawn across your terminal.",&[],None,true,true,false),
    entry!("asciiquarium","Animations","asciiquarium","asciiquarium","A peaceful ASCII ocean full of fish, boats, and underwater surprises.",&[],None,true,true,false),
    entry!("aafire","Animations","aafire","aalib","A flickering fire rendered entirely in ASCII characters.",&[],None,true,true,false),
    entry!("oneko","Animations","oneko","oneko","A little cat follows your pointer. Requires an X11 display (or XWayland); excluded from terminal playback modes.",&[],None,true,false,false),
    entry!("fastfetch","System","fastfetch","fastfetch","A colorful snapshot of your operating system, hardware, and desktop.",&[],None,false,false,false),
    entry!("btop","System","btop","btop","An interactive system monitor with beautiful CPU, memory, disk, and network graphs.",&[],None,true,false,false),
    entry!("htop","System","htop","htop","An interactive process viewer. Inspect processes and resource usage.",&[],None,true,false,false),
    entry!("nvtop","System","nvtop","nvtop","Live GPU activity and process monitoring; hardware support depends on your driver.",&[],None,true,false,false),
    entry!("figlet","Text","figlet","figlet","Turn words into oversized ASCII lettering. Launches with a TERMINAL ART sample.",&["TERMINAL ART"],None,false,false,false),
    entry!("toilet","Text","toilet","toilet","Bold terminal typography with colorful effects.",&["-F","gay","ART"],None,false,false,false),
    entry!("cowsay","Text","cowsay","cowsay","A friendly ASCII cow with a message for your terminal.",&["Make room for a little terminal art."],None,false,false,false),
    entry!("ponysay","Text","ponysay","ponysay","Colorful pony illustrations deliver a cheerful message.",&["Your terminal is a canvas."],None,false,false,false),
    entry!("fortune","Text","fortune","fortune-mod","A small dose of wisdom, wit, or delightful nonsense.",&[],None,false,false,false),
    entry!("lolcat","Text","lolcat","lolcat","Paint text with a rainbow gradient. Includes sample text so it never waits on an empty pipe.",&[],Some("Your terminal is a canvas.\nCreate something wonderful.\n     T E R M I N A L   A R T\n"),false,false,false),
    entry!("cava","Visualizers","cava","cava","A real-time spectrum analyzer for your audio. Requires a working audio source.",&[],None,true,true,false),
    entry!("wttr.in","Utilities","curl","curl","Weather in your terminal from wttr.in. Requires internet access; your request is sent to that service.",&["--fail","--location","--max-time","15","https://wttr.in?0"],None,false,false,false),
    entry!("tty-clock","Utilities","tty-clock","tty-clock","A crisp digital clock for your terminal.",&["-c","-C","6"],None,true,true,false),
    entry!("astroterm","Space","astroterm","astroterm","An animated planetarium: stars, planets, and constellation lines sweep across the sky. Runs an accelerated equatorial sky view; Q or Esc returns.",&["--color","--constellations","--unicode","--speed","10000"],None,true,true,false),
    entry!("globe","Space","globe","globe-cli","A rotating ASCII Earth with a night side and an orbiting camera. A tiny planet floating in your terminal. Ctrl-C returns.",&["-s","-n","-c","2","-g","10"],None,true,true,false),
    Entry {
        install_hint: Some("Build Haruno19/starfetch from source; see README Space section."),
        ..entry!("starfetch","Space","starfetch","starfetch","Colorful constellation art and astronomy facts. Each launch selects a random constellation; output stays visible until Enter.",&["-c","cyan"],None,false,false,false)
    },
    Entry {
        install_hint: Some("Use pipx install terrascope, or upstream's packaging/arch/PKGBUILD; no published Arch/AUR package yet."),
        ..entry!("terrascope","Space","terrascope","terrascope","Interactive Braille Earth map with weather, aircraft, earthquakes, and day/night layers. First-run map downloads and live layers use the network. Q quits.",&["--no-redefine-palette"],None,true,false,false)
    },
];
pub const CATEGORIES: &[&str] = &[
    "All",
    "Favorites",
    "Recent",
    "Animations",
    "System",
    "Text",
    "Visualizers",
    "Utilities",
    "Space",
];

pub fn which(name: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|p| p.join(name))
        .find(|p| {
            p.is_file()
                && p.metadata()
                    .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        })
}
pub fn usable(entry: &Entry) -> bool {
    which(entry.executable).is_some() && (entry.id != "oneko" || env::var_os("DISPLAY").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_unique_ids_and_safe_packages() {
        let mut ids = std::collections::HashSet::new();
        for e in ENTRIES {
            assert!(ids.insert(e.id));
            assert!(CATEGORIES.contains(&e.category));
            assert!(e
                .package
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._+".contains(c)));
            assert!(!e.matrix || e.cycle);
        }
        assert_eq!(ENTRIES.len(), 24);
    }
}
