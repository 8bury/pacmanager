//! Desktop applications from Arch AppStream, ordered by public pkgstats counts.
use crate::{backend, model::Package};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

#[derive(Default)]
pub struct Catalog {
    pub packages: Vec<Package>,
    pub metadata: HashMap<String, AppMetadata>,
    pub warnings: Vec<String>,
    pub popularity_available: bool,
}
#[derive(Clone, Debug, Default)]
pub struct AppMetadata {
    pub categories: Vec<String>,
    pub popularity: Option<f64>,
}
pub struct Category {
    pub id: &'static str,
    pub title: &'static str,
}
pub const CATEGORY_DEFINITIONS: &[Category] = &[
    Category {
        id: "Graphics",
        title: "Criatividade",
    },
    Category {
        id: "Office",
        title: "Produtividade",
    },
    Category {
        id: "Development",
        title: "Desenvolvimento",
    },
    Category {
        id: "AudioVideo",
        title: "Multimídia",
    },
    Category {
        id: "Network",
        title: "Internet",
    },
    Category {
        id: "Game",
        title: "Jogos",
    },
    Category {
        id: "Education",
        title: "Educação",
    },
    Category {
        id: "Science",
        title: "Ciência",
    },
    Category {
        id: "System",
        title: "Sistema",
    },
    Category {
        id: "Utility",
        title: "Utilitários",
    },
];
const SOURCE: &str = "https://sources.archlinux.org/other/packages/archlinux-appstream-data/";
const PKGSTATS: &str = "https://pkgstats.archlinux.de/api/packages?limit=10000";
const LIMIT: usize = 128 * 1024 * 1024;
const FRESH: Duration = Duration::from_secs(86400);
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Component {
    package: String,
    name: String,
    summary: String,
    categories: Vec<String>,
}
fn valid_package(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"@._+-".contains(&c))
}
fn localized(node: roxmltree::Node<'_, '_>, tag: &str) -> String {
    node.children()
        .filter(|n| n.has_tag_name(tag))
        .filter_map(|n| {
            let rank = match n.attribute(("http://www.w3.org/XML/1998/namespace", "lang")) {
                Some("pt_BR") => 0,
                Some("pt") => 1,
                Some("en") => 2,
                None => 3,
                _ => return None,
            };
            Some((rank, n.text().unwrap_or_default().trim()))
        })
        .filter(|(_, text)| !text.is_empty())
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, text)| text.to_owned())
        .unwrap_or_default()
}
fn parse_xml(bytes: &[u8]) -> Result<Vec<Component>, String> {
    let xml = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let mut components: HashMap<String, Component> = HashMap::new();
    for node in doc.descendants().filter(|n| {
        n.has_tag_name("component") && n.attribute("type") == Some("desktop-application")
    }) {
        let categories: Vec<String> = node
            .children()
            .filter(|n| n.has_tag_name("categories"))
            .flat_map(|n| n.children())
            .filter(|n| n.has_tag_name("category"))
            .filter_map(|n| n.text())
            .map(str::to_owned)
            .collect();
        for package in node
            .children()
            .filter(|n| n.has_tag_name("pkgname"))
            .filter_map(|n| n.text())
        {
            let package = package.trim();
            if !valid_package(package) {
                continue;
            }
            let component = Component {
                package: package.into(),
                name: localized(node, "name"),
                summary: localized(node, "summary"),
                categories: categories.clone(),
            };
            components
                .entry(package.into())
                .and_modify(|old| {
                    for category in &component.categories {
                        if !old.categories.contains(category) {
                            old.categories.push(category.clone());
                        }
                    }
                })
                .or_insert(component);
        }
    }
    let mut result: Vec<_> = components.into_values().collect();
    result.sort_by(|a, b| a.package.cmp(&b.package));
    Ok(result)
}
fn read_xml(path: &Path) -> Result<Vec<Component>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let reader: Box<dyn Read> = if path.extension().is_some_and(|e| e == "gz") {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    let mut bytes = Vec::new();
    reader
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("AppStream excedeu o limite de tamanho".into());
    }
    parse_xml(&bytes)
}
fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
        })
        .join("pacmanager")
}
fn is_fresh(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < FRESH)
}
fn download(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .new_agent();
    let mut response = agent.get(url).call().map_err(|e| e.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(limit as u64)
        .read_to_vec()
        .map_err(|e| e.to_string())
}
fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Cache inválido")?).map_err(|e| e.to_string())?;
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
fn cached<T: serde::de::DeserializeOwned + serde::Serialize>(
    path: &Path,
    label: &str,
    warnings: &mut Vec<String>,
    fetch: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let old = fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<T>(&b).ok());
    if is_fresh(path)
        && let Some(old) = old
    {
        return Ok(old);
    }
    match fetch() {
        Ok(value) => {
            match serde_json::to_vec(&value)
                .map_err(|e| e.to_string())
                .and_then(|b| write_cache(path, &b))
            {
                Ok(()) => {}
                Err(e) => warnings.push(format!("Não foi possível guardar {label}: {e}")),
            }
            Ok(value)
        }
        Err(error) => match old {
            Some(value) => {
                warnings.push(format!(
                    "{label}: usando cache antigo, atualização indisponível ({error})"
                ));
                Ok(value)
            }
            None => Err(format!("{label} indisponível: {error}")),
        },
    }
}
fn remote_components() -> Result<Vec<Component>, String> {
    let index = String::from_utf8(download(SOURCE, 1024 * 1024)?).map_err(|e| e.to_string())?;
    let date = index
        .split("href=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .filter_map(|s| s.strip_suffix('/'))
        .filter(|s| s.len() == 8 && s.bytes().all(|c| c.is_ascii_digit()))
        .max()
        .ok_or("Versão de AppStream não encontrada")?;
    let mut all = Vec::new();
    for repo in ["core", "extra", "multilib"] {
        let url = format!("{SOURCE}{date}/{repo}/Components-x86_64.xml.gz");
        let bytes = download(&url, 24 * 1024 * 1024)?;
        let mut xml = Vec::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut xml)
            .map_err(|e| e.to_string())?;
        if xml.len() > LIMIT {
            return Err("AppStream excedeu o limite de tamanho".into());
        }
        all.extend(parse_xml(&xml)?);
    }
    Ok(all)
}
fn local_components(warnings: &mut Vec<String>) -> Vec<Component> {
    let mut all = Vec::new();
    for root in [
        "/usr/share/swcatalog/xml",
        "/usr/share/app-info/xmls",
        "/var/cache/swcatalog/xml",
        "/var/cache/app-info/xmls",
    ] {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.file_name().is_some_and(|s| {
                s.to_string_lossy().ends_with(".xml") || s.to_string_lossy().ends_with(".xml.gz")
            }) {
                continue;
            }
            match read_xml(&path) {
                Ok(c) => all.extend(c),
                Err(e) => warnings.push(format!("AppStream {}: {e}", path.display())),
            }
        }
    }
    all
}
fn parse_popularity(bytes: &[u8]) -> Result<HashMap<String, f64>, String> {
    let json: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let entries = json
        .get("packagePopularities")
        .and_then(|v| v.as_array())
        .ok_or("Resposta pkgstats inválida")?;
    let map: HashMap<_, _> = entries
        .iter()
        .filter_map(|v| {
            Some((
                v.get("name")?.as_str()?.to_owned(),
                v.get("popularity")?.as_f64()?,
            ))
        })
        .filter(|(name, p)| valid_package(name) && p.is_finite() && (0.0..=100.0).contains(p))
        .collect();
    if map.is_empty() {
        return Err("pkgstats sem dados válidos".into());
    }
    Ok(map)
}
pub fn load() -> Result<Catalog, String> {
    let mut warnings = Vec::new();
    let mut components = local_components(&mut warnings);
    if components.is_empty() {
        components = cached(
            &cache_dir().join("appstream-v1.json"),
            "AppStream",
            &mut warnings,
            remote_components,
        )?;
    }
    let popularity = match cached(
        &cache_dir().join("pkgstats-v1.json"),
        "Popularidade",
        &mut warnings,
        || parse_popularity(&download(PKGSTATS, 8 * 1024 * 1024)?),
    ) {
        Ok(map) => map,
        Err(e) => {
            warnings.push(e);
            HashMap::new()
        }
    };
    let records: HashMap<_, _> = components
        .into_iter()
        .map(|c| (c.package.clone(), c))
        .collect();
    let mut seen = std::collections::HashSet::new();
    // Query only AppStream names so the backend resolves icons for applications,
    // rather than walking icon directories for every repository package.
    let mut names: Vec<_> = records.keys().map(String::as_str).collect();
    names.sort_unstable();
    let mut packages = Vec::new();
    let mut batch = Vec::new();
    let mut length = 4;
    for name in names {
        let escaped: String = name
            .chars()
            .flat_map(|c| {
                if ".+".contains(c) {
                    vec!['\\', c]
                } else {
                    vec![c]
                }
            })
            .collect();
        if length + escaped.len() + 1 > 32 * 1024 && !batch.is_empty() {
            packages.extend(backend::search(&format!("^({})$", batch.join("|")), false)?);
            batch.clear();
            length = 4;
        }
        length += escaped.len() + 1;
        batch.push(escaped);
    }
    if !batch.is_empty() {
        packages.extend(backend::search(&format!("^({})$", batch.join("|")), false)?);
    }
    let mut metadata = HashMap::new();
    packages.retain_mut(|p| {
        let Some(c) = records.get(&p.name) else {
            return false;
        };
        if !seen.insert(p.name.clone()) {
            return false;
        }
        p.is_app = true;
        if !c.name.is_empty() {
            p.display_name = c.name.clone();
        }
        if !c.summary.is_empty() {
            p.description = c.summary.clone();
        }
        metadata.insert(
            p.name.clone(),
            AppMetadata {
                categories: c.categories.clone(),
                popularity: popularity.get(&p.name).copied(),
            },
        );
        true
    });
    packages.sort_by(|a, b| {
        let ap = metadata[&a.name].popularity;
        let bp = metadata[&b.name].popularity;
        bp.partial_cmp(&ap)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            })
            .then_with(|| a.name.cmp(&b.name))
    });
    if packages.is_empty() {
        warnings
            .push("Nenhum aplicativo AppStream corresponde aos repositórios configurados.".into());
    }
    let popularity_available = metadata.values().any(|m| m.popularity.is_some());
    Ok(Catalog {
        packages,
        metadata,
        warnings,
        popularity_available,
    })
}
pub fn demo(packages: Vec<Package>) -> Catalog {
    let groups = [
        ("Graphics", &["gimp", "inkscape", "blender", "krita"][..]),
        (
            "Office",
            &["libreoffice-fresh", "okular", "keepassxc", "ghostwriter"][..],
        ),
        ("Development", &["code", "kate", "geany", "kitty"][..]),
        ("AudioVideo", &["vlc", "mpv", "audacity", "kdenlive"][..]),
        (
            "Network",
            &["firefox", "chromium", "thunderbird", "telegram-desktop"][..],
        ),
        (
            "Game",
            &["steam", "lutris", "retroarch", "supertuxkart"][..],
        ),
    ];
    let metadata = packages
        .iter()
        .map(|p| {
            (
                p.name.clone(),
                AppMetadata {
                    categories: groups
                        .iter()
                        .filter(|(_, names)| names.contains(&p.name.as_str()))
                        .map(|(id, _)| id.to_string())
                        .collect(),
                    popularity: None,
                },
            )
        })
        .collect();
    Catalog {
        packages,
        metadata,
        warnings: vec![],
        popularity_available: false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xml_filters_localizes_merges_and_validates() {
        let xml = br#"<components><component type="desktop-application"><pkgname>paint</pkgname><name>Paint</name><name xml:lang="pt">Pintura</name><name xml:lang="pt_BR">Desenho</name><summary>Draw</summary><categories><category>Graphics</category></categories></component><component type="desktop-application"><pkgname>paint</pkgname><categories><category>Utility</category></categories></component><component type="console-application"><pkgname>cli</pkgname></component><component type="desktop-application"><pkgname>--bad</pkgname></component></components>"#;
        let c = parse_xml(xml).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].name, "Desenho");
        assert_eq!(c[0].summary, "Draw");
        assert_eq!(c[0].categories, vec!["Graphics", "Utility"]);
    }
    #[test]
    fn xml_uses_english_and_rejects_malformed_documents() {
        let c = parse_xml(br#"<components><component type="desktop-application"><pkgname>app</pkgname><pkgname>app-extra</pkgname><name xml:lang="de">Deutsch</name><name xml:lang="en">English</name><summary xml:lang="pt">Resumo</summary></component></components>"#).unwrap();
        assert_eq!(c.len(), 2);
        assert!(
            c.iter()
                .all(|c| c.name == "English" && c.summary == "Resumo")
        );
        assert!(parse_xml(b"<components>").is_err());
    }
    #[test]
    fn popularity_rejects_invalid_entries() {
        let p=parse_popularity(br#"{"packagePopularities":[{"name":"paint","popularity":12.5},{"name":"bad","popularity":200}]}"#).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p["paint"], 12.5);
        assert!(parse_popularity(b"{}").is_err());
    }
    #[test]
    fn stale_cache_survives_network_failure_and_fresh_skips_fetch() {
        let p = std::env::temp_dir().join(format!("pacmanager-cache-test-{}", std::process::id()));
        write_cache(&p, b"[1,2]").unwrap();
        let mut w = vec![];
        let value: Vec<u32> = cached(&p, "test", &mut w, || {
            panic!("fresh cache should skip network")
        })
        .unwrap();
        assert_eq!(value, vec![1, 2]);
        fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(SystemTime::now() - FRESH - Duration::from_secs(1)),
            )
            .unwrap();
        let old: Vec<u32> = cached(&p, "test", &mut w, || Err("offline".into())).unwrap();
        assert_eq!(old, value);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("cache antigo"));
        fs::remove_file(p).unwrap();
    }
}
