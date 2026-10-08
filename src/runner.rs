//! Child processes inherit the terminal only while the UI is suspended.
use crate::catalog::{which, Entry};
use anyhow::{bail, Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal,
};
use nix::{
    sys::signal::{killpg, sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
    unistd::Pid,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    io::Write,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub fn command(entry: &Entry) -> Result<Command> {
    let path = which(entry.executable).context("Executable disappeared; press U to refresh")?;
    let mut cmd = Command::new(path);
    cmd.args(entry.args);
    Ok(cmd)
}
pub fn launch(entry: &Entry, terminated: &AtomicBool) -> Result<()> {
    let mut cmd = command(entry)?;
    if entry.input.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().context("Could not launch program")?;
    if let Some(input) = entry.input {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
    }
    let status = wait_child(&mut child, terminated)?;
    if !entry.continuous || !status.success() {
        println!("\nProgram finished ({status}). Press Enter to return to art.");
        terminal::enable_raw_mode()?;
        while !terminated.load(Ordering::Relaxed) {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Esc) {
                        break;
                    }
                }
            }
        }
        terminal::disable_raw_mode()?;
    }
    if !status.success() {
        bail!("{} exited with {status}", entry.title);
    }
    Ok(())
}

/// A separate process group lets playback stop the entire effect, including scripts.
struct Effect(Child);
impl Drop for Effect {
    fn drop(&mut self) {
        let group = Pid::from_raw(self.0.id() as i32);
        let _ = killpg(group, Signal::SIGTERM);
        std::thread::sleep(Duration::from_millis(80));
        let _ = killpg(group, Signal::SIGKILL);
        let _ = self.0.wait();
    }
}
/// Returns false when the user requests an end to the whole sequence.
pub fn effect(entry: &Entry, seconds: u64, terminated: &AtomicBool) -> Result<bool> {
    let mut cmd = command(entry)?;
    cmd.stdin(Stdio::null()).process_group(0);
    // Keep keyboard ownership in the launcher, but allow the background effect
    // to configure and draw on the controlling terminal without being stopped.
    // SAFETY: this post-fork hook only invokes async-signal-safe sigaction calls;
    // it neither allocates nor touches locks. Ignored dispositions survive exec.
    unsafe {
        cmd.pre_exec(|| {
            let ignore = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
            sigaction(Signal::SIGTTOU, &ignore).map_err(std::io::Error::from)?;
            sigaction(Signal::SIGTTIN, &ignore).map_err(std::io::Error::from)?;
            Ok(())
        });
    }
    let mut child = Effect(cmd.spawn()?);
    let start = Instant::now();
    loop {
        if terminated.load(Ordering::Relaxed) {
            return Ok(false);
        }
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(false),
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(false)
                    }
                    KeyCode::Right | KeyCode::Char(' ') => return Ok(true),
                    _ => {}
                }
            }
        }
        if start.elapsed() >= Duration::from_secs(seconds) {
            return Ok(true);
        }
        if let Some(status) = child.0.try_wait()? {
            if !status.success() {
                bail!("{} failed ({status})", entry.title);
            }
            return Ok(true);
        }
    }
}

pub fn manager() -> Option<&'static str> {
    ["paru", "yay", "pacman"]
        .into_iter()
        .find(|m| which(m).is_some())
}
pub fn install(entry: &Entry, terminated: &AtomicBool) -> Result<()> {
    if let Some(hint) = entry.install_hint {
        bail!("{hint}");
    }
    if manager() == Some("pacman") && requires_aur(entry) {
        bail!(
            "{} is an AUR package; install yay or paru first",
            entry.package
        );
    }
    install_packages(&[entry.package], terminated)?;
    if which(entry.executable).is_none() {
        bail!(
            "Package completed but {} is still absent from PATH",
            entry.executable
        );
    }
    Ok(())
}

