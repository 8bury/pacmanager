use pacmanager::{backend, model::Source};

#[test]
#[ignore = "requires pacman and access to aur.archlinux.org"]
fn searches_real_aur_and_merges_installed_state() {
    let (packages, warning) = backend::search_with_warnings("visual-studio-code-bin", true)
        .expect("official package search should succeed");
    assert!(warning.is_none(), "AUR query failed: {warning:?}");
    let package = packages
        .iter()
        .find(|p| p.name == "visual-studio-code-bin" && p.source == Source::Aur)
        .expect("AUR search should contain the requested package");
    assert!(!package.version.is_empty());
    let installed = backend::installed().expect("installed query should succeed");
    if let Some(local) = installed.iter().find(|p| p.name == package.name) {
        assert_eq!(package.installed_version, local.installed_version);
        if local.icon_path.is_some() {
            assert_eq!(package.icon_path, local.icon_path);
        }
    }
}
