//! Saved Hyprland/Kitty workspace compositions. Captures argv, never shell text.
use crate::{
    catalog::{which, ENTRIES},
    config::Config,
};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub version: u32,
    pub name: String,
    pub workspace: i64,
    pub monitor: Monitor,
    pub windows: Vec<Window>,
    pub notes: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Monitor {
    pub name: String,
    pub origin: [i64; 2],
    pub size: [i64; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Window {
    pub label: String,
    pub class: String,
    pub source_pid: u64,
    pub position: [i64; 2],
    pub size: [i64; 2],
    pub floating: bool,
    pub terminal: bool,
    /// Empty argv means reuse-only (for example a Spotify-owned mini-player).
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub kitty_config: Option<String>,
    pub cava_config: Option<String>,
}
fn output(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("Could not run {program}"))?;
    ensure!(
        out.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(String::from_utf8(out.stdout)?)
}
fn hypr_json(what: &str) -> Result<Value> {
    Ok(serde_json::from_str(&output("hyprctl", &[what, "-j"])?)?)
}
fn eval(code: &str) -> Result<()> {
    let result = output("hyprctl", &["eval", code])?;
    ensure!(
        !result.to_lowercase().contains("error"),
        "Hyprland: {result}"
    );
    Ok(())
}
fn pair(value: &Value) -> Result<[i64; 2]> {
    Ok([
        value[0].as_i64().context("Missing coordinate")?,
        value[1].as_i64().context("Missing coordinate")?,
    ])
}
fn monitor(v: &Value) -> Result<Monitor> {
    let scale = v["scale"].as_f64().unwrap_or(1.0);
    ensure!(scale > 0.0, "Invalid monitor scale");
    let mut size = [
        v["width"].as_i64().context("Missing monitor width")?,
        v["height"].as_i64().context("Missing monitor height")?,
    ];
    if v["transform"].as_i64().unwrap_or(0) % 2 == 1 {
        size.swap(0, 1);
    }
    Ok(Monitor {
        name: v["name"].as_str().context("Missing monitor name")?.into(),
        origin: [v["x"].as_i64().unwrap_or(0), v["y"].as_i64().unwrap_or(0)],
        size: [
            (size[0] as f64 / scale) as i64,
            (size[1] as f64 / scale) as i64,
        ],
    })
}
pub fn available() -> bool {
    which("hyprctl").is_some() && std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some()
}
pub fn directory() -> Result<PathBuf> {
    Ok(Config::path()?
        .parent()
        .context("Missing config parent")?
        .join("presets"))
}
fn slug(name: &str) -> Result<String> {
    ensure!(
        !name.trim().is_empty() && name.len() <= 80,
        "Preset name must contain 1–80 characters"
    );
    ensure!(
        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || " _-".contains(c)),
        "Use letters, numbers, spaces, hyphens, or underscores in preset names"
    );
    Ok(name.trim().to_ascii_lowercase().replace(' ', "-"))
}
impl Preset {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported preset version {}",
            self.version
        );
        slug(&self.name)?;
        ensure!(self.workspace > 0, "Only numbered workspaces are supported");
        ensure!(
            self.monitor.size.iter().all(|v| *v > 0),
            "Invalid monitor size"
        );
        ensure!(
            !self.windows.is_empty() && self.windows.len() <= 64,
            "Preset must contain 1–64 windows"
        );
        for w in &self.windows {
            ensure!(w.size.iter().all(|v| *v > 0), "Invalid window size");
            ensure!(
                !w.terminal || !w.command.is_empty(),
                "Terminal has no command"
            );
            ensure!(
                w.command.iter().all(|s| !s.contains('\0')),
                "Invalid command argument"
            );
        }
        Ok(())
    }
    pub fn save(&self, replace: bool) -> Result<PathBuf> {
        self.validate()?;
        let dir = directory()?;
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", slug(&self.name)?));
        let bytes = serde_json::to_vec_pretty(self)?;
        if replace {
            let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
            fs::write(&tmp, bytes)?;
            fs::rename(tmp, &path)?;
        } else {
            use std::io::Write;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .with_context(|| {
                    format!(
                        "Preset already exists or cannot be saved: {} (use --replace to overwrite)",
                        path.display()
                    )
                })?;
            file.write_all(&bytes)?;
        }
        Ok(path)
    }
}
pub fn list() -> Result<Vec<Preset>> {
    let dir = directory()?;
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut presets = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let preset: Preset = serde_json::from_str(&fs::read_to_string(&path)?)
                .with_context(|| format!("Invalid preset {}", path.display()))?;
            preset.validate()?;
            presets.push(preset);
        }
    }
    presets.sort_by_key(|p| p.name.to_lowercase());
    Ok(presets)
}
pub fn load(name: &str) -> Result<Preset> {
    let path = directory()?.join(format!("{}.json", slug(name)?));
    let p: Preset = serde_json::from_str(
        &fs::read_to_string(&path).with_context(|| format!("Preset not found: {name}"))?,
    )?;
    p.validate()?;
    Ok(p)
}
fn socket(pid: u64) -> Result<String> {
    let sockets = fs::read_to_string("/proc/net/unix")?;
    let suffix = format!("-{pid}");
    let path = sockets
        .lines()
        .filter_map(|l| l.split_whitespace().nth(7))
        .find(|s| s.ends_with(&suffix))
        .context(
            "Kitty remote socket not found; enable allow_remote_control and listen_on in Kitty",
        )?;
    Ok(format!("unix:{path}"))
}
fn kitty(pid: u64, args: &[&str]) -> Result<String> {
    let sock = socket(pid)?;
    let mut all = vec!["@", "--to", sock.as_str()];
    all.extend_from_slice(args);
    output("kitty", &all)
}
fn terminal_capture(client: &Value) -> Result<Option<Window>> {
    let pid = client["pid"].as_u64().context("Missing PID")?;
    let data: Value = serde_json::from_str(&kitty(pid, &["ls"])?)?;
    let tabs = data[0]["tabs"].as_array().context("Kitty tabs missing")?;
    ensure!(tabs.len()==1 && tabs[0]["windows"].as_array().is_some_and(|w|w.len()==1),"Split/tabbed Kitty windows need a Kitty session; capture currently supports one program per OS window");
    let term = &tabs[0]["windows"][0];
    let foreground = term["foreground_processes"]
        .as_array()
        .context("Foreground processes missing")?;
    let found = foreground.iter().rev().find_map(|p| {
        let argv: Vec<String> = p["cmdline"]
            .as_array()?
            .iter()
            .filter_map(|s| s.as_str().map(str::to_owned))
            .collect();
        // Resolve an interpreter wrapper to the actual catalog executable.
        let offset = argv.iter().take(2).position(|arg| {
            ENTRIES
                .iter()
                .any(|e| Path::new(arg).file_name().and_then(|s| s.to_str()) == Some(e.executable))
        })?;
        Some((
            argv[offset..].to_vec(),
            p["cwd"].as_str().unwrap_or("/").to_owned(),
        ))
    });
    let Some((command, cwd)) = found else {
        return Ok(None);
    };
    let label = Path::new(&command[0])
        .file_name()
        .context("Invalid executable")?
        .to_string_lossy()
        .into_owned();
    let mut colors = kitty(pid, &["get-colors"])?;
    colors.push_str(&format!(
        "\nbackground_opacity {}\n",
        data[0]["background_opacity"].as_f64().unwrap_or(1.0)
    ));
    if let Some(dir) = dirs::config_dir() {
        if let Ok(conf) = fs::read_to_string(dir.join("kitty/kitty.conf")) {
            for line in conf.lines() {
                if line.split_whitespace().next().is_some_and(|k| {
                    ["font_family", "font_size", "window_padding_width"].contains(&k)
                }) {
                    colors.push_str(line);
                    colors.push('\n');
                }
            }
        }
    }
    let cava_config = if label == "cava" {
        let path = command
            .iter()
            .position(|a| a == "-p")
            .and_then(|i| command.get(i + 1))
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("cava/config")));
        path.filter(|p| p.is_file())
            .map(fs::read_to_string)
            .transpose()?
    } else {
        None
    };
    Ok(Some(Window {
        label,
        class: client["class"].as_str().unwrap_or("").into(),
        source_pid: pid,
        position: pair(&client["at"])?,
        size: pair(&client["size"])?,
        floating: client["floating"].as_bool().unwrap_or(true),
        terminal: true,
        command,
        cwd: cwd.into(),
        kitty_config: Some(colors),
        cava_config,
    }))
}
pub fn capture(name: &str, workspace: Option<i64>) -> Result<Preset> {
    slug(name)?;
    ensure!(
        available(),
        "Workspace presets require Hyprland and Kitty remote control"
    );
    let workspace = workspace.unwrap_or(
        hypr_json("activeworkspace")?["id"]
            .as_i64()
            .context("Active workspace missing")?,
    );
    let clients = hypr_json("clients")?;
    let mut clients: Vec<_> = clients
        .as_array()
        .context("Invalid clients response")?
        .iter()
        .filter(|c| c["workspace"]["id"].as_i64() == Some(workspace))
        .collect();
    ensure!(!clients.is_empty(), "Workspace {workspace} is empty");
    clients.sort_by_key(|c| {
        (
            c["floating"].as_bool().unwrap_or(false),
            -c["focusHistoryID"].as_i64().unwrap_or(0),
        )
    });
    let monitors = hypr_json("monitors")?;
    let source_monitor = monitors
        .as_array()
        .context("Invalid monitors response")?
        .iter()
        .find(|m| m["id"] == clients[0]["monitor"])
        .context("Monitor not found")?;
    let mut preset = Preset {
        version: 1,
        name: name.trim().into(),
        workspace,
        monitor: monitor(source_monitor)?,
        windows: vec![],
        notes: vec![],
    };
    for c in clients {
        let pid = c["pid"].as_u64().context("Missing PID")?;
        let exe = fs::read_link(format!("/proc/{pid}/exe")).unwrap_or_default();
        if exe.file_name().and_then(|s| s.to_str()) == Some("kitty") {
            match terminal_capture(c)? {
                Some(w) => preset.windows.push(w),
                None => preset.notes.push(format!(
                    "Skipped terminal {pid}: no recognized art program running"
                )),
            }
        } else {
            let class = c["class"].as_str().unwrap_or("").to_owned();
            let is_spotify = exe.file_name().and_then(|s| s.to_str()) == Some("spotify");
            let main = is_spotify && class.eq_ignore_ascii_case("spotify");
            let label = if main {
                "Spotify".into()
            } else if is_spotify {
                "Spotify mini-player".into()
            } else {
                class.clone()
            };
            if !main {
                preset.notes.push(format!(
                    "{label}: reuse existing window; reopen it in its app if closed"
                ));
            }
            preset.windows.push(Window {
                label,
                class,
                source_pid: pid,
                position: pair(&c["at"])?,
                size: pair(&c["size"])?,
                floating: c["floating"].as_bool().unwrap_or(true),
                terminal: false,
                command: if main {
                    vec![exe.to_string_lossy().into_owned()]
                } else {
                    vec![]
                },
                cwd: dirs::home_dir().unwrap_or_else(|| "/".into()),
                kitty_config: None,
                cava_config: None,
            });
        }
    }
    preset.validate()?;
    Ok(preset)
}
fn target_class(p: &Preset, index: usize) -> Result<String> {
    Ok(format!("art-preset-{}-{index}", slug(&p.name)?))
}
fn matches(c: &Value, w: &Window, class: &str) -> bool {
    if w.terminal {
        c["class"].as_str() == Some(class)
            || (c["pid"].as_u64() == Some(w.source_pid) && c["class"].as_str() == Some(&w.class))
    } else {
        c["class"].as_str() == Some(&w.class)
            && (w.class.eq_ignore_ascii_case("spotify")
                || c["pid"].as_u64() == Some(w.source_pid)
                || fs::read_link(format!("/proc/{}/exe", c["pid"]))
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n == "spotify"))
                    .unwrap_or(false))
    }
}
fn find(w: &Window, class: &str) -> Result<Option<Value>> {
    let all = hypr_json("clients")?;
    Ok(all
        .as_array()
        .context("Invalid client list")?
        .iter()
        .find(|c| matches(c, w, class))
        .cloned())
}
/// Keep the art program idle until its terminal has reached its final size.
/// Closing this gate without releasing it never starts the requested command.
struct PendingStart {
    listener: std::os::unix::net::UnixListener,
    path: PathBuf,
}
impl PendingStart {
    fn new() -> Result<Self> {
        use std::os::unix::fs::PermissionsExt;
        let runtime = dirs::runtime_dir().unwrap_or_else(std::env::temp_dir);
        let path = runtime.join(format!(
            "art-{}-{:016x}.sock",
            std::process::id(),
            rand::random::<u64>()
        ));
        let listener = std::os::unix::net::UnixListener::bind(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, path })
    }
    fn release(&self) -> Result<()> {
        use std::io::Write;
        for _ in 0..100 {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    stream.write_all(b"G")?;
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50))
                }
                Err(e) => return Err(e.into()),
            }
        }
        bail!("New terminal did not connect to its launch gate")
    }
}
impl Drop for PendingStart {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
pub fn preset_exec(args: &[String]) -> Result<()> {
    use std::{
        io::Read,
        os::unix::{net::UnixStream, process::CommandExt},
    };
    ensure!(
        args.len() >= 2,
        "Internal preset command needs a socket and executable"
    );
    let mut stream = UnixStream::connect(&args[0])?;
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    let mut byte = [0];
    stream.read_exact(&mut byte)?;
    ensure!(byte[0] == b'G', "Preset launch cancelled");
    drop(stream);
    // Give the terminal a moment to finish processing its final configure event.
    thread::sleep(Duration::from_millis(150));
    Err(Command::new(&args[1]).args(&args[2..]).exec().into())
}
/// Generate no operations for an already-correct window. In particular, never
/// toggle floating state or resize a running animation just to reapply a preset.
fn placement_code(
    client: &Value,
    w: &Window,
    target: i64,
    at: [i64; 2],
    size: [i64; 2],
) -> Result<String> {
    let address = client["address"]
        .as_str()
        .context("Window address missing")?;
    ensure!(
        address.starts_with("0x")
            && address.len() > 2
            && address[2..].chars().all(|c| c.is_ascii_hexdigit()),
        "Invalid window address"
    );
    let selector = format!("window='address:{address}'");
    let mut code = String::new();
    let moved = client["workspace"]["id"].as_i64() != Some(target);
    if moved {
        code.push_str(&format!(
            "hl.dispatch(hl.dsp.window.move({{{selector},workspace={target},follow=false}}));"
        ));
    }
    let changed = client["floating"].as_bool() != Some(w.floating);
    if changed {
        code.push_str(&format!(
            "hl.dispatch(hl.dsp.window.float({{{selector},action='{}'}}));",
            if w.floating { "enable" } else { "disable" }
        ));
    }
    if w.floating {
        if changed || pair(&client["size"])? != size {
            code.push_str(&format!(
                "hl.dispatch(hl.dsp.window.resize({{{selector},x={},y={},relative=false}}));",
                size[0], size[1]
            ));
        }
        if moved || changed || pair(&client["at"])? != at {
            code.push_str(&format!(
                "hl.dispatch(hl.dsp.window.move({{{selector},x={},y={},relative=false}}));",
                at[0], at[1]
            ));
        }
    }
    Ok(code)
}
fn spawn(p: &Preset, w: &Window, index: usize, class: &str) -> Result<Option<PendingStart>> {
    let executable = w.command.first().context("No launch command")?;
    ensure!(which(executable).is_some(), "{} is not installed", w.label);
    let mut argv = w.command.clone();
    let assets = directory()?.join(format!("{}-assets", slug(&p.name)?));
    fs::create_dir_all(&assets)?;
    if let Some(config) = &w.cava_config {
        let path = assets.join(format!("{index}-cava.conf"));
        fs::write(&path, config)?;
        if let Some(i) = argv.iter().position(|a| a == "-p") {
            argv.truncate(i);
        }
        argv.extend(["-p".into(), path.to_string_lossy().into_owned()]);
    }
    let pending = if w.terminal {
        Some(PendingStart::new()?)
    } else {
        None
    };
    let mut command = if w.terminal {
        // New art terminals start floating, so opening them cannot retile and
        // resize an already-running background effect. Scoped to our class.
        eval(&format!("_G.art_preset_rules=_G.art_preset_rules or {{}}; if not _G.art_preset_rules['{class}'] then hl.window_rule({{match={{class='^{class}$'}},float=true}}); _G.art_preset_rules['{class}']=true end"))?;
        let mut cmd = Command::new("kitty");
        cmd.args([
            "--class",
            class,
            "--title",
            &format!("{} · {}", p.name, w.label),
        ]);
        if let Some(config) = &w.kitty_config {
            let path = assets.join(format!("{index}-kitty.conf"));
            fs::write(&path, config)?;
            if let Some(default) = dirs::config_dir()
                .map(|d| d.join("kitty/kitty.conf"))
                .filter(|p| p.is_file())
            {
                cmd.arg("--config").arg(default);
            }
            cmd.arg("--config").arg(path);
        }
        cmd.arg("--")
            .arg(std::env::current_exe()?)
            .arg("__preset-exec")
            .arg(&pending.as_ref().context("Missing launch gate")?.path)
            .args(&argv);
        cmd
    } else {
        let mut cmd = Command::new(executable);
        cmd.args(&argv[1..]);
        cmd
    };
    use std::os::unix::process::CommandExt;
    command
        .current_dir(&w.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command.spawn()?;
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pending)
}
fn geometry(w: &Window, from: &Monitor, to: &Monitor) -> ([i64; 2], [i64; 2]) {
    let mut at = [0; 2];
    let mut size = [0; 2];
    for i in 0..2 {
        let ratio = to.size[i] as f64 / from.size[i] as f64;
        at[i] = to.origin[i] + ((w.position[i] - from.origin[i]) as f64 * ratio).round() as i64;
        size[i] = (w.size[i] as f64 * ratio).round().max(1.0) as i64;
    }
    (at, size)
}
/// Restore only saved windows. Existing unrelated windows are never closed.
pub fn launch(p: &Preset, workspace: Option<i64>) -> Result<String> {
    p.validate()?;
    ensure!(available(), "Workspace presets require Hyprland");
    let target = workspace.unwrap_or(p.workspace);
    ensure!(target > 0, "Invalid target workspace");
    let monitors = hypr_json("monitors")?;
    let monitors = monitors.as_array().context("Invalid monitor list")?;
    let dest = monitor(
        monitors
            .iter()
            .find(|m| m["name"].as_str() == Some(&p.monitor.name))
            .or_else(|| monitors.iter().find(|m| m["focused"] == true))
            .or_else(|| monitors.first())
            .context("No monitor found")?,
    )?;
    let mut restored = 0;
    let mut warnings = vec![];
    for (index, w) in p.windows.iter().enumerate() {
        let class = target_class(p, index)?;
        let mut client = find(w, &class)?;
        let mut pending = None;
        if client.is_none() && !w.command.is_empty() {
            match spawn(p, w, index, &class) {
                Ok(gate) => pending = gate,
                Err(e) => {
                    warnings.push(e.to_string());
                    continue;
                }
            }
            for _ in 0..50 {
                thread::sleep(Duration::from_millis(100));
                client = find(w, &class)?;
                if client.is_some() {
                    break;
                }
            }
        }
        let Some(client) = client else {
            warnings.push(format!(
                "{}: open it in its app, then reapply preset",
                w.label
            ));
            continue;
        };
        let (at, size) = geometry(w, &p.monitor, &dest);
        let code = placement_code(&client, w, target, at, size)?;
        if !code.is_empty() {
            eval(&code)?;
        }
        if let Some(gate) = pending {
            gate.release()?;
        }
        restored += 1;
    }
    if hypr_json("activeworkspace")?["id"].as_i64() != Some(target) {
        eval(&format!(
            "hl.dispatch(hl.dsp.focus({{workspace={target}}}))"
        ))?;
    }
    Ok(format!(
        "{}: {restored}/{} windows restored on workspace {target}. {}",
        p.name,
        p.windows.len(),
        warnings.join("; ")
    ))
}
/// Non-interactive entry point, also used by scripts and desktop shortcuts.
pub fn cli(args: &[String]) -> Result<()> {
    let action = args.first().map(String::as_str).unwrap_or("list");
    let workspace = args
        .iter()
        .position(|s| s == "--workspace")
        .map(|i| {
            args.get(i + 1)
                .context("Missing workspace number")?
                .parse::<i64>()
                .context("Invalid workspace number")
        })
        .transpose()?;
    match action {
        "list"=>for p in list()? {println!("{} — {} windows, workspace {}",p.name,p.windows.len(),p.workspace);},
        "save"=>{let p=capture(args.get(1).context("Usage: art preset save NAME [--workspace N] [--replace]")?,workspace)?;let path=p.save(args.iter().any(|s|s=="--replace"))?;println!("Saved {} ({} windows) to {}",p.name,p.windows.len(),path.display());for note in p.notes {println!("  {note}");}},
        "launch"=>println!("{}",launch(&load(args.get(1).context("Usage: art preset launch NAME")?)?,workspace)?),
        "show"=>println!("{}",serde_json::to_string_pretty(&load(args.get(1).context("Usage: art preset show NAME")?)?)?),
        _=>bail!("Usage: art preset [list | save NAME [--workspace N] [--replace] | launch NAME [--workspace N] | show NAME]"),
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_cannot_escape_preset_directory() {
        assert!(slug("../../bad").is_err());
        assert!(slug("hi\nthere").is_err());
        assert_eq!(slug("Spotify Night").unwrap(), "spotify-night");
    }
    #[test]
    fn coordinates_scale_with_monitor_and_origin() {
        let old = Monitor {
            name: "test".into(),
            origin: [100, 0],
            size: [1000, 1000],
        };
        let new = Monitor {
            name: "test".into(),
            origin: [2000, 0],
            size: [2000, 1000],
        };
        let window = Window {
            label: "test".into(),
            class: "test".into(),
            source_pid: 1,
            position: [200, 100],
            size: [400, 300],
            floating: true,
            terminal: true,
            command: vec!["cava".into()],
            cwd: "/".into(),
            kitty_config: None,
            cava_config: None,
        };
        assert_eq!(geometry(&window, &old, &new), ([2200, 100], [800, 300]));
        assert!(!matches(
            &serde_json::json!({"class":"Console","pid":42}),
            &window,
            "art-preset-test-1"
        ));
        let mut client = serde_json::json!({"address":"0x123abc","workspace":{"id":3},"floating":true,"at":[200,100],"size":[400,300]});
        assert!(placement_code(&client, &window, 3, [200, 100], [400, 300])
            .unwrap()
            .is_empty());
        client["floating"] = false.into();
        let code = placement_code(&client, &window, 3, [200, 100], [400, 300]).unwrap();
        assert!(code.contains("action='enable'"));
        assert!(!code.contains("action='set'"));
        client["floating"] = true.into();
        client["at"] = serde_json::json!([190, 100]);
        let code = placement_code(&client, &window, 3, [200, 100], [400, 300]).unwrap();
        assert!(code.contains("window.move"));
        assert!(!code.contains("window.resize"));
        assert!(!code.contains("window.float"));
        client["address"] = "0x';bad()".into();
        assert!(placement_code(&client, &window, 3, [200, 100], [400, 300]).is_err());
    }
}
