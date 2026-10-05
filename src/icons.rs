//! Local desktop metadata and icon lookup. Call these from workers, never while painting.
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DesktopApp {
    pub name: String,
    pub icon: Option<String>,
}

pub(crate) fn desktop_app(text: &str) -> Option<DesktopApp> {
    let mut active = false;
    let mut fields = HashMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            active = line == "[Desktop Entry]";
        } else if active
            && !line.starts_with('#')
            && let Some((key, value)) = line.split_once('=')
        {
            fields.insert(key.trim(), value.trim());
        }
    }
    if ["Hidden", "NoDisplay"]
        .iter()
        .any(|key| fields.get(key) == Some(&"true"))
    {
        return None;
    }
    let name = fields
        .get("Name[pt_BR]")
        .or_else(|| fields.get("Name[pt]"))
        .or_else(|| fields.get("Name"))?;
    if name.is_empty() {
        return None;
    }
    Some(DesktopApp {
        name: (*name).into(),
        icon: fields
            .get("Icon")
            .filter(|v| !v.is_empty())
            .map(|v| (*v).into()),
    })
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-@".contains(&b))
}
fn supported(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("png" | "svg")
    )
}

pub(crate) struct IconResolver {
    files: HashMap<String, String>,
    names: HashMap<String, String>,
}
impl IconResolver {
    pub(crate) fn new() -> Self {
        let mut roots = Vec::new();
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
            roots.push(
                env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join(".local/share"))
                    .join("icons"),
            );
            roots.push(home.join(".icons"));
        } else if let Some(data) = env::var_os("XDG_DATA_HOME") {
            roots.push(PathBuf::from(data).join("icons"));
        }
        roots.push(PathBuf::from("/usr/share/icons"));
        Self::from_roots(&roots, &[PathBuf::from("/usr/share/pixmaps")])
    }
    fn from_roots(roots: &[PathBuf], pixmaps: &[PathBuf]) -> Self {
        let mut directories = Vec::new();
        for root in roots {
            let mut themes: Vec<_> = fs::read_dir(root)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            themes.sort_by_key(|p| match p.file_name().and_then(|s| s.to_str()) {
                Some("hicolor") => (0, String::new()),
                Some("Papirus") => (1, String::new()),
                Some("Adwaita") => (2, String::new()),
                _ => (3, p.to_string_lossy().into_owned()),
            });
            for theme in themes {
                for size in [
                    "scalable", "128x128", "64x64", "48x48", "256x256", "96x96", "32x32", "24x24",
                    "16x16",
                ] {
                    for context in ["apps", "applications"] {
                        directories.push(theme.join(size).join(context));
                    }
                }
                // Theme manifests cover custom layouts without recursive scanning.
                if let Ok(index) = fs::read_to_string(theme.join("index.theme")) {
                    let mut declared = Vec::new();
                    let mut section = "";
                    let mut contexts = HashMap::new();
                    for line in index.lines().map(str::trim) {
                        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']'))
                        {
                            section = name;
                        } else if let Some(value) = line.strip_prefix("Context=") {
                            contexts.insert(section, value);
                        } else if let Some(value) = line
                            .strip_prefix("Directories=")
                            .or_else(|| line.strip_prefix("ScaledDirectories="))
                        {
                            declared.extend(value.split(',').map(str::trim));
                        }
                    }
                    for directory in declared {
                        let path = Path::new(directory);
                        if contexts
                            .get(directory)
                            .is_some_and(|context| *context != "Applications")
                        {
                            continue;
                        }
                        if !path.is_absolute()
                            && path
                                .components()
                                .all(|c| matches!(c, std::path::Component::Normal(_)))
                        {
                            directories.push(theme.join(path));
                        }
                    }
                }
            }
            directories.push(root.clone());
        }
        directories.extend_from_slice(pixmaps);
        let mut seen = HashSet::new();
        let mut files = HashMap::new();
        let mut names = HashMap::new();
        for directory in directories {
            if !seen.insert(directory.clone()) {
                continue;
            }
            let mut entries: Vec<_> = fs::read_dir(directory)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| supported(path) && path.is_file())
                .collect();
            // Within each directory SVG wins; earlier themes and sizes keep precedence.
            entries.sort_by_key(|path| {
                (
                    path.extension().and_then(|s| s.to_str()) != Some("svg"),
                    path.clone(),
                )
            });
            for path in entries {
                let Some(filename) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let value = path.to_string_lossy().into_owned();
                files
                    .entry(filename.to_owned())
                    .or_insert_with(|| value.clone());
                names.entry(stem.to_owned()).or_insert(value);
            }
        }
        Self { files, names }
    }
    pub(crate) fn resolve(&self, name: &str) -> Option<String> {
        let path = Path::new(name);
        if path.is_absolute() {
            return (supported(path) && path.is_file()).then(|| name.into());
        }
        if !safe_name(name) {
            return None;
        }
        if supported(path) {
            self.files.get(name).cloned()
        } else {
            self.names.get(name).cloned()
        }
    }
    pub(crate) fn package(&self, name: &str) -> Option<String> {
        let aliases: &[&str] = match name {
            "visual-studio-code-bin" | "code" => {
                &["visual-studio-code", "code", "com.visualstudio.code"]
            }
            "brave-bin" => &["brave-browser", "brave"],
            "telegram-desktop" => &["telegram", "org.telegram.desktop"],
            "obs-studio" => &["com.obsproject.Studio", "obs"],
            "libreoffice-fresh" | "libreoffice-still" => {
                &["libreoffice-startcenter", "libreoffice"]
            }
            "signal-desktop" => &["signal"],
            _ => &[],
        };
        self.resolve(name)
            .or_else(|| aliases.iter().find_map(|alias| self.resolve(alias)))
            .or_else(|| name.strip_suffix("-bin").and_then(|n| self.resolve(n)))
    }
}
/// Find an installed theme icon for a package, including common desktop aliases.
pub fn resolve_app_icon(package_name: &str) -> Option<String> {
    IconResolver::new().package(package_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_sections_and_visibility() {
        assert_eq!(
            desktop_app(
                "[Other]\nIcon=wrong\n[Desktop Entry]\nName=Code\nIcon=code\n[Desktop Action Open]\nName=Wrong\nIcon=wrong"
            ),
            Some(DesktopApp {
                name: "Code".into(),
                icon: Some("code".into())
            })
        );
        for hidden in ["Hidden", "NoDisplay"] {
            assert!(
                desktop_app(&format!(
                    "[Desktop Entry]\nName=Code\n{hidden}=true\nIcon=code"
                ))
                .is_none()
            );
        }
    }
    #[test]
    fn custom_theme_and_alias() {
        let root = env::temp_dir().join(format!(
            "pacmanager-icons-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let apps = root.join("Custom/unusual-launchers");
        fs::create_dir_all(&apps).unwrap();
        fs::write(root.join("Custom/index.theme"), "[Icon Theme]\nDirectories=unusual-launchers,unrelated\n[unusual-launchers]\nContext=Applications\n[unrelated]\nContext=Actions\n").unwrap();
        let actions = root.join("Custom/unrelated");
        fs::create_dir_all(&actions).unwrap();
        fs::write(actions.join("irrelevant.svg"), "<svg/>").unwrap();
        fs::write(apps.join("com.visualstudio.code.png"), "fixture").unwrap();
        let icon = apps.join("com.visualstudio.code.svg");
        fs::write(&icon, "<svg/>").unwrap();
        let resolver = IconResolver::from_roots(std::slice::from_ref(&root), &[]);
        assert_eq!(
            resolver.package("visual-studio-code-bin"),
            Some(icon.to_string_lossy().into_owned())
        );
        assert_eq!(
            resolver.resolve(icon.to_str().unwrap()),
            Some(icon.to_string_lossy().into_owned())
        );
        assert!(resolver.resolve("../code").is_none());
        assert!(resolver.resolve("unknown").is_none());
        assert!(resolver.resolve("irrelevant").is_none());
        // Named lookups use the snapshot and do not inspect the filesystem again.
        fs::remove_file(&icon).unwrap();
        assert_eq!(
            resolver.package("visual-studio-code-bin"),
            Some(icon.to_string_lossy().into_owned())
        );
        assert!(resolver.resolve(icon.to_str().unwrap()).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