fn install_packages(packages: &[&str], terminated: &AtomicBool) -> Result<()> {
    let manager = manager().context("No supported package manager found (paru, yay, pacman)")?;
    println!(
        "Installing {} with {manager}. Review the package manager prompts below.\n",
        packages.join(" ")
    );
    let mut command = if manager == "pacman" {
        if nix::unistd::Uid::effective().is_root() {
            Command::new("pacman")
        } else {
            let mut c =
                Command::new(which("sudo").context("pacman requires sudo; sudo was not found")?);
            c.arg("pacman");
            c
        }
    } else {
        Command::new(manager)
    };
    // Keep native confirmations and AUR review prompts. Never run an AUR helper as root.
    if manager != "pacman" && nix::unistd::Uid::effective().is_root() {
        bail!("Run art as a regular user to use an AUR helper");
    }
    let mut child = command
        .args(["-S", "--needed", "--"])
        .args(packages)
        .spawn()?;
    let status = wait_child(&mut child, terminated)?;
    if !status.success() {
        bail!("Installation failed or was cancelled ({status}); press U to retry");
    }
    Ok(())
}
pub fn requires_aur(entry: &Entry) -> bool {
    matches!(
        entry.package,
        "cbonsai" | "globe-cli" | "oneko" | "pipes.sh" | "tty-clock" | "unimatrix-git"
    )
}
/// Bulk installs retain the package manager's native confirmation/review prompts.
pub fn install_selection(args: &[String]) -> Result<()> {
    use crate::catalog::{CATEGORIES, ENTRIES};
    let dry = args.iter().any(|s| s == "--dry-run");
    let selected: Vec<_> = args.iter().filter(|s| s.as_str() != "--dry-run").collect();
    anyhow::ensure!(
        selected.len() == 1,
        "Usage: art install [--all | CATEGORY | PROGRAM] [--dry-run]"
    );
    let query = selected[0].to_lowercase();
    anyhow::ensure!(
        query == "--all"
            || ENTRIES.iter().any(|e| e.id.eq_ignore_ascii_case(&query))
            || CATEGORIES[3..]
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&query)),
        "Unknown category or program: {query}"
    );
    let manager = manager().context("Install requires pacman, yay, or paru on Arch Linux")?;
    let mut packages = std::collections::BTreeSet::new();
    let mut entries = vec![];
    for e in ENTRIES.iter().filter(|e| {
        query == "--all"
            || e.id.eq_ignore_ascii_case(&query)
            || e.category.eq_ignore_ascii_case(&query)
    }) {
        if which(e.executable).is_some() {
            println!("{}: already installed", e.title);
            continue;
        }
        if let Some(hint) = e.install_hint {
            println!("{}: skipped — {hint}", e.title);
            continue;
        }
        if manager == "pacman" && requires_aur(e) {
            println!("{}: skipped — requires yay or paru (AUR)", e.title);
            continue;
        }
        packages.insert(e.package);
        entries.push(e);
    }
    if packages.is_empty() {
        println!("No installable missing packages in this selection.");
        return Ok(());
    }
    let packages: Vec<_> = packages.into_iter().collect();
    println!("{manager} -S --needed -- {}", packages.join(" "));
    if dry {
        println!("Dry run: no changes made.");
        return Ok(());
    }
    install_packages(&packages, &AtomicBool::new(false))?;
    let missing: Vec<_> = entries
        .iter()
        .filter(|e| which(e.executable).is_none())
        .map(|e| e.executable)
        .collect();
    anyhow::ensure!(
        missing.is_empty(),
        "Installation finished but these executables are absent from PATH: {}",
        missing.join(", ")
    );
    Ok(())
}

/// Poll to allow orderly terminal restoration after a termination request.
fn wait_child(child: &mut Child, terminated: &AtomicBool) -> Result<std::process::ExitStatus> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if terminated.load(Ordering::Relaxed) {
            child.kill()?;
            return Ok(child.wait()?);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
