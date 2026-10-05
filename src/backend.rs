use crate::model::{Capabilities, Operation, Package, Source};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    io::Read,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn available(name: &str) -> bool {
    env::var_os("PATH").is_some_and(|p| {
        env::split_paths(&p).any(|d| {
            let p = d.join(name);
            p.is_file() && executable(&p)
        })
    })
}
#[cfg(unix)]
fn executable(p: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(p).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}
#[cfg(not(unix))]
fn executable(_: &std::path::Path) -> bool {
    true
}
const TERMINALS: &[&str] = &["kitty", "foot", "xterm", "alacritty", "gnome-terminal"];
pub fn capabilities() -> Capabilities {
    Capabilities {
        pacman: available("pacman"),
        yay: available("yay"),
        terminal: TERMINALS.iter().any(|t| available(t)),
    }
}
fn run(program: &str, args: &[&str]) -> Result<Output, String> {
    run_bounded(program, args, Duration::from_secs(30))
}
fn run_bounded(program: &str, args: &[&str], timeout: Duration) -> Result<Output, String> {
    let deadline = Instant::now() + timeout;
    let mut child = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Não foi possível executar {program}: {e}"))?;
    let (sender, receiver) = mpsc::channel();
    let stdout = child
        .stdout
        .take()
        .ok_or("Saída de processo indisponível")?;
    let stderr = child.stderr.take().ok_or("Erro de processo indisponível")?;
    for (index, mut pipe) in [
        (0, Box::new(stdout) as Box<dyn Read + Send>),
        (1, Box::new(stderr) as Box<dyn Read + Send>),
    ] {
        let sender = sender.clone();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
            let _ = sender.send((index, result));
        });
    }
    drop(sender);
    let mut status = None;
    let mut streams: [Option<Vec<u8>>; 2] = [None, None];
    loop {
        match child.try_wait() {
            Ok(Some(exit)) => status = Some(exit),
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{program}: {error}"));
            }
        }
        while let Ok((index, result)) = receiver.try_recv() {
            match result {
                Ok(bytes) => streams[index] = Some(bytes),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{program}: {error}"));
                }
            }
        }
        if let Some(status) = status
            && streams.iter().all(Option::is_some)
        {
            return Ok(Output {
                status,
                stdout: streams[0].take().unwrap_or_default(),
                stderr: streams[1].take().unwrap_or_default(),
            });
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{program}: tempo limite de {} segundos excedido",
                timeout.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}
