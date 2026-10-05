mod app;
mod design;
use pacmanager::backend;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if backend::transaction_helper(&args) {
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "PacManager\n\nUso: pacmanager [--theme light|dark|system] [--demo] [--demo-app NOME] [--demo-tab ABA] [--check] [--help]\n  --theme Escolhe o tema nesta execução.\n  --check Verifica o backend sem alterar pacotes.\n  --demo  Abre uma prévia com dados fictícios, sem consultar ou alterar o sistema."
        );
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--check") && !args.iter().any(|arg| arg == "--demo") {
        let capabilities = backend::capabilities();
        println!(
            "pacman={} yay={} terminal={}",
            capabilities.pacman, capabilities.yay, capabilities.terminal
        );
        let mut failed = false;
        match pacmanager::catalog::load() {
            Ok(catalog) => {
                println!("catálogo: {} aplicativos", catalog.packages.len());
                println!("popularidade disponível: {}", catalog.popularity_available);
                for warning in catalog.warnings {
                    eprintln!("catálogo: {warning}");
                }
            }
            Err(error) => {
                eprintln!("catálogo: {error}");
                failed = true;
            }
        }
        for (name, result) in [
            ("instalados", backend::installed()),
            ("busca firefox", backend::search("firefox", false)),
            ("atualizações", backend::updates()),
        ] {
            match result {
                Ok(packages) => println!("{name}: {} pacotes", packages.len()),
                Err(error) => {
                    eprintln!("{name}: {error}");
                    failed = true;
                }
            }
        }
        if failed {
            std::process::exit(1);
        }
        return Ok(());
    }
    let demo = args.iter().any(|arg| arg == "--demo");
    let theme = args.iter().position(|arg| arg == "--theme").map(|index| {
        match args.get(index + 1).map(String::as_str) {
            Some("light") => eframe::egui::ThemePreference::Light,
            Some("dark") => eframe::egui::ThemePreference::Dark,
            Some("system") => eframe::egui::ThemePreference::System,
            _ => {
                eprintln!("--theme espera light, dark ou system");
                std::process::exit(2);
            }
        }
    });
    let demo_app = args
        .iter()
        .position(|arg| arg == "--demo-app")
        .and_then(|index| args.get(index + 1))
        .cloned();
    let demo_tab = args
        .iter()
        .position(|arg| arg == "--demo-tab")
        .and_then(|index| args.get(index + 1))
        .cloned();
    if demo_tab
        .as_deref()
        .is_some_and(|tab| !["explore", "installed", "updates", "categories"].contains(&tab))
    {
        eprintln!("--demo-tab espera explore, installed, updates ou categories");
        std::process::exit(2);
    }
    if (demo_app.is_some() || demo_tab.is_some()) && !demo {
        eprintln!("--demo-app e --demo-tab requerem --demo");
        std::process::exit(2);
    }
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("PacManager")
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([800.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PacManager",
        options,
        Box::new(move |cc| {
            let mut store = app::Store::new(cc, demo);
            if let Some(theme) = theme {
                let _ = design::set_theme(&cc.egui_ctx, theme, false);
            }
            if let Some(tab) = &demo_tab {
                store.open_demo_tab(tab);
            }
            if let Some(name) = &demo_app {
                store.open_demo_package(name);
            }
            Ok(Box::new(store))
        }),
    )
}
