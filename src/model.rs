#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Official(String),
    Aur,
    Local,
}

#[derive(Clone, Debug)]
pub struct Package {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub source: Source,
    pub installed_version: Option<String>,
    pub is_app: bool,
    pub icon_path: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Operation {
    Install(Package),
    Remove(Package),
    Update(Package),
    Upgrade,
}

#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    pub pacman: bool,
    pub yay: bool,
    pub terminal: bool,
}

// Backend API: capabilities() -> Capabilities, installed() -> Result<Vec<Package>, String>,
// search(query: &str, include_aur: bool) -> Result<Vec<Package>, String>,
// updates() -> Result<Vec<Package>, String>, execute(&Operation) -> Result<(), String>.
// All blocking functions must be called by UI workers, never in the render loop.