fn output(program: &str, args: &[&str]) -> Result<String, String> {
    let o = run(program, args)?;
    if !o.status.success() {
        return Err(format!(
            "{program}: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}
fn app_names(files: &str) -> HashMap<String, crate::icons::DesktopApp> {
    let mut apps = HashMap::new();
    for line in files.lines() {
        let Some((name, path)) = line.split_once(' ') else {
            continue;
        };
        if path.starts_with("/usr/share/applications/")
            && path.ends_with(".desktop")
            && let Ok(text) = fs::read_to_string(path)
            && let Some(app) = crate::icons::desktop_app(&text)
        {
            apps.entry(name.to_owned()).or_insert(app);
        }
    }
    apps
}
fn known_app(name: &str) -> bool {
    matches!(
        name,
        "firefox"
            | "chromium"
            | "google-chrome"
            | "brave-bin"
            | "vivaldi"
            | "vlc"
            | "mpv"
            | "gimp"
            | "inkscape"
            | "krita"
            | "blender"
            | "obs-studio"
            | "discord"
            | "telegram-desktop"
            | "signal-desktop"
            | "spotify"
            | "steam"
            | "lutris"
            | "heroic-games-launcher-bin"
            | "code"
            | "visual-studio-code-bin"
            | "geany"
            | "kate"
            | "gedit"
            | "mousepad"
            | "thunar"
            | "dolphin"
            | "nautilus"
            | "pcmanfm"
            | "konsole"
            | "kitty"
            | "alacritty"
            | "foot"
            | "wezterm"
            | "xterm"
            | "gnome-terminal"
            | "audacity"
            | "kdenlive"
            | "shotcut"
            | "openshot"
            | "handbrake"
            | "filezilla"
            | "transmission-gtk"
            | "qbittorrent"
            | "keepassxc"
            | "bitwarden"
            | "bitwarden-bin"
            | "thunderbird"
            | "evolution"
            | "libreoffice-fresh"
            | "libreoffice-still"
            | "okular"
            | "evince"
            | "zathura"
            | "pavucontrol"
            | "bleachbit"
            | "gnome-calculator"
            | "gnome-clocks"
            | "gnome-calendar"
    )
}
fn parse_info(text: &str) -> Vec<Package> {
    let mut out = Vec::new();
    let mut fields = HashMap::<String, String>::new();
    let mut last = String::new();
    for line in text.lines().chain(std::iter::once("")) {
        if line.trim().is_empty() {
            if let Some(name) = fields.get("Name") {
                out.push(Package {
                    name: name.clone(),
                    display_name: name.clone(),
                    version: fields.get("Version").cloned().unwrap_or_default(),
                    description: fields.get("Description").cloned().unwrap_or_default(),
                    source: Source::Official("installed".into()),
                    installed_version: fields.get("Version").cloned(),
                    is_app: false,
                    icon_path: None,
                });
            }
            fields.clear();
            last.clear();
        } else if let Some((key, value)) = line.split_once(" : ") {
            last = key.trim().into();
            fields.insert(last.clone(), value.trim().into());
        } else if line.starts_with(' ')
            && let Some(value) = fields.get_mut(&last)
        {
            value.push(' ');
            value.push_str(line.trim());
        }
    }
    out
}
pub fn installed() -> Result<Vec<Package>, String> {
    let mut packages = parse_info(&output("pacman", &["-Qi"])?);
    let apps = app_names(&output("pacman", &["-Ql"])?);
    let mut repositories = HashMap::<String, String>::new();
    for line in output("pacman", &["-Sl"])?.lines() {
        let mut f = line.split_whitespace();
        if let (Some(repo), Some(name)) = (f.next(), f.next()) {
            repositories
                .entry(name.into())
                .or_insert_with(|| repo.into());
        }
    }
    let foreign: HashSet<_> = update_output("pacman", &["-Qmq"])?
        .lines()
        .map(str::to_owned)
        .collect();
    let icons = crate::icons::IconResolver::new();
    for p in &mut packages {
        p.icon_path = apps
            .get(&p.name)
            .and_then(|app| app.icon.as_deref())
            .and_then(|icon| icons.resolve(icon))
            .or_else(|| icons.package(&p.name));
        p.is_app = apps.contains_key(&p.name);
        if let Some(label) = apps.get(&p.name) {
            p.display_name = label.name.clone();
        }
        if foreign.contains(&p.name) {
            p.source = Source::Local;
        } else if let Some(repo) = repositories.get(&p.name) {
            p.source = Source::Official(repo.clone());
        }
    }
    Ok(packages)
}
fn parse_search(text: &str) -> Vec<Package> {
    let mut out: Vec<Package> = Vec::new();
    for line in text.lines() {
        if line.starts_with(' ') {
            if let Some(p) = out.last_mut() {
                if !p.description.is_empty() {
                    p.description.push(' ');
                }
                p.description.push_str(line.trim());
            }
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(id) = fields.next() else { continue };
        let Some((repo, name)) = id.split_once('/') else {
            continue;
        };
        let Some(version) = fields.next() else {
            continue;
        };
        out.push(Package {
            name: name.into(),
            display_name: name.into(),
            version: version.into(),
            description: String::new(),
            source: Source::Official(repo.into()),
            installed_version: None,
            is_app: known_app(name),
            icon_path: None,
        });
    }
    out
}
fn encode(query: &str) -> String {
    query
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
#[derive(Deserialize)]
struct AurReply {
    #[serde(default)]
    results: Vec<AurPackage>,
    error: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AurPackage {
    name: String,
    version: String,
    description: Option<String>,
}
fn aur_search(query: &str) -> Result<Vec<Package>, String> {
    let url = format!(
        "https://aur.archlinux.org/rpc/v5/search/{}?by=name-desc",
        encode(query)
    );
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let response: AurReply = agent
        .get(&url)
        .call()
        .map_err(|e| format!("AUR: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("Resposta AUR inválida: {e}"))?;
    if let Some(error) = response.error {
        return Err(format!("AUR: {error}"));
    }
    Ok(response
        .results
        .into_iter()
        .map(|p| Package {
            is_app: known_app(&p.name),
            display_name: p.name.clone(),
            name: p.name,
            version: p.version,
            description: p.description.unwrap_or_default(),
            source: Source::Aur,
            installed_version: None,
            icon_path: None,
        })
        .collect())
}
pub fn search(query: &str, include_aur: bool) -> Result<Vec<Package>, String> {
    let (packages, warning) = search_with_warnings(query, include_aur)?;
    if let Some(warning) = warning {
        return Err(warning);
    }
    Ok(packages)
}
pub fn search_with_warnings(
    query: &str,
    include_aur: bool,
) -> Result<(Vec<Package>, Option<String>), String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok((Vec::new(), None));
    }
    if query.starts_with('-') || query.chars().any(char::is_control) {
        return Err("Pesquisa inválida".into());
    }
    let o = run("pacman", &["-Ss", query])?;
    if !o.status.success() && !(o.status.code() == Some(1) && o.stderr.is_empty()) {
        return Err(String::from_utf8_lossy(&o.stderr).into_owned());
    }
    let mut packages = parse_search(&String::from_utf8_lossy(&o.stdout));
    let mut warning = None;
    if include_aur {
        match aur_search(query) {
            Ok(aur) => packages.extend(aur),
            Err(error) => warning = Some(error),
        }
    }
    let local = installed()?;
    let versions: HashMap<_, _> = local.iter().map(|p| (p.name.as_str(), p)).collect();
    let mut seen = HashSet::new();
    packages.retain(|p| seen.insert((p.name.clone(), format!("{:?}", p.source))));
    let icons = crate::icons::IconResolver::new();
    for p in &mut packages {
        p.icon_path = icons.package(&p.name);
        if let Some(local) = versions.get(p.name.as_str()) {
            p.installed_version = local.installed_version.clone();
            p.is_app |= local.is_app;
            p.display_name = local.display_name.clone();
            p.icon_path = local.icon_path.clone().or_else(|| p.icon_path.clone());
        }
    }
    Ok((packages, warning))
}

fn parse_updates(text: &str, aur: bool) -> Vec<Package> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.split_whitespace();
            let name = f.next()?;
            let old = f.next()?;
            if f.next()? != "->" {
                return None;
            };
            let new = f.next()?;
            Some(Package {
                name: name.into(),
                display_name: name.into(),
                version: new.into(),
                description: String::new(),
                source: if aur {
                    Source::Aur
                } else {
                    Source::Official("official".into())
                },
                installed_version: Some(old.into()),
                is_app: known_app(name),
                icon_path: None,
            })
        })
        .collect()
}
fn update_output(program: &str, args: &[&str]) -> Result<String, String> {
    let o = run(program, args)?;
    if !o.status.success()
        && !(o.status.code() == Some(1) && o.stdout.is_empty() && o.stderr.is_empty())
    {
        return Err(format!(
            "{program}: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}
pub fn updates() -> Result<Vec<Package>, String> {
    let (packages, warning) = updates_with_warnings()?;
    if let Some(warning) = warning {
        return Err(warning);
    }
    Ok(packages)
}
pub fn updates_with_warnings() -> Result<(Vec<Package>, Option<String>), String> {
    let mut packages = parse_updates(&update_output("pacman", &["-Qu"])?, false);
    let mut warning = None;
    if available("yay") {
        match update_output("yay", &["-Qua"]) {
            Ok(text) => packages.extend(parse_updates(&text, true)),
            Err(error) => warning = Some(error),
        }
    }
    let local = installed()?;
    for p in &mut packages {
        if let Some(l) = local.iter().find(|l| l.name == p.name) {
            p.description = l.description.clone();
            p.display_name = l.display_name.clone();
            if matches!(p.source, Source::Official(_)) {
                p.source = l.source.clone();
            }
            p.is_app = l.is_app;
            p.icon_path = l.icon_path.clone();
        }
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages.dedup_by(|a, b| a.name == b.name);
    Ok((packages, warning))
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@._+-".contains(&b))
}
fn operation_command(op: &Operation) -> Result<(String, Vec<String>), String> {
    match op {
        Operation::Upgrade => {
            if available("yay") {
                Ok(("yay".into(), vec!["-Syu".into()]))
            } else {
                Ok(("sudo".into(), vec!["pacman".into(), "-Syu".into()]))
            }
        }
        Operation::Install(p) | Operation::Remove(p) | Operation::Update(p) => {
            if !valid_name(&p.name) {
                return Err("Nome de pacote inválido".into());
            }
            if matches!(op, Operation::Update(_)) && p.installed_version.is_none() {
                return Err("Somente pacotes instalados podem ser atualizados".into());
            }
            match op {
                Operation::Update(_) => match &p.source {
                    Source::Aur => Ok((
                        "yay".into(),
                        vec![
                            "-S".into(),
                            "--aur".into(),
                            "--needed".into(),
                            p.name.clone(),
                        ],
                    )),
                    Source::Official(repo)
                        if valid_name(repo) && repo != "installed" && repo != "official" =>
                    {
                        Ok((
                            "sudo".into(),
                            vec![
                                "pacman".into(),
                                "-S".into(),
                                "--needed".into(),
                                format!("{repo}/{}", p.name),
                            ],
                        ))
                    }
                    _ => Err("Pacote sem repositório de atualização conhecido".into()),
                },
                Operation::Remove(_) => Ok((
                    "sudo".into(),
                    vec!["pacman".into(), "-R".into(), p.name.clone()],
                )),
                _ if p.source == Source::Local => {
                    Err("Pacote local sem repositório de instalação conhecido".into())
                }
                _ if p.source == Source::Aur => Ok((
                    "yay".into(),
                    vec!["-S".into(), "--aur".into(), p.name.clone()],
                )),
                _ => Ok((
                    "sudo".into(),
                    vec![
                        "pacman".into(),
                        "-Syu".into(),
                        match &p.source {
                            Source::Official(repo) if valid_name(repo) => {
                                format!("{repo}/{}", p.name)
                            }
                            _ => return Err("Repositório inválido".into()),
                        },
                    ],
                )),
            }
        }
    }
}
pub fn execute(op: &Operation) -> Result<(), String> {
    let (program, args) = operation_command(op)?;
    if !available(&program) {
        return Err(format!("Instale {program} para realizar esta operação"));
    }
    if !available("pacman") {
        return Err("pacman não está disponível".into());
    }
    if env::var("USER").as_deref() == Ok("root") || output("id", &["-u"])?.trim() == "0" {
        return Err("Abra o PacManager como usuário normal para executar transações".into());
    }
    let terminal = TERMINALS
        .iter()
        .find(|t| available(t))
        .ok_or("Instale kitty, foot, xterm, alacritty ou gnome-terminal")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let status_dir = env::temp_dir().join(format!("pacmanager-{}-{stamp}", std::process::id()));
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&status_dir)
            .map_err(|e| e.to_string())?;
    }
    let status_path = status_dir.join("status");
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut command = Command::new(terminal);
    match *terminal {
        "kitty" => {}
        "gnome-terminal" => {
            command.args(["--wait", "--"]);
        }
        _ => {
            command.arg("-e");
        }
    }
    let status = command
        .arg(exe)
        .arg("--pacmanager-transaction-helper")
        .arg(&status_path)
        .arg(program)
        .args(args)
        .status();
    let result = fs::read_to_string(&status_path);
    let _ = fs::remove_file(&status_path);
    let _ = fs::remove_dir(&status_dir);
    let status = status.map_err(|e| e.to_string())?;
    match result {
        Ok(code) if code.trim() == "0" => Ok(()),
        Ok(code) => Err(format!("A operação terminou com código {}", code.trim())),
        Err(_) => Err(format!(
            "O terminal encerrou sem confirmar a operação ({status})"
        )),
    }
}
fn transaction_lock() -> Result<fs::File, String> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
    let home = PathBuf::from(env::var_os("HOME").ok_or("HOME indisponível")?);
    let owner = fs::metadata(&home).map_err(|e| e.to_string())?.uid();
    let base = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".cache"));
    if !base.exists() {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&base)
            .map_err(|e| e.to_string())?;
    }
    let metadata = fs::symlink_metadata(&base).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != owner {
        return Err("Diretório de transações inválido".into());
    }
    let directory = base.join("pacmanager");
    match fs::DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.to_string()),
    }
    let metadata = fs::symlink_metadata(&directory).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != owner || metadata.permissions().mode() & 0o077 != 0 {
        return Err("O diretório de transações deve ser privado".into());
    }
    let path = directory.join("transaction.lock");
    let file = match fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            if !metadata.is_file()
                || metadata.uid() != owner
                || metadata.permissions().mode() & 0o077 != 0
            {
                return Err("Arquivo de bloqueio inválido".into());
            }
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| e.to_string())?
        }
        Err(error) => return Err(error.to_string()),
    };
    file.try_lock()
        .map_err(|_| "Outra transação do PacManager está em andamento".to_owned())?;
    Ok(file)
}
/// Main calls this before opening the GUI. The terminal helper records the actual package-manager exit code.
pub fn transaction_helper(args: &[String]) -> bool {
    if args.first().map(String::as_str) != Some("--pacmanager-transaction-helper") {
        return false;
    }
    if args.len() < 3 {
        std::process::exit(2);
    }
    let path = PathBuf::from(&args[1]);
    let code = match transaction_lock() {
        Ok(lock) => {
            let code = Command::new(&args[2])
                .args(&args[3..])
                .status()
                .map(|s| s.code().unwrap_or(1))
                .unwrap_or(1);
            drop(lock);
            code
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    };
    if let Err(e) = (|| -> io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(code.to_string().as_bytes())
    })() {
        eprintln!("Não foi possível registrar o resultado: {e}");
        std::process::exit(1);
    }
    println!("\nOperação finalizada (código {code}). Pressione Enter para fechar.");
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
    std::process::exit(code);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn info_multiline() {
        let p = parse_info(
            "Name            : firefox\nVersion         : 1.2-1\nDescription     : Web browser\n                  continuation\n\n",
        );
        assert_eq!(p[0].name, "firefox");
        assert_eq!(p[0].description, "Web browser continuation");
    }
    #[test]
    fn search_fixture() {
        let p = parse_search(
            "extra/firefox 1.2-1 [installed]\n    Web browser\n    Continued\ncore/bash 5.3\n    Shell\n",
        );
        assert_eq!(p.len(), 2);
        assert!(p[0].is_app);
        assert!(!p[1].is_app);
        assert_eq!(p[0].description, "Web browser Continued");
    }
    #[test]
    fn updates_fixture() {
        let p = parse_updates("firefox 1.0 -> 2.0\nwarning: ignored\n", true);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].installed_version.as_deref(), Some("1.0"));
        assert_eq!(p[0].source, Source::Aur);
    }
    #[test]
    fn safe_names_and_encoding() {
        for n in ["--help", "a b", "$(id)", "a;id", ""] {
            assert!(!valid_name(n));
        }
        assert!(valid_name("libc++"));
        assert_eq!(encode("a/b &c"), "a%2Fb%20%26c");
    }
    #[test]
    fn installed_desktop_fixture() {
        assert_eq!(
            crate::icons::desktop_app(
                "[Desktop Entry]\nName=Firefox\nName[pt_BR]=Navegador Firefox\n[Desktop Action Test]\nName=Test"
            ),
            Some(crate::icons::DesktopApp {
                name: "Navegador Firefox".into(),
                icon: None
            })
        );
        assert_eq!(
            crate::icons::desktop_app("[Desktop Entry]\nName=Hidden\nNoDisplay=true"),
            None
        );
    }
    #[test]
    fn transaction_recipes_preserve_prompts() {
        let mut p = Package {
            name: "firefox".into(),
            display_name: "Firefox".into(),
            version: "1".into(),
            description: String::new(),
            source: Source::Official("extra".into()),
            installed_version: None,
            is_app: true,
            icon_path: None,
        };
        assert_eq!(
            operation_command(&Operation::Install(p.clone())).unwrap(),
            (
                "sudo".into(),
                vec!["pacman".into(), "-Syu".into(), "extra/firefox".into()]
            )
        );
        assert_eq!(
            operation_command(&Operation::Remove(p.clone())).unwrap(),
            (
                "sudo".into(),
                vec!["pacman".into(), "-R".into(), "firefox".into()]
            )
        );
        p.source = Source::Aur;
        assert_eq!(
            operation_command(&Operation::Install(p.clone())).unwrap(),
            (
                "yay".into(),
                vec!["-S".into(), "--aur".into(), "firefox".into()]
            )
        );
        p.name = "--noconfirm".into();
        assert!(operation_command(&Operation::Install(p)).is_err());
    }
    #[test]
    fn selected_updates_preserve_prompts_and_repository_databases() {
        let mut package = parse_updates("firefox 1 -> 2", false).remove(0);
        package.source = Source::Official("extra".into());
        assert_eq!(
            operation_command(&Operation::Update(package.clone())).unwrap(),
            (
                "sudo".into(),
                vec![
                    "pacman".into(),
                    "-S".into(),
                    "--needed".into(),
                    "extra/firefox".into()
                ]
            )
        );
        package.source = Source::Aur;
        assert_eq!(
            operation_command(&Operation::Update(package.clone())).unwrap(),
            (
                "yay".into(),
                vec![
                    "-S".into(),
                    "--aur".into(),
                    "--needed".into(),
                    "firefox".into()
                ]
            )
        );
        for source in [
            Source::Local,
            Source::Official("installed".into()),
            Source::Official("official".into()),
            Source::Official("extra;id".into()),
            Source::Official("--help".into()),
        ] {
            package.source = source;
            assert!(operation_command(&Operation::Update(package.clone())).is_err());
        }
        package.source = Source::Official("extra".into());
        package.installed_version = None;
        assert!(operation_command(&Operation::Update(package.clone())).is_err());
        package.installed_version = Some("1".into());
        for name in ["--noconfirm", "a/b", "a;id", "$(id)"] {
            package.name = name.into();
            assert!(operation_command(&Operation::Update(package.clone())).is_err());
        }
    }
    #[test]
    fn read_only_process_timeout() {
        let started = Instant::now();
        assert!(run_bounded("/usr/bin/sleep", &["2"], Duration::from_millis(50)).is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn drains_both_large_pipes_without_deadlock() {
        // Test fixture only; production package commands never execute through a shell.
        let output = run_bounded(
            "/bin/sh",
            &[
                "-c",
                "head -c 262144 /dev/zero; head -c 262144 /dev/zero >&2",
            ],
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 262144);
        assert_eq!(output.stderr.len(), 262144);
    }
    #[test]
    fn file_lock_rejects_concurrent_transaction() {
        let path = env::temp_dir().join(format!(
            "pacmanager-lock-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let first = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let second = fs::OpenOptions::new().write(true).open(&path).unwrap();
        first.try_lock().unwrap();
        assert!(second.try_lock().is_err());
        drop(first);
        second.try_lock().unwrap();
        drop(second);
        fs::remove_file(path).unwrap();
    }
}
