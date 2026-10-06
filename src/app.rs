use crate::design;
use eframe::egui::{self, Color32, RichText};
use pacmanager::catalog;
use pacmanager::{
    backend,
    model::{Capabilities, Operation, Package, Source},
};
use std::{
    collections::{HashMap, HashSet},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Explore,
    Installed,
    Updates,
    Categories,
}
enum Message {
    Capabilities(Capabilities),
    Catalog(Result<catalog::Catalog, String>),
    Installed(Result<Vec<Package>, String>),
    Updates(Result<(Vec<Package>, Option<String>), String>),
    Search(u64, Result<(Vec<Package>, Option<String>), String>),
    Operation(Result<(), String>),
}
pub struct Store {
    demo: bool,
    tab: Tab,
    query: String,
    aur: bool,
    all: bool,
    installed: Vec<Package>,
    installed_versions: HashMap<String, String>,
    updates: Vec<Package>,
    update_by_name: HashMap<String, Package>,
    demo_packages: Vec<Package>,
    catalog: catalog::Catalog,
    catalog_loading: bool,
    catalog_error: Option<String>,
    collection: Option<String>,
    results: Vec<Package>,
    expanded_packages: HashSet<String>,
    capabilities: Option<Capabilities>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    search_id: u64,
    searching: bool,
    search_due: Option<Instant>,
    installed_loading: bool,
    updates_loading: bool,
    running: bool,
    status: String,
    error: Option<String>,
    selected: Option<Package>,
    confirm: Option<Operation>,
}
impl Store {
    pub fn new(cc: &eframe::CreationContext<'_>, demo: bool) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        design::install(&cc.egui_ctx);
        let mut app = Self::state(demo);
        if demo {
            app.capabilities = Some(Capabilities {
                pacman: true,
                yay: true,
                terminal: true,
            });
            app.catalog = catalog::demo(
                app.demo_packages
                    .iter()
                    .filter(|p| p.source != Source::Aur && p.is_app)
                    .cloned()
                    .collect(),
            );
            app.results = app
                .demo_packages
                .clone()
                .into_iter()
                .filter(|p| p.source != Source::Aur)
                .collect();
            app.installed = app
                .demo_packages
                .clone()
                .into_iter()
                .filter(|p| p.installed_version.is_some())
                .collect();
            app.installed_versions = app
                .installed
                .iter()
                .filter_map(|p| p.installed_version.clone().map(|v| (p.name.clone(), v)))
                .collect();
            app.updates = app.installed.iter().filter(|p| p.is_app).cloned().collect();
            app.reindex_updates();
        } else {
            app.worker(&cc.egui_ctx, || {
                Message::Capabilities(backend::capabilities())
            });
            app.catalog_loading = true;
            app.worker(&cc.egui_ctx, || Message::Catalog(catalog::load()));
            app.refresh(&cc.egui_ctx);
        }
        app
    }
    fn state(demo: bool) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            demo,
            tab: Tab::Explore,
            query: String::new(),
            aur: false,
            all: false,
            installed: vec![],
            installed_versions: HashMap::new(),
            updates: vec![],
            update_by_name: HashMap::new(),
            demo_packages: if demo { fixtures() } else { vec![] },
            catalog: catalog::Catalog::default(),
            catalog_loading: false,
            catalog_error: None,
            collection: None,
            results: vec![],
            expanded_packages: HashSet::new(),
            capabilities: None,
            sender,
            receiver,
            search_id: 0,
            searching: false,
            search_due: None,
            installed_loading: false,
            updates_loading: false,
            running: false,
            status: String::new(),
            error: None,
            selected: None,
            confirm: None,
        }
    }
    pub fn open_demo_package(&mut self, name: &str) {
        if self.demo
            && let Some(package) = self.demo_packages.iter().find(|p| p.name == name).cloned()
        {
            self.open_package(package);
        }
    }
    fn close_package(&mut self) {
        self.selected = None;
    }
    fn open_package(&mut self, mut package: Package) {
        package.installed_version = self.installed_versions.get(&package.name).cloned();
        self.selected = Some(package);
    }
    fn reindex_updates(&mut self) {
        self.update_by_name = self
            .updates
            .iter()
            .map(|p| (update_key(p), p.clone()))
            .collect();
    }
    fn available_update(&self, package: &Package) -> Option<&Package> {
        self.update_by_name.get(&update_key(package)).or_else(|| {
            if package.source == Source::Local {
                self.update_by_name.get(&format!("{}:aur", package.name))
            } else {
                None
            }
        })
    }
    pub fn open_demo_tab(&mut self, tab: &str) {
        if self.demo {
            self.tab = match tab {
                "installed" => Tab::Installed,
                "updates" => Tab::Updates,
                "categories" => Tab::Categories,
                _ => Tab::Explore,
            };
        }
    }
    fn package_page(&mut self, ui: &mut egui::Ui, package: &Package) {
        if design::line_icon(ui, design::NavIcon::Back, 28.0)
            .on_hover_text("Voltar")
            .clicked()
        {
            self.close_package();
            return;
        }
        ui.add_space(28.0);
        if let Some(error) = &self.error {
            ui.colored_label(design::palette(ui.ctx()).error, error);
        }
        let update = self.available_update(package).cloned();
        egui::ScrollArea::vertical().id_salt("package_page").show(ui, |ui| {
            ui.horizontal(|ui| {
                design::app_icon_static(ui, package, 112.0);
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.label(design::heading(display_name(package), 30.0));
                    ui.label(RichText::new(tagline(package)).color(design::palette(ui.ctx()).secondary));
                    ui.add_space(15.0);
                    ui.horizontal(|ui| {
                        let op = if let Some(p) = &update { Some(Operation::Update(p.clone())) } else if package.installed_version.is_none() { Some(Operation::Install(package.clone())) } else { None };
                        if let Some(op) = op {
                            let label = if matches!(op, Operation::Update(_)) { "Atualizar" } else { "Instalar" };
                            if design::pill(ui, label, self.allowed(&op) && !self.updates_loading, true).clicked() { self.confirm = Some(op); }
                        } else { ui.label(RichText::new("Instalado").color(design::palette(ui.ctx()).secondary)); }
                        if package.installed_version.is_some() {
                            let op = Operation::Remove(package.clone());
                            if design::pill(ui, "Remover", self.allowed(&op), false).clicked() { self.confirm = Some(op); }
                        }
                    });
                });
            });
            ui.add_space(30.0);
            ui.separator();
            ui.add_space(16.0);
            ui.horizontal_wrapped(|ui| {
                for (label, value) in [
                    ("PACOTE", package.name.clone()),
                    ("INSTALADO", package.installed_version.clone().unwrap_or_else(|| "Não instalado".into())),
                    ("DISPONÍVEL", if package.source == Source::Local && update.is_none() { "Não encontrado".into() } else { update.as_ref().unwrap_or(package).version.clone() }),
                    ("ORIGEM", source_label(&package.source)),
                ] {
                    ui.vertical(|ui| { ui.set_min_width(145.0); ui.label(RichText::new(label).size(10.0).color(design::palette(ui.ctx()).secondary)); ui.label(RichText::new(value).size(14.0)); });
                }
            });
            ui.add_space(18.0); ui.separator(); ui.add_space(24.0);
            ui.label(design::heading("Sobre o aplicativo", 23.0));
            ui.add_space(10.0); ui.label(&package.description);
            if package.source == Source::Local { ui.label("Este pacote foi instalado de uma origem externa."); }
            if package.source == Source::Aur { ui.label(RichText::new("Pacote da comunidade AUR. Revise os arquivos de construção no terminal.").color(design::palette(ui.ctx()).secondary)); }
            let related = related_indices(&self.catalog, &package.name);
            if !related.is_empty() {
                ui.add_space(40.0); ui.separator(); ui.add_space(24.0);
                ui.label(design::heading("Aplicativos relacionados", 22.0));
                self.catalog_grid(ui, &related, "related");
            }
        });
    }
    fn catalog_grid(&mut self, ui: &mut egui::Ui, indices: &[usize], section: &str) {
        let columns = columns(ui.available_width());
        let width = (ui.available_width() - (columns - 1) as f32 * 24.0) / columns as f32;
        for row in indices.chunks(columns) {
            let (row_rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 80.0), egui::Sense::hover());
            for (column, &index) in row.iter().enumerate() {
                let rect = egui::Rect::from_min_size(
                    row_rect.min + egui::vec2(column as f32 * (width + 24.0), 0.0),
                    egui::vec2(width, 80.0),
                );
                let package = self.catalog.packages[index].clone();
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt((section, index))
                        .max_rect(rect),
                );
                self.package_row(&mut child, &package, false);
            }
        }
    }
    fn package_row(&mut self, ui: &mut egui::Ui, package: &Package, metadata: bool) {
        ui.push_id(("package_row", update_key(package)), |ui| {
            self.package_row_content(ui, package, metadata, None);
        });
    }
    fn package_group_row(&mut self, ui: &mut egui::Ui, package: &Package, repositories: usize) {
        ui.push_id(("package_group", &package.name), |ui| {
            self.package_row_content(ui, package, true, Some(repositories));
        });
    }
    fn package_repository_group(&mut self, ui: &mut egui::Ui, packages: &[Package]) {
        let package = &packages[0];
        let id = ui.id().with(("repository_expansion", &package.name));
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            id,
            self.expanded_packages.contains(&package.name),
        );
        self.package_group_row(ui, package, packages.len());
        state.set_open(self.expanded_packages.contains(&package.name));
        state.show_body_unindented(ui, |ui| {
            for package in packages {
                ui.indent(("repository_option", update_key(package)), |ui| {
                    self.package_row(ui, package, true);
                });
            }
        });
    }
    fn package_row_content(
        &mut self,
        ui: &mut egui::Ui,
        package: &Package,
        metadata: bool,
        repositories: Option<usize>,
    ) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 80.0), egui::Sense::hover());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let content_rect = rect.shrink2(egui::vec2(8.0, 0.0));
        if metadata {
            ui.painter().hline(
                rect.x_range(),
                rect.bottom(),
                egui::Stroke::new(1.0, design::palette(ui.ctx()).separator),
            );
        }
        let action_rect = egui::Rect::from_min_size(
            egui::pos2(
                content_rect.right() - if repositories.is_some() { 32.0 } else { 88.0 },
                rect.center().y - 14.0,
            ),
            egui::vec2(if repositories.is_some() { 32.0 } else { 88.0 }, 28.0),
        );
        let body_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(action_rect.left() - 8.0, rect.bottom()),
        );
        let response = ui
            .interact(
                if repositories.is_some() {
                    rect
                } else {
                    body_rect
                },
                ui.id().with(("package", &package.name)),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if ui.rect_contains_pointer(rect) {
            ui.painter()
                .rect_filled(rect, 6.0, design::palette(ui.ctx()).row_hover);
        }
        let icon_rect = egui::Rect::from_min_size(
            content_rect.min + egui::vec2(0.0, 16.0),
            egui::vec2(48.0, 48.0),
        );
        let mut icon_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("icon", &package.name))
                .max_rect(icon_rect),
        );
        design::app_icon_static(&mut icon_ui, package, 48.0);
        let text_left = content_rect.left() + 60.0;
        for (line, text, size, color) in [
            (
                0,
                display_name(package).to_owned(),
                15.0,
                design::palette(ui.ctx()).text,
            ),
            (
                1,
                tagline(package).to_owned(),
                12.0,
                design::palette(ui.ctx()).secondary,
            ),
            (
                2,
                if let Some(count) = repositories {
                    format!("{count} repositórios disponíveis")
                } else if self.tab == Tab::Updates {
                    format!(
                        "{} para {}",
                        package.installed_version.as_deref().unwrap_or("?"),
                        package.version
                    )
                } else if metadata {
                    format!("{} · {}", source_label(&package.source), package.version)
                } else {
                    String::new()
                },
                11.0,
                design::palette(ui.ctx()).secondary,
            ),
        ] {
            let line_rect = egui::Rect::from_min_size(
                egui::pos2(text_left, rect.top() + 14.0 + line as f32 * 18.0),
                egui::vec2((body_rect.right() - text_left - 4.0).max(1.0), 18.0),
            );
            let mut line_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("text", &package.name, line))
                    .max_rect(line_rect),
            );
            let rich = if line == 0 {
                design::heading(&text, size)
            } else {
                RichText::new(&text).size(size).color(color)
            };
            line_ui
                .add(egui::Label::new(rich).truncate().selectable(false))
                .on_hover_text(text);
        }
        let installed = self.installed_versions.contains_key(&package.name)
            || package.installed_version.is_some();
        let op = repositories
            .is_none()
            .then(|| {
                self.available_update(package)
                    .cloned()
                    .map(Operation::Update)
                    .or_else(|| (!installed).then(|| Operation::Install(package.clone())))
            })
            .flatten();
        let expanded = self.expanded_packages.contains(&package.name);
        let label = match &op {
            Some(Operation::Update(_)) => "Atualizar",
            Some(_) => "Instalar",
            None => "Detalhes",
        };
        let action_clicked = if repositories.is_some() {
            let openness = ui
                .ctx()
                .animate_bool_responsive(ui.id().with("disclosure_arrow"), expanded);
            let angle = openness * std::f32::consts::FRAC_PI_2;
            let points = [
                egui::vec2(-3.0, -5.0),
                egui::vec2(3.0, 0.0),
                egui::vec2(-3.0, 5.0),
            ]
            .map(|p| {
                action_rect.center()
                    + egui::vec2(
                        p.x * angle.cos() - p.y * angle.sin(),
                        p.x * angle.sin() + p.y * angle.cos(),
                    )
            });
            ui.painter().add(egui::Shape::line(
                points.to_vec(),
                egui::Stroke::new(2.0, design::palette(ui.ctx()).secondary),
            ));
            false // The whole group header, including the arrow, is clickable.
        } else {
            let mut action_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("action", &package.name))
                    .max_rect(action_rect),
            );
            design::pill(
                &mut action_ui,
                label,
                op.as_ref().is_none_or(|op| self.allowed(op)),
                false,
            )
            .clicked()
        };
        if repositories.is_some() && (action_clicked || response.clicked()) {
            if expanded {
                self.expanded_packages.remove(&package.name);
            } else {
                self.expanded_packages.insert(package.name.clone());
            }
            ui.ctx().request_repaint();
        } else if action_clicked {
            if let Some(op) = op {
                self.confirm = Some(op);
            } else {
                self.open_package(package.clone());
            }
        } else if response.clicked() {
            self.open_package(package.clone());
        }
    }
    fn explore(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = &self.error {
            ui.colored_label(design::palette(ui.ctx()).error, error);
        }
        if self.catalog_loading {
            centered_loader(ui);
            return;
        }
        if let Some(error) = &self.catalog_error {
            ui.colored_label(design::palette(ui.ctx()).error, error);
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(design::heading(if self.catalog.popularity_available { "Populares" } else { "Aplicativos" }, 22.0)); ui.add_space(14.0);
            let indices: Vec<usize> = (0..self.catalog.packages.len()).take(12).collect();
            self.catalog_grid(ui, &indices, "featured");
            ui.add_space(20.0); ui.separator(); ui.add_space(22.0);
            for category in catalog::CATEGORY_DEFINITIONS {
                let indices = category_indices(&self.catalog, Some(category.id));
                if indices.is_empty() { continue; }
                ui.horizontal(|ui| { ui.label(design::heading(category.title, 22.0)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.link("Ver todos").on_hover_cursor(egui::CursorIcon::PointingHand).clicked() { self.tab = Tab::Categories; self.collection = Some(category.id.into()); } }); });
                ui.add_space(12.0);
                self.catalog_grid(ui, &indices[..indices.len().min(12)], category.id); ui.add_space(22.0); ui.separator(); ui.add_space(22.0);
            }
            if self.catalog.packages.is_empty() && !self.catalog_loading { ui.label("Nenhum aplicativo encontrado nos repositórios configurados. Use a busca para consultar outros pacotes."); }
            for warning in &self.catalog.warnings { ui.label(RichText::new(warning).color(design::palette(ui.ctx()).secondary)); }
        });
    }
    fn worker(&self, ctx: &egui::Context, task: impl FnOnce() -> Message + Send + 'static) {
        let tx = self.sender.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(task());
            ctx.request_repaint();
        });
    }
    fn refresh(&mut self, ctx: &egui::Context) {
        if self.demo {
            return;
        }
        self.installed_loading = true;
        self.updates_loading = true;
        self.worker(ctx, || Message::Installed(backend::installed()));
        self.worker(ctx, || Message::Updates(backend::updates_with_warnings()));
    }
    fn search(&mut self, ctx: &egui::Context) {
        self.search_due = None;
        self.searching = false;
        self.search_id += 1;
        self.error = None;
        if self.demo {
            let query = self.query.to_lowercase();
            self.results = self
                .demo_packages
                .clone()
                .into_iter()
                .filter(|p| {
                    (self.aur || p.source != Source::Aur)
                        && format!("{} {} {}", p.name, p.display_name, p.description)
                            .to_lowercase()
                            .contains(&query)
                })
                .collect();
            pacmanager::search::rank_results(&mut self.results, &self.query, Some(&self.catalog));
            return;
        }
        if self.query.trim().is_empty() {
            self.results.clear();
            self.searching = false;
            return;
        }
        self.searching = true;
        let id = self.search_id;
        let query = self.query.trim().to_owned();
        let aur = self.aur;
        self.worker(ctx, move || {
            Message::Search(id, backend::search_with_warnings(&query, aur))
        });
    }
    fn schedule_search(&mut self, ctx: &egui::Context, now: Instant) {
        self.selected = None;
        self.expanded_packages.clear();
        self.search_id += 1;
        self.searching = false;
        self.results.clear();
        self.error = None;
        self.search_due =
            (!self.query.trim().is_empty()).then_some(now + Duration::from_millis(350));
        ctx.request_repaint_after(Duration::from_millis(350));
    }
    fn dispatch_due_search(&mut self, ctx: &egui::Context, now: Instant) {
        if self.tab != Tab::Explore {
            self.search_due = None;
        } else if self.search_due.is_some_and(|due| now >= due) {
            self.search(ctx);
        }
    }
    fn receive(&mut self, ctx: &egui::Context) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::Capabilities(c) => self.capabilities = Some(c),
                Message::Catalog(result) => {
                    self.catalog_loading = false;
                    match result {
                        Ok(packages) => {
                            self.catalog = packages;
                            pacmanager::search::rank_results(
                                &mut self.results,
                                &self.query,
                                Some(&self.catalog),
                            );
                        }
                        Err(error) => self.catalog_error = Some(error),
                    }
                }
                Message::Installed(result) => {
                    self.installed_loading = false;
                    match result {
                        Ok(p) => {
                            self.installed_versions = p
                                .iter()
                                .filter_map(|p| {
                                    p.installed_version.clone().map(|v| (p.name.clone(), v))
                                })
                                .collect();
                            self.installed = p;
                            for package in &mut self.results {
                                package.installed_version =
                                    self.installed_versions.get(&package.name).cloned();
                            }
                            if let Some(package) = &mut self.selected {
                                package.installed_version =
                                    self.installed_versions.get(&package.name).cloned();
                            }
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Message::Updates(result) => {
                    self.updates_loading = false;
                    match result {
                        Ok((p, warning)) => {
                            self.updates = p;
                            self.reindex_updates();
                            if warning.is_some() {
                                self.error = warning;
                            }
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Message::Search(id, result) if id == self.search_id => {
                    self.searching = false;
                    match result {
                        Ok((p, warning)) => {
                            if let Some(selected) = &mut self.selected
                                && let Some(package) = p.iter().find(|p| {
                                    p.name == selected.name && p.source == selected.source
                                })
                            {
                                *selected = package.clone();
                                selected.installed_version =
                                    self.installed_versions.get(&selected.name).cloned();
                            }
                            self.results = p;
                            pacmanager::search::rank_results(
                                &mut self.results,
                                &self.query,
                                Some(&self.catalog),
                            );
                            self.error = warning;
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
                Message::Search(_, _) => {}
                Message::Operation(result) => {
                    self.running = false;
                    match result {
                        Ok(()) => {
                            self.status = "Operação concluída. Lista atualizada.".into();
                            self.refresh(ctx);
                            if self.tab == Tab::Explore {
                                self.search(ctx);
                            }
                        }
                        Err(e) => {
                            self.status = "A operação não foi concluída.".into();
                            self.error = Some(e);
                        }
                    }
                }
            }
        }
    }
    fn allowed(&self, operation: &Operation) -> bool {
        self.capabilities.as_ref().is_some_and(|c| {
            c.pacman
                && c.terminal
                && (!matches!(operation, Operation::Install(p) | Operation::Update(p) if p.source == Source::Aur) || c.yay)
        }) && !matches!(operation, Operation::Install(p) | Operation::Update(p) if p.source == Source::Local)
            && !self.running
    }
    fn execute(&mut self, ctx: &egui::Context, operation: Operation) {
        if !self.allowed(&operation) {
            return;
        }
        if self.demo {
            self.status = "Prévia: nenhuma operação foi executada.".into();
            return;
        }
        self.running = true;
        self.error = None;
        self.status =
            "Continue no terminal e mantenha-o aberto até terminar, mesmo se fechar esta janela. Revise os pacotes antes de confirmar.".into();
        self.worker(ctx, move || {
            Message::Operation(backend::execute(&operation))
        });
    }
}
impl eframe::App for Store {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = root.ctx().clone();
        let ctx = &context;
        self.receive(ctx);
        self.dispatch_due_search(ctx, Instant::now());
        if self.running
            || self.searching
            || self.installed_loading
            || self.updates_loading
            || self.catalog_loading
        {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
        egui::Panel::left("navigation")
            .exact_size(230.0)
            .frame(
                egui::Frame::new()
                    .fill(design::palette(ctx).sidebar)
                    .inner_margin(16.0),
            )
            .show(root, |ui| {
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    design::pacman_logo(ui, 28.0);
                    let search_width = ui.available_width();
                    egui::Frame::new()
                        .fill(design::palette(ui.ctx()).search)
                        .corner_radius(7.0)
                        .inner_margin(7.0)
                        .show(ui, |ui| {
                            ui.set_width((search_width - 14.0).max(0.0));
                            ui.horizontal(|ui| {
                                design::line_icon(ui, design::NavIcon::Search, 15.0);
                                let text_width =
                                    (ui.available_width() - 16.0 - ui.spacing().item_spacing.x)
                                        .max(0.0);
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut self.query)
                                        .frame(egui::Frame::NONE)
                                        .hint_text("Buscar")
                                        .desired_width(text_width),
                                );
                                if response.changed() && self.tab == Tab::Explore {
                                    self.schedule_search(ctx, Instant::now());
                                }
                                if response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    self.selected = None;
                                    if self.tab == Tab::Explore {
                                        self.search(ctx);
                                    }
                                }
                                let filters = design::line_icon(ui, design::NavIcon::Filters, 16.0)
                                    .on_hover_text("Filtros");
                                filters.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        "Filtros",
                                    )
                                });
                                egui::Popup::menu(&filters).show(|ui| {
                                    let enabled = self.capabilities.as_ref().is_some_and(|c| c.yay);
                                    if ui
                                        .add_enabled(
                                            enabled,
                                            egui::Checkbox::new(&mut self.aur, "Incluir AUR"),
                                        )
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .changed()
                                        && self.tab == Tab::Explore
                                    {
                                        self.search(ctx);
                                    }
                                });
                            });
                        });
                });
                ui.add_space(22.0);
                for (tab, label, icon) in [
                    (Tab::Explore, "Explorar", design::NavIcon::Discover),
                    (Tab::Installed, "Instalados", design::NavIcon::Installed),
                    (Tab::Categories, "Categorias", design::NavIcon::Categories),
                    (Tab::Updates, "Atualizações", design::NavIcon::Updates),
                ] {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 36.0),
                        egui::Sense::hover(),
                    );
                    let response = ui
                        .interact(rect, ui.id().with(("nav", tab as u8)), egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let selected = self.tab == tab;
                    let fill = if selected {
                        design::palette(ui.ctx()).nav_selected
                    } else if response.hovered() {
                        design::palette(ui.ctx()).nav_hover
                    } else {
                        Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(rect, 6.0, fill);
                    let mut child = ui.new_child(
                        egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(8.0, 8.0))),
                    );
                    child.visuals_mut().override_text_color = Some(if selected {
                        design::palette(ui.ctx()).blue
                    } else {
                        design::palette(ui.ctx()).text
                    });
                    child.horizontal(|ui| {
                        design::line_icon(ui, icon, 18.0);
                        ui.add(egui::Label::new(RichText::new(label).size(14.0)).selectable(false));
                        if tab == Tab::Updates && !self.updates.is_empty() {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(self.updates.len().to_string()).size(11.0),
                                    );
                                },
                            );
                        }
                    });
                    if response.clicked() {
                        let changed = self.tab != tab;
                        self.tab = tab;
                        self.selected = None;
                        if tab != Tab::Explore {
                            self.search_due = None;
                        }
                        if changed && tab == Tab::Explore && !self.query.trim().is_empty() {
                            self.search(ctx);
                        }
                    }
                }
                let footer = egui::Rect::from_min_size(
                    egui::pos2(ui.max_rect().left(), ui.max_rect().bottom() - 40.0),
                    egui::vec2(ui.available_width(), 30.0),
                );
                let mut footer_ui = ui.new_child(egui::UiBuilder::new().max_rect(footer));
                footer_ui
                    .menu_button("Aparência", |ui| {
                        for (label, theme) in [
                            ("Claro", egui::ThemePreference::Light),
                            ("Escuro", egui::ThemePreference::Dark),
                            ("Sistema", egui::ThemePreference::System),
                        ] {
                            if ui
                                .button(label)
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                if let Err(error) = design::set_theme(ctx, theme, true) {
                                    self.error = Some(error);
                                }
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
            });
        if !self.status.is_empty() || self.running {
            egui::Panel::bottom("status")
                .frame(
                    egui::Frame::new()
                        .fill(design::palette(ctx).background)
                        .inner_margin(8.0),
                )
                .show(root, |ui| {
                    ui.horizontal(|ui| {
                        if self.running {
                            ui.spinner();
                        }
                        ui.label(
                            RichText::new(&self.status)
                                .size(11.0)
                                .color(design::palette(ui.ctx()).secondary),
                        );
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(design::palette(ctx).background)
                    .inner_margin(28.0),
            )
            .show(root, |ui| {
                if let Some(package) = self.selected.clone() {
                    self.package_page(ui, &package);
                    return;
                }
                let browsing = self.tab == Tab::Explore && self.query.trim().is_empty();
                let title = match self.tab {
                    Tab::Explore if browsing => "Explorar",
                    Tab::Explore => "Resultados da busca",
                    Tab::Installed => "Instalados",
                    Tab::Updates => "Atualizações",
                    Tab::Categories => "Categorias",
                };
                tab_header(ui, title);
                if browsing {
                    self.explore(ui);
                    return;
                }
                ui.horizontal_wrapped(|ui| {
                    if self.tab == Tab::Updates {
                        let sync = Operation::SyncRepositories;
                        if ui
                            .add_enabled(
                                self.allowed(&sync)
                                    && !self.installed_loading
                                    && !self.updates_loading,
                                egui::Button::new("Atualizar repositórios"),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(
                                "Sincronizar os índices e consultar as atualizações disponíveis",
                            )
                            .clicked()
                        {
                            self.confirm = Some(sync);
                        }
                        let op = Operation::Upgrade;
                        if design::pill_sized(
                            ui,
                            "Atualizar tudo",
                            self.allowed(&op) && !self.updates_loading,
                            true,
                            116.0,
                        )
                        .clicked()
                        {
                            self.confirm = Some(op);
                        }
                    }
                    if self.tab == Tab::Installed {
                        ui.checkbox(&mut self.all, "Mostrar componentes do sistema")
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                    }
                    if self.tab == Tab::Installed
                        && ui
                            .add_enabled(
                                !self.running && !self.installed_loading && !self.updates_loading,
                                egui::Button::new("Recarregar"),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                    {
                        self.refresh(ctx);
                    }
                    if self.tab == Tab::Categories {
                        for collection in catalog::CATEGORY_DEFINITIONS {
                            if category_indices(&self.catalog, Some(collection.id)).is_empty() {
                                continue;
                            }
                            if ui
                                .selectable_label(
                                    self.collection.as_deref() == Some(collection.id),
                                    collection.title,
                                )
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.collection = Some(collection.id.into());
                            }
                        }
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(design::palette(ui.ctx()).error, error);
                }
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(16.0);
                let loading = match self.tab {
                    Tab::Explore => self.searching,
                    Tab::Installed => self.installed_loading,
                    Tab::Updates => self.updates_loading,
                    Tab::Categories => self.catalog_loading,
                };
                if loading || (self.tab == Tab::Explore && self.search_due.is_some()) {
                    centered_loader(ui);
                    return;
                }
                if self.tab == Tab::Categories {
                    if let Some(error) = &self.catalog_error {
                        ui.colored_label(design::palette(ui.ctx()).error, error);
                    }
                    let indices = category_indices(&self.catalog, self.collection.as_deref());
                    let columns = columns(ui.available_width());
                    let rows = indices.len().div_ceil(columns);
                    egui::ScrollArea::vertical().show_rows(ui, 80.0, rows, |ui, range| {
                        let start = range.start * columns;
                        let end = (range.end * columns).min(indices.len());
                        self.catalog_grid(ui, &indices[start..end], "category");
                    });
                    return;
                }
                let source = match self.tab {
                    Tab::Explore => &self.results,
                    Tab::Installed => &self.installed,
                    Tab::Updates => &self.updates,
                    Tab::Categories => unreachable!(),
                };
                let query = self.query.to_lowercase();
                let indices: Vec<usize> = source
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        (self.tab != Tab::Installed || self.all || p.is_app)
                            && (self.tab == Tab::Explore
                                || query.is_empty()
                                || format!("{} {} {}", p.name, p.display_name, p.description)
                                    .to_lowercase()
                                    .contains(&query))
                    })
                    .map(|(i, _)| i)
                    .collect();
                let groups = package_groups(source, &indices);
                ui.label(
                    RichText::new(format!("{} pacotes", groups.len()))
                        .size(11.0)
                        .color(design::palette(ui.ctx()).secondary),
                );
                ui.add_space(12.0);
                if indices.is_empty() && !loading {
                    ui.label(if self.tab == Tab::Updates && self.error.is_none() {
                        "Nenhuma atualização disponível."
                    } else {
                        "Nenhum pacote encontrado."
                    });
                }
                ui.spacing_mut().item_spacing.y = 6.0;
                egui::ScrollArea::vertical()
                    .id_salt(("package_list", self.tab as u8))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for group in &groups {
                            let packages: Vec<Package> = group
                                .iter()
                                .map(|&index| {
                                    match self.tab {
                                        Tab::Explore => &self.results[index],
                                        Tab::Installed => &self.installed[index],
                                        _ => &self.updates[index],
                                    }
                                    .clone()
                                })
                                .collect();
                            if packages.len() == 1 {
                                self.package_row(ui, &packages[0], true);
                            } else {
                                self.package_repository_group(ui, &packages);
                            }
                        }
                    });
            });
        if let Some(operation) = self.confirm.clone() {
            let mut open = true;
            let mut proceed = false;
            let mut cancel = false;
            egui::Window::new("Confirmar operação").open(&mut open).collapsible(false).resizable(false).default_width(430.0).show(ctx, |ui| {
                match &operation {
                    Operation::Install(p) => { ui.heading(format!("Instalar {}?", p.display_name)); ui.label("O terminal mostrará os pacotes e as dependências antes da confirmação."); if p.source == Source::Aur { ui.label("Revise o PKGBUILD e os arquivos do AUR antes de continuar."); } },
                    Operation::Update(p) => { ui.heading(format!("Atualizar {}?", p.display_name)); if matches!(p.source, Source::Official(_)) { ui.label("Atualizar apenas este pacote pode causar incompatibilidades no Arch. A atualização completa é recomendada."); } else { ui.label("Revise os arquivos de construção e a transação no terminal antes de confirmar."); } },
                    Operation::Remove(p) => { ui.heading(format!("Remover {}?", p.display_name)); ui.label("O gerenciador verificará se outros pacotes dependem deste aplicativo. Confira o resumo no terminal."); },
                    Operation::Upgrade => { ui.heading("Atualizar o sistema completo?"); ui.label("O terminal atualizará os repositórios e todos os pacotes. Confira a transação e mantenha o terminal aberto até terminar."); },
                    Operation::SyncRepositories => { ui.heading("Atualizar os repositórios?"); ui.label("O terminal sincronizará os índices sem instalar pacotes. Ao terminar, a lista de atualizações será recarregada."); ui.label("Depois da sincronização, use Atualizar tudo antes de atualizar pacotes individuais para evitar incompatibilidades no Arch."); },
                }
                if let Operation::Install(package) | Operation::Update(package) = &operation {
                    ui.label(format!("{} · {}", source_label(&package.source), package.version));
                }
                if self.demo { ui.colored_label(design::palette(ui.ctx()).blue, "Esta prévia não executa comandos."); }
                ui.add_space(14.0); ui.horizontal(|ui| { proceed = ui.add_enabled(self.allowed(&operation), egui::Button::new("Continuar")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked(); cancel = ui.button("Cancelar").on_hover_cursor(egui::CursorIcon::PointingHand).clicked(); });
            });
            if proceed {
                self.confirm = None;
                self.execute(ctx, operation);
            } else if !open || cancel {
                self.confirm = None;
            }
        }
    }
}
fn tab_header(ui: &mut egui::Ui, title: &str) -> egui::Rect {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(13.0),
        design::palette(ui.ctx()).text,
    );
    ui.add_space(28.0);
    rect
}
fn centered_loader(ui: &mut egui::Ui) -> egui::Rect {
    let rect = ui.available_rect_before_wrap();
    let target = egui::Rect::from_center_size(rect.center(), egui::vec2(24.0, 24.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(target));
    child.add(egui::Spinner::new().size(24.0)).rect
}
fn update_key(package: &Package) -> String {
    let source = match &package.source {
        Source::Official(repo) => format!("official:{repo}"),
        Source::Aur => "aur".into(),
        Source::Local => "local".into(),
    };
    format!("{}:{source}", package.name)
}
// Keep each group's first search position and preserve the order of its sources.
fn package_groups(packages: &[Package], indices: &[usize]) -> Vec<Vec<usize>> {
    let mut positions = HashMap::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for &index in indices {
        let position = *positions
            .entry(packages[index].name.as_str())
            .or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
        groups[position].push(index);
    }
    groups
}
fn columns(width: f32) -> usize {
    if width >= 960.0 {
        3
    } else if width >= 600.0 {
        2
    } else {
        1
    }
}
fn display_name(package: &Package) -> &str {
    &package.display_name
}
fn tagline(package: &Package) -> &str {
    &package.description
}
fn category_indices(catalog: &catalog::Catalog, category: Option<&str>) -> Vec<usize> {
    catalog
        .packages
        .iter()
        .enumerate()
        .filter(|(_, package)| {
            category.is_none_or(|id| {
                catalog.metadata.get(&package.name).is_some_and(|metadata| {
                    metadata.categories.iter().any(|category| category == id)
                })
            })
        })
        .map(|(index, _)| index)
        .collect()
}
fn related_indices(catalog: &catalog::Catalog, name: &str) -> Vec<usize> {
    let Some(metadata) = catalog.metadata.get(name) else {
        return Vec::new();
    };
    catalog
        .packages
        .iter()
        .enumerate()
        .filter(|(_, package)| {
            package.name != name
                && catalog
                    .metadata
                    .get(&package.name)
                    .is_some_and(|candidate| {
                        candidate.categories.iter().any(|category| {
                            catalog::CATEGORY_DEFINITIONS
                                .iter()
                                .any(|definition| definition.id == category)
                                && metadata.categories.contains(category)
                        })
                    })
        })
        .take(6)
        .map(|(index, _)| index)
        .collect()
}
fn source_label(source: &Source) -> String {
    match source {
        Source::Official(repo) => format!("Oficial · {repo}"),
        Source::Aur => "Comunidade · AUR".into(),
        Source::Local => "Origem externa".into(),
    }
}
fn fixtures() -> Vec<Package> {
    let packages: Vec<Package> = [
        (
            "firefox",
            "Firefox",
            "Navegue na web com privacidade e extensões.",
            true,
            false,
        ),
        (
            "vlc",
            "VLC",
            "Reproduza seus vídeos, músicas e transmissões.",
            true,
            false,
        ),
        (
            "gimp",
            "GIMP",
            "Edite fotografias e crie imagens.",
            false,
            false,
        ),
        (
            "blender",
            "Blender",
            "Modele, anime e renderize cenas em 3D.",
            false,
            false,
        ),
        (
            "visual-studio-code-bin",
            "Visual Studio Code",
            "Editor de código com extensões e depuração.",
            false,
            true,
        ),
        (
            "inkscape",
            "Inkscape",
            "Desenhe ilustrações e gráficos vetoriais.",
            false,
            false,
        ),
        (
            "glibc",
            "GNU C Library",
            "Biblioteca básica do sistema.",
            true,
            false,
        ),
    ]
    .into_iter()
    .map(|(name, display, description, installed, aur)| Package {
        name: name.into(),
        display_name: display.into(),
        version: "1.0.2".into(),
        description: description.into(),
        source: if aur {
            Source::Aur
        } else {
            Source::Official("extra".into())
        },
        installed_version: installed.then(|| "1.0.1".into()),
        is_app: name != "glibc",
        icon_path: pacmanager::icons::resolve_app_icon(name),
        aur_popularity: None,
    })
    .collect();
    packages
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id_diagnostics(shape: &egui::Shape) -> bool {
        match shape {
            egui::Shape::Text(text) => {
                text.galley.job.text.contains("First use of")
                    || text.galley.job.text.contains("Second use of")
            }
            egui::Shape::Vec(shapes) => shapes.iter().any(id_diagnostics),
            _ => false,
        }
    }
    fn demo() -> Store {
        let mut store = Store::state(true);
        store.capabilities = Some(Capabilities {
            pacman: true,
            yay: true,
            terminal: true,
        });
        store
    }
    #[test]
    fn repository_expansion_animates_height_in_both_directions() {
        let ctx = egui::Context::default();
        design::install(&ctx);
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.style_mut_of(theme, |style| style.animation_time = 0.2);
        }
        let mut store = demo();
        let mut first = fixtures().remove(0);
        first.icon_path = None;
        let mut second = first.clone();
        second.source = Source::Official("cachyos-extra-v3".into());
        let packages = [first, second];
        let render = |store: &mut Store, time| {
            let mut height = 0.0;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    store.package_repository_group(ui, &packages);
                    height = ui.min_rect().height();
                },
            );
            output.textures_delta.clear();
            assert!(!output.shapes.iter().any(|s| id_diagnostics(&s.shape)));
            height
        };
        let closed = render(&mut store, 0.0);
        store.expanded_packages.insert(packages[0].name.clone());
        render(&mut store, 0.1);
        let opening = render(&mut store, 0.15);
        let opened = render(&mut store, 0.4);
        assert!(
            closed < opening && opening < opened,
            "{closed} < {opening} < {opened}"
        );
        store.expanded_packages.clear();
        render(&mut store, 0.45);
        let closing = render(&mut store, 0.5);
        let reclosed = render(&mut store, 0.8);
        assert!(reclosed < closing && closing < opened);
        assert_eq!(closed, reclosed);
    }
    #[test]
    fn hovering_the_action_highlights_the_entire_row() {
        let ctx = egui::Context::default();
        design::install(&ctx);
        let mut store = demo();
        let mut package = fixtures().remove(0);
        package.icon_path = None;
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(650.0, 80.0));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events: vec![egui::Event::PointerMoved(egui::pos2(
                    rect.right() - 44.0,
                    rect.center().y,
                ))],
                ..Default::default()
            },
            |ui| {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                store.package_row(&mut child, &package, true);
            },
        );
        output.textures_delta.clear();
        let color = design::palette(&ctx).row_hover;
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(shape) if shape.rect == rect && shape.fill == color)));
    }
    #[test]
    fn same_name_rows_have_unique_ids_per_source() {
        let ctx = egui::Context::default();
        design::install(&ctx);
        let mut store = demo();
        let mut package = fixtures().remove(0);
        package.icon_path = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for source in [
                Source::Official("extra".into()),
                Source::Official("cachyos-extra-v3".into()),
                Source::Aur,
            ] {
                package.source = source;
                store.package_row(ui, &package, true);
            }
        });
        output.textures_delta.clear();
        assert!(!output.shapes.iter().any(|s| id_diagnostics(&s.shape)));
    }
    #[test]
    fn groups_preserve_relevance_source_order_and_filters() {
        let mut packages = fixtures();
        packages.truncate(2);
        let mut duplicate = packages[0].clone();
        duplicate.source = Source::Official("cachyos-extra-v3".into());
        packages.push(duplicate.clone());
        duplicate.source = Source::Aur;
        packages.push(duplicate);
        assert_eq!(
            package_groups(&packages, &[0, 1, 2, 3]),
            vec![vec![0, 2, 3], vec![1]]
        );
        assert_eq!(
            package_groups(&packages, &[2, 1, 0]),
            vec![vec![2, 0], vec![1]]
        );
        assert_eq!(package_groups(&packages, &[1, 3]), vec![vec![1], vec![3]]);
    }
    #[test]
    fn repository_group_expands_selects_source_and_collapses_without_id_conflicts() {
        let ctx = egui::Context::default();
        design::install(&ctx);
        let mut store = demo();
        let mut first = fixtures().remove(0);
        first.installed_version = None;
        first.icon_path = None;
        first.source = Source::Official("cachyos-extra-v3".into());
        let mut second = first.clone();
        second.source = Source::Official("extra".into());
        let packages = [first, second];
        let render = |store: &mut Store, events| {
            let mut targets = Vec::new();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(
                        egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(650.0, 500.0)),
                    ));
                    child.spacing_mut().item_spacing.y = 6.0;
                    targets.push(child.next_widget_position() + egui::vec2(100.0, 40.0));
                    store.package_group_row(&mut child, &packages[0], 2);
                    if store.expanded_packages.contains(&packages[0].name) {
                        for package in &packages {
                            child.indent(("repository_option", update_key(package)), |ui| {
                                targets.push(
                                    ui.next_widget_position()
                                        + egui::vec2(ui.available_width() - 44.0, 40.0),
                                );
                                store.package_row(ui, package, true);
                            });
                        }
                    }
                },
            );
            output.textures_delta.clear();
            assert!(!output.shapes.iter().any(|s| id_diagnostics(&s.shape)));
            targets
        };
        let click = |store: &mut Store, pos| {
            for pressed in [true, false] {
                render(
                    store,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
        };
        let targets = render(&mut store, vec![]);
        assert_eq!(targets.len(), 1);
        click(&mut store, targets[0]);
        assert!(store.expanded_packages.contains(&packages[0].name));
        assert!(store.selected.is_none());
        assert!(store.confirm.is_none());
        let targets = render(&mut store, vec![]);
        assert_eq!(targets.len(), 3);
        click(&mut store, targets[2]);
        match store.confirm.take().unwrap() {
            Operation::Install(package) => {
                assert_eq!(package.source, Source::Official("extra".into()))
            }
            _ => panic!("expected install for the chosen repository"),
        }
        click(&mut store, targets[0]);
        assert!(!store.expanded_packages.contains(&packages[0].name));
        assert_eq!(render(&mut store, vec![]).len(), 1);
    }
    #[test]
    fn explore_repeated_apps_do_not_clash_widget_ids() {
        fn diagnostics(shape: &egui::Shape) -> bool {
            match shape {
                egui::Shape::Text(text) => {
                    text.galley.job.text.contains("First use of")
                        || text.galley.job.text.contains("Second use of")
                }
                egui::Shape::Vec(shapes) => shapes.iter().any(diagnostics),
                _ => false,
            }
        }
        let ctx = egui::Context::default();
        design::install(&ctx);
        let mut store = demo();
        store.catalog = catalog::demo(
            store
                .demo_packages
                .iter()
                .filter(|p| p.is_app && p.source != Source::Aur)
                .cloned()
                .collect(),
        );
        for package in &mut store.catalog.packages {
            package.icon_path = None;
        }
        for height in [760.0, 2200.0] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1100.0, height),
                    )),
                    ..Default::default()
                },
                |ui| {
                    store.explore(ui);
                },
            );
            output.textures_delta.clear();
            assert!(
                !output.shapes.iter().any(|shape| diagnostics(&shape.shape)),
                "explore emitted duplicate widget ID diagnostics"
            );
        }
    }
    #[test]
    fn package_rows_keep_geometry_and_click_body_for_long_labels() {
        for width in [514.0, 814.0, 1214.0] {
            let mut bounds = Vec::new();
            for long in [false, true] {
                let ctx = egui::Context::default();
                design::install(&ctx);
                let mut store = demo();
                let mut package = store.demo_packages[0].clone();
                if long {
                    package.display_name =
                        "A very long application name that must truncate".repeat(5);
                    package.description =
                        "A description that must remain on a single line".repeat(8);
                    package.name = "uncurated".into();
                }
                package.icon_path = None;
                let mut row = egui::Rect::NOTHING;
                let mut initial_output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let rect =
                        egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(width, 80.0));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    store.package_row(&mut child, &package, true);
                    row = child.min_rect();
                });
                initial_output.textures_delta.clear();
                bounds.push(row);
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events: vec![egui::Event::PointerMoved(
                            row.left_center() + egui::vec2(80.0, 0.0),
                        )],
                        ..Default::default()
                    },
                    |ui| {
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(
                            egui::Rect::from_min_size(
                                egui::pos2(20.0, 20.0),
                                egui::vec2(width, 80.0),
                            ),
                        ));
                        store.package_row(&mut child, &package, true);
                    },
                );
                output.textures_delta.clear();
                assert_eq!(
                    output.platform_output.cursor_icon,
                    egui::CursorIcon::PointingHand
                );
            }
            assert_eq!(bounds[0], bounds[1]);
            assert_eq!(bounds[0].height(), 80.0);
            assert_eq!(bounds[0].width(), width);
        }
    }
    #[test]
    fn categories_and_suggestions_follow_metadata_membership() {
        let store = demo();
        let catalog = catalog::demo(
            store
                .demo_packages
                .into_iter()
                .filter(|p| p.is_app && p.source != Source::Aur)
                .collect(),
        );
        let graphics = category_indices(&catalog, Some("Graphics"));
        assert!(!graphics.is_empty());
        assert!(graphics.iter().all(|&index| {
            catalog.metadata[&catalog.packages[index].name]
                .categories
                .iter()
                .any(|id| id == "Graphics")
        }));
        assert!(category_indices(&catalog, Some("unknown")).is_empty());
        let related = related_indices(&catalog, "gimp");
        assert!(!related.is_empty());
        assert!(related.iter().all(|&index| {
            let name = &catalog.packages[index].name;
            name != "gimp" && name != "firefox" && name != "vlc"
        }));
        assert!(related_indices(&catalog, "unknown").is_empty());
        assert!(related_indices(&catalog, "firefox").is_empty());
    }
    #[test]
    fn category_filters_accept_new_packages_from_metadata() {
        let mut package = fixtures().remove(0);
        package.name = "new-appstream-package".into();
        let mut catalog = catalog::Catalog::default();
        catalog.metadata.insert(
            package.name.clone(),
            catalog::AppMetadata {
                categories: vec!["Science".into()],
                popularity: None,
            },
        );
        catalog.packages.push(package);
        assert_eq!(category_indices(&catalog, Some("Science")), vec![0]);
        assert!(category_indices(&catalog, Some("Network")).is_empty());
        assert_eq!(category_indices(&catalog, None), vec![0]);
        catalog.metadata.clear();
        assert!(category_indices(&catalog, Some("Science")).is_empty());
    }
    #[test]
    fn suggestions_do_not_match_toolkit_categories() {
        let mut packages = fixtures();
        packages.truncate(2);
        let mut catalog = catalog::Catalog {
            packages,
            ..Default::default()
        };
        catalog.metadata.insert(
            "firefox".into(),
            catalog::AppMetadata {
                categories: vec!["Qt".into(), "Network".into()],
                popularity: None,
            },
        );
        catalog.metadata.insert(
            "vlc".into(),
            catalog::AppMetadata {
                categories: vec!["Qt".into(), "Graphics".into()],
                popularity: None,
            },
        );
        assert!(related_indices(&catalog, "firefox").is_empty());
        catalog
            .metadata
            .get_mut("vlc")
            .unwrap()
            .categories
            .push("Network".into());
        assert_eq!(related_indices(&catalog, "firefox"), vec![1]);
    }
    #[test]
    fn headers_share_geometry_and_loaders_center_in_remaining_area() {
        for width in [360.0, 800.0, 1200.0] {
            let mut baseline = None;
            for title in [
                "Explorar",
                "Resultados da busca",
                "Instalados",
                "Atualizações",
                "Categorias",
            ] {
                let ctx = egui::Context::default();
                design::install(&ctx);
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    let bounds =
                        egui::Rect::from_min_size(egui::pos2(30.0, 40.0), egui::vec2(width, 500.0));
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bounds));
                    let header = tab_header(&mut child, title);
                    assert_eq!(header.width(), width);
                    assert_eq!(header.height(), 18.0);
                    assert_eq!(header.center().x, bounds.center().x);
                    if let Some(previous) = baseline {
                        assert_eq!(header, previous);
                    }
                    baseline = Some(header);
                    let available = child.available_rect_before_wrap();
                    let loading = centered_loader(&mut child);
                    assert_eq!(loading.center(), available.center());
                    assert_eq!(loading.size(), egui::vec2(24.0, 24.0));
                    assert!(loading.top() > header.bottom());
                });
                output.textures_delta.clear();
            }
        }
    }
    #[test]
    fn search_ranking_uses_catalog_in_either_completion_order() {
        let ctx = egui::Context::default();
        for catalog_first in [false, true] {
            let mut store = demo();
            store.query = "obs".into();
            let mut packages = fixtures();
            packages.truncate(2);
            for (p, name) in packages.iter_mut().zip(["obs-alpha", "obs-studio"]) {
                p.name = name.into();
                p.display_name = name.into();
                p.is_app = true;
            }
            let mut catalog = catalog::Catalog::default();
            catalog.package_popularity.insert("obs-studio".into(), 20.0);
            let catalog_message = Message::Catalog(Ok(catalog));
            let search_message = Message::Search(store.search_id, Ok((packages, None)));
            let messages = if catalog_first {
                [catalog_message, search_message]
            } else {
                [search_message, catalog_message]
            };
            for message in messages {
                store.sender.send(message).unwrap();
                store.receive(&ctx);
            }
            assert_eq!(store.results[0].name, "obs-studio");
        }
    }
    #[test]
    fn typing_debounces_search_and_invalidates_stale_responses() {
        let ctx = egui::Context::default();
        let mut store = demo();
        let now = Instant::now();
        store.query = "fire".into();
        store.open_demo_package("firefox");
        assert!(store.selected.is_some());
        store.schedule_search(&ctx, now);
        assert!(store.selected.is_none());
        let stale_id = store.search_id;
        store.dispatch_due_search(&ctx, now + Duration::from_millis(349));
        assert!(store.results.is_empty());
        assert!(store.search_due.is_some());
        store.query = "gimp".into();
        store.schedule_search(&ctx, now + Duration::from_millis(100));
        store
            .sender
            .send(Message::Search(
                stale_id,
                Ok((vec![fixtures().remove(0)], None)),
            ))
            .unwrap();
        store.receive(&ctx);
        assert!(store.results.is_empty());
        store.dispatch_due_search(&ctx, now + Duration::from_millis(449));
        assert!(store.results.is_empty());
        store.dispatch_due_search(&ctx, now + Duration::from_millis(450));
        assert_eq!(store.results.len(), 1);
        assert_eq!(store.results[0].name, "gimp");
        assert!(store.search_due.is_none());
        store.query = "vlc".into();
        store.schedule_search(&ctx, now);
        store.search(&ctx);
        assert_eq!(store.results[0].name, "vlc");
        assert!(store.search_due.is_none());
        store.query.clear();
        store.schedule_search(&ctx, now);
        assert!(store.search_due.is_none());
        assert!(store.results.is_empty());
        store.query = "fire".into();
        store.schedule_search(&ctx, now);
        store.tab = Tab::Installed;
        store.open_demo_package("gimp");
        store.dispatch_due_search(&ctx, now + Duration::from_millis(350));
        assert!(store.search_due.is_none());
        assert!(store.results.is_empty());
        assert_eq!(store.selected.as_ref().unwrap().name, "gimp");
    }
    #[test]
    fn package_page_preserves_list_filters() {
        let mut store = demo();
        store.tab = Tab::Installed;
        store.query = "fire".into();
        store.all = true;
        store.aur = true;
        store.open_demo_package("firefox");
        assert!(store.selected.is_some());
        store.close_package();
        assert!(store.selected.is_none());
        assert!(store.tab == Tab::Installed);
        assert_eq!(store.query, "fire");
        assert!(store.all && store.aur);
    }
    #[test]
    fn demo_update_never_dispatches_backend() {
        let mut store = demo();
        let package = store.demo_packages[0].clone();
        store.execute(&egui::Context::default(), Operation::Update(package));
        assert!(!store.running);
        assert!(store.receiver.try_recv().is_err());
        assert!(store.status.contains("nenhuma operação"));
    }
    #[test]
    fn repository_sync_respects_capabilities_and_demo_mode() {
        let mut store = demo();
        let op = Operation::SyncRepositories;
        store.capabilities.as_mut().unwrap().yay = false;
        assert!(store.allowed(&op));
        store.running = true;
        assert!(!store.allowed(&op));
        store.running = false;
        store.capabilities.as_mut().unwrap().terminal = false;
        assert!(!store.allowed(&op));
        store.capabilities.as_mut().unwrap().terminal = true;
        store.capabilities.as_mut().unwrap().pacman = false;
        assert!(!store.allowed(&op));
        store.capabilities.as_mut().unwrap().pacman = true;
        store.execute(&egui::Context::default(), op);
        assert!(!store.running);
        assert!(store.receiver.try_recv().is_err());
        assert!(store.status.contains("nenhuma operação"));
    }
    #[test]
    fn update_requires_matching_source_or_local_aur() {
        let mut store = demo();
        let mut package = store.demo_packages[0].clone();
        package.source = Source::Aur;
        store.updates = vec![package.clone()];
        store.reindex_updates();
        assert!(store.available_update(&store.demo_packages[0]).is_none());
        package.source = Source::Local;
        assert!(matches!(
            store.available_update(&package).map(|p| &p.source),
            Some(Source::Aur)
        ));
        package.name = "unknown".into();
        assert!(store.available_update(&package).is_none());
    }
}
