//! Search relevance uses local metadata only; it never fetches network data.
use crate::{
    catalog::Catalog,
    model::{Package, Source},
};
use std::cmp::Ordering;

fn name_match(text: &str, query: &str) -> u8 {
    if text == query {
        return 4;
    }
    if text.match_indices(query).any(|(start, matched)| {
        let before = text[..start].chars().next_back();
        let after = text[start + matched.len()..].chars().next();
        before.is_none_or(|c| !c.is_alphanumeric()) && after.is_none_or(|c| !c.is_alphanumeric())
    }) {
        3
    } else if text.contains(query) {
        2
    } else {
        0
    }
}

fn relevance(package: &Package, query: &str, catalog: Option<&Catalog>) -> (u8, bool, f64) {
    let name = package.name.to_lowercase();
    let display = package.display_name.to_lowercase();
    let name_relevance = if name == query {
        5 // The exact package name always wins, including plugins and variants.
    } else {
        name_match(&name, query).max(name_match(&display, query))
    };
    let relevance =
        name_relevance.max(u8::from(package.description.to_lowercase().contains(query)));
    let app = package.is_app || catalog.is_some_and(|c| c.metadata.contains_key(&package.name));
    let variant = ["-git", "-bin", "-svn", "-hg", "-nightly", "-beta", "-debug"]
        .iter()
        .any(|suffix| name.ends_with(suffix));
    let popularity = catalog
        .and_then(|c| {
            c.package_popularity
                .get(&package.name)
                .copied()
                .or_else(|| c.metadata.get(&package.name).and_then(|m| m.popularity))
        })
        .map(valid_popularity)
        .unwrap_or_default();
    (relevance, app && !variant, popularity)
}

fn valid_popularity(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn source_order(source: &Source) -> u8 {
    match source {
        Source::Official(_) => 2,
        Source::Local => 1,
        Source::Aur => 0,
    }
}

/// Exact names, name matches, main applications, then installation popularity.
/// AUR votes/popularity break ties within AUR, separately from pkgstats percentages.
pub fn rank_results(packages: &mut [Package], query: &str, catalog: Option<&Catalog>) {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return;
    }
    packages.sort_by(|a, b| {
        let (ar, aa, ap) = relevance(a, &query, catalog);
        let (br, ba, bp) = relevance(b, &query, catalog);
        br.cmp(&ar)
            .then_with(|| ba.cmp(&aa))
            .then_with(|| bp.total_cmp(&ap))
            .then_with(|| source_order(&b.source).cmp(&source_order(&a.source)))
            .then_with(|| {
                if a.source != Source::Aur || b.source != Source::Aur {
                    return Ordering::Equal;
                }
                let ap = a.aur_popularity.as_ref();
                let bp = b.aur_popularity.as_ref();
                valid_popularity(bp.map_or(0.0, |p| p.popularity))
                    .total_cmp(&valid_popularity(ap.map_or(0.0, |p| p.popularity)))
                    .then_with(|| bp.map_or(0, |p| p.votes).cmp(&ap.map_or(0, |p| p.votes)))
            })
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| match (&a.source, &b.source) {
                (Source::Official(a), Source::Official(b)) => a.cmp(b),
                _ => Ordering::Equal,
            })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AurPopularity;

    fn package(name: &str, app: bool) -> Package {
        Package {
            name: name.into(),
            display_name: name.into(),
            version: "1".into(),
            description: "Plugin for OBS Studio".into(),
            source: Source::Official("extra".into()),
            installed_version: None,
            is_app: app,
            icon_path: None,
            aur_popularity: None,
        }
    }

    #[test]
    fn obs_prefers_main_app_but_exact_plugin_or_variant_wins() {
        let mut packages = vec![
            package("obs-vkcapture", false),
            package("obs-studio-git", true),
            package("libsomething", false),
            package("obs-studio", true),
            package("observer", true),
        ];
        rank_results(&mut packages, " OBS ", None);
        assert_eq!(packages[0].name, "obs-studio");
        assert_eq!(packages.last().unwrap().name, "libsomething");
        for query in ["obs-vkcapture", "obs-studio-git"] {
            rank_results(&mut packages, query, None);
            assert_eq!(packages[0].name, query);
        }
    }

    #[test]
    fn catalog_identifies_apps_and_popularity_breaks_similar_matches() {
        let mut catalog = Catalog::default();
        catalog
            .metadata
            .insert("obs-studio".into(), Default::default());
        catalog.package_popularity.insert("obs-plugin".into(), 90.0);
        catalog.package_popularity.insert("obs-studio".into(), 10.0);
        let mut packages = vec![package("obs-plugin", false), package("obs-studio", false)];
        rank_results(&mut packages, "obs", Some(&catalog));
        assert_eq!(packages[0].name, "obs-studio");
        packages.push(package("obs-recorder", true));
        catalog
            .package_popularity
            .insert("obs-recorder".into(), 20.0);
        rank_results(&mut packages, "obs", Some(&catalog));
        assert_eq!(packages[0].name, "obs-recorder");
        rank_results(&mut packages, "obs-plugin", Some(&catalog));
        assert_eq!(packages[0].name, "obs-plugin");
    }

    #[test]
    fn display_name_match_beats_description_and_installation_popularity() {
        let mut app = package("recorder", true);
        app.display_name = "OBS Studio".into();
        let mut catalog = Catalog::default();
        catalog.package_popularity.insert("other".into(), 100.0);
        let mut packages = vec![package("other", true), app];
        rank_results(&mut packages, "obs studio", Some(&catalog));
        assert_eq!(packages[0].name, "recorder");
    }

    #[test]
    fn aur_signals_are_separate_and_missing_data_has_stable_fallback() {
        let aur = |name: &str, popularity, votes| {
            let mut p = package(name, false);
            p.source = Source::Aur;
            p.aur_popularity = Some(AurPopularity { popularity, votes });
            p
        };
        let mut packages = vec![
            aur("obs-a", 1.0, 5),
            aur("obs-b", 2.0, 1),
            aur("obs-c", 2.0, 10),
            package("obs-official", false),
            aur("obs-z", f64::NAN, 0),
            aur("obs-d", 0.0, 0),
        ];
        rank_results(&mut packages, "obs", None);
        assert_eq!(
            packages.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["obs-official", "obs-c", "obs-b", "obs-a", "obs-d", "obs-z"]
        );
        packages.reverse();
        rank_results(&mut packages, "obs", None);
        assert_eq!(packages[0].name, "obs-official");
        assert_eq!(packages.last().unwrap().name, "obs-z");
    }
}
