//! Theme-aware store styling and resolution-independent vector artwork.
use eframe::egui::{self, Color32, Rect, Response, RichText, Stroke, Vec2};
use pacmanager::model::Package;

#[derive(Clone, Copy)]
pub struct Palette {
    pub text: Color32,
    pub secondary: Color32,
    pub blue: Color32,
    pub sidebar: Color32,
    pub separator: Color32,
    pub background: Color32,
    pub search: Color32,
    pub nav_selected: Color32,
    pub nav_hover: Color32,
    pub row_hover: Color32,
    pub error: Color32,
    /// Fundo de cartões e faixas de metadados.
    pub card: Color32,
    /// Fundo de botões secundários e chips.
    pub control: Color32,
    /// Realce suave do azul: hover de botões e avisos informativos.
    pub accent_soft: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub error_soft: Color32,
}

fn theme_palette(theme: egui::Theme) -> Palette {
    if theme == egui::Theme::Dark {
        Palette {
            text: Color32::from_rgb(236, 236, 239),
            secondary: Color32::from_rgb(172, 172, 181),
            blue: Color32::from_rgb(110, 174, 255),
            sidebar: Color32::from_rgb(30, 30, 34),
            separator: Color32::from_rgb(58, 58, 65),
            background: Color32::from_rgb(23, 23, 27),
            search: Color32::from_rgb(43, 43, 49),
            nav_selected: Color32::from_rgb(57, 57, 66),
            nav_hover: Color32::from_rgb(46, 46, 53),
            row_hover: Color32::from_rgb(34, 34, 40),
            error: Color32::from_rgb(255, 133, 133),
            card: Color32::from_rgb(32, 32, 37),
            control: Color32::from_rgb(43, 43, 49),
            accent_soft: Color32::from_rgb(33, 47, 69),
            danger: Color32::from_rgb(207, 64, 64),
            warning: Color32::from_rgb(242, 190, 98),
            warning_soft: Color32::from_rgb(51, 42, 24),
            error_soft: Color32::from_rgb(60, 29, 31),
        }
    } else {
        Palette {
            text: Color32::from_rgb(29, 29, 31),
            secondary: Color32::from_rgb(110, 110, 115),
            blue: Color32::from_rgb(0, 102, 218),
            sidebar: Color32::from_rgb(238, 238, 240),
            separator: Color32::from_rgb(228, 228, 231),
            background: Color32::WHITE,
            search: Color32::from_rgb(225, 225, 228),
            nav_selected: Color32::from_rgb(218, 218, 223),
            nav_hover: Color32::from_rgb(229, 229, 233),
            row_hover: Color32::from_rgb(247, 247, 249),
            error: Color32::from_rgb(180, 40, 40),
            card: Color32::from_rgb(246, 246, 248),
            control: Color32::from_rgb(240, 240, 246),
            accent_soft: Color32::from_rgb(230, 239, 252),
            danger: Color32::from_rgb(209, 43, 43),
            warning: Color32::from_rgb(146, 88, 0),
            warning_soft: Color32::from_rgb(255, 246, 224),
            error_soft: Color32::from_rgb(253, 236, 236),
        }
    }
}

pub fn palette(ctx: &egui::Context) -> Palette {
    theme_palette(ctx.theme())
}

fn preference_path() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .map(|root| root.join("pacmanager/theme"))
}

fn parse_preference(value: &str) -> egui::ThemePreference {
    match value.trim() {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    }
}

pub fn theme_preference() -> egui::ThemePreference {
    preference_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|value| parse_preference(&value))
        .unwrap_or(egui::ThemePreference::System)
}

pub fn set_theme(
    ctx: &egui::Context,
    preference: egui::ThemePreference,
    persist: bool,
) -> Result<(), String> {
    ctx.set_theme(preference);
    if persist {
        let path = preference_path().ok_or("Não foi possível localizar a pasta de configuração")?;
        std::fs::create_dir_all(path.parent().expect("theme has a parent"))
            .map_err(|error| format!("Não foi possível salvar o tema: {error}"))?;
        let value = match preference {
            egui::ThemePreference::Light => "light",
            egui::ThemePreference::Dark => "dark",
            egui::ThemePreference::System => "system",
        };
        std::fs::write(path, value)
            .map_err(|error| format!("Não foi possível salvar o tema: {error}"))?;
    }
    Ok(())
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, file) in [
        ("store-regular", "NotoSans-Regular.ttf"),
        ("store-bold", "NotoSans-Bold.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(format!("/usr/share/fonts/noto/{file}")) {
            fonts
                .font_data
                .insert(name.into(), egui::FontData::from_owned(bytes).into());
            if name == "store-regular" {
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, name.into());
            } else {
                fonts
                    .families
                    .insert(egui::FontFamily::Name(name.into()), vec![name.into()]);
            }
        }
    }
    // The named family remains valid on systems without Noto Sans.
    if !fonts
        .families
        .contains_key(&egui::FontFamily::Name("store-bold".into()))
    {
        let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
        fonts
            .families
            .insert(egui::FontFamily::Name("store-bold".into()), fallback);
    }
    ctx.set_fonts(fonts);
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let colors = theme_palette(theme);
        let mut style = (*ctx.style_of(theme)).clone();
        style.visuals = match theme {
            egui::Theme::Light => egui::Visuals::light(),
            egui::Theme::Dark => egui::Visuals::dark(),
        };
        style.visuals.override_text_color = Some(colors.text);
        style.visuals.panel_fill = colors.background;
        style.visuals.window_fill = colors.background;
        style.visuals.extreme_bg_color = colors.row_hover;
        style.visuals.faint_bg_color = colors.row_hover;
        style.visuals.selection.stroke = Stroke::new(1.0, colors.blue);
        style.visuals.hyperlink_color = colors.blue;
        style.visuals.error_fg_color = colors.error;
        for visuals in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.noninteractive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            visuals.fg_stroke = Stroke::new(1.0, colors.text);
            visuals.bg_stroke = Stroke::new(1.0, colors.separator);
            visuals.corner_radius = 6.into();
        }
        style.visuals.widgets.inactive.bg_fill = colors.search;
        style.visuals.widgets.hovered.bg_fill = colors.nav_hover;
        style.visuals.widgets.active.bg_fill = colors.nav_selected;
        style.visuals.widgets.open.bg_fill = colors.nav_selected;
        style.visuals.window_corner_radius = 14.into();
        style.visuals.menu_corner_radius = 10.into();
        style.visuals.window_stroke = Stroke::new(1.0, colors.separator);
        let shadow_alpha = if theme == egui::Theme::Dark { 110 } else { 28 };
        style.visuals.popup_shadow = egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(shadow_alpha),
        };
        style.visuals.window_shadow = egui::Shadow {
            offset: [0, 12],
            blur: 40,
            spread: 0,
            color: Color32::from_black_alpha(shadow_alpha + 12),
        };
        style.visuals.text_cursor.stroke = Stroke::new(2.0, colors.blue);
        style.visuals.selection.bg_fill = colors.blue.gamma_multiply(0.3);
        style.spacing.scroll = egui::style::ScrollStyle::floating();
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, bold_font(28.0));
        ctx.set_style_of(theme, style);
    }
    ctx.set_theme(theme_preference());
}

pub fn bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("store-bold".into()))
}

pub fn heading(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(bold_font(size))
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    /// Ação principal: azul preenchido.
    Primary,
    /// Ação secundária: fundo neutro e texto azul.
    Secondary,
    /// Ação destrutiva: vermelho preenchido.
    Danger,
}

/// Suaviza a troca entre cores de repouso e hover.
fn hover_mix(ui: &egui::Ui, id: egui::Id, hovered: bool, rest: Color32, hover: Color32) -> Color32 {
    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("hover_mix"), hovered, 0.12);
    rest.lerp_to_gamma(hover, t)
}

pub fn pill(ui: &mut egui::Ui, label: &str, enabled: bool, primary: bool) -> Response {
    pill_sized(ui, label, enabled, primary, 88.0)
}

pub fn pill_sized(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    primary: bool,
    width: f32,
) -> Response {
    let tone = if primary {
        Tone::Primary
    } else {
        Tone::Secondary
    };
    pill_tone(ui, label, enabled, tone, width)
}

/// Botão em cápsula; cresce para caber o rótulo quando `min_width` não basta.
pub fn pill_tone(
    ui: &mut egui::Ui,
    label: &str,
    enabled: bool,
    tone: Tone,
    min_width: f32,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        let colors = palette(ui.ctx());
        let dark = ui.visuals().dark_mode;
        let font = bold_font(12.0);
        let galley_width = ui
            .painter()
            .layout_no_wrap(label.into(), font.clone(), colors.text)
            .size()
            .x;
        let width = min_width.max(galley_width + 32.0);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
        let (rest, hover, pressed, text) = match tone {
            Tone::Primary if dark => (
                Color32::from_rgb(35, 104, 194),
                Color32::from_rgb(47, 121, 219),
                Color32::from_rgb(25, 81, 155),
                Color32::WHITE,
            ),
            Tone::Primary => (
                colors.blue,
                Color32::from_rgb(0, 87, 193),
                Color32::from_rgb(0, 70, 164),
                Color32::WHITE,
            ),
            Tone::Danger => (
                colors.danger,
                colors.danger.lerp_to_gamma(Color32::BLACK, 0.1),
                colors.danger.lerp_to_gamma(Color32::BLACK, 0.22),
                Color32::WHITE,
            ),
            Tone::Secondary if dark => (
                colors.control,
                colors.accent_soft,
                colors.nav_selected,
                colors.blue,
            ),
            Tone::Secondary => (
                colors.control,
                colors.accent_soft,
                Color32::from_rgb(210, 226, 248),
                colors.blue,
            ),
        };
        let fill = if !enabled {
            colors.control
        } else if response.is_pointer_button_down_on() {
            pressed
        } else {
            hover_mix(ui, response.id, response.hovered(), rest, hover)
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 14, fill);
        let color = if enabled { text } else { colors.secondary };
        let galley = painter.layout_no_wrap(label.into(), font, color);
        painter
            .with_clip_rect(rect.shrink2(egui::vec2(6.0, 0.0)))
            .galley(rect.center() - galley.size() * 0.5, galley, color);
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
}

/// Filtro em cápsula com estado selecionado.
pub fn chip(ui: &mut egui::Ui, label: &str, selected: bool) -> Response {
    let colors = palette(ui.ctx());
    let font = egui::FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.into(), font.clone(), colors.text);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 28.0, 30.0),
        egui::Sense::click(),
    );
    let (fill, text) = if selected {
        (colors.text, colors.background)
    } else {
        (
            hover_mix(
                ui,
                response.id,
                response.hovered(),
                colors.control,
                colors.nav_hover,
            ),
            colors.text,
        )
    };
    ui.painter().rect_filled(rect, 15, fill);
    let galley = ui.painter().layout_no_wrap(label.into(), font, text);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, text);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Interruptor com rótulo à esquerda.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str) -> Response {
    let colors = palette(ui.ctx());
    let font = egui::FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(label.into(), font, colors.text);
    let (rect, mut response) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 10.0 + SWITCH.x, 28.0),
        egui::Sense::click(),
    );
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, label));
    let text_pos = egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(text_pos, galley, colors.text);
    draw_switch(
        ui,
        &response,
        *on,
        rect.right_center() - egui::vec2(SWITCH.x * 0.5, 0.0),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

const SWITCH: Vec2 = egui::vec2(34.0, 20.0);

fn draw_switch(ui: &egui::Ui, response: &Response, on: bool, center: egui::Pos2) {
    let colors = palette(ui.ctx());
    let t = ui.ctx().animate_bool_with_time(response.id, on, 0.15);
    let track = Rect::from_center_size(center, SWITCH);
    let off = if ui.visuals().dark_mode {
        colors.nav_selected
    } else {
        Color32::from_rgb(214, 214, 220)
    };
    let fill = off.lerp_to_gamma(colors.blue, t);
    let fill = if ui.is_enabled() {
        fill
    } else {
        fill.gamma_multiply(0.5)
    };
    ui.painter().rect_filled(track, SWITCH.y * 0.5, fill);
    let knob_x = egui::lerp(
        (track.left() + SWITCH.y * 0.5)..=(track.right() - SWITCH.y * 0.5),
        t,
    );
    let knob = egui::pos2(knob_x, track.center().y);
    ui.painter().circle_filled(
        knob + egui::vec2(0.0, 0.5),
        SWITCH.y * 0.5 - 1.5,
        Color32::from_black_alpha(40),
    );
    ui.painter()
        .circle_filled(knob, SWITCH.y * 0.5 - 2.0, Color32::WHITE);
}

/// Linha de opção com título, descrição e interruptor alinhado à direita.
pub fn switch_row(ui: &mut egui::Ui, on: &mut bool, title: &str, detail: &str) -> Response {
    let colors = palette(ui.ctx());
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, title)
    });
    let text = if ui.is_enabled() {
        colors.text
    } else {
        colors.secondary
    };
    ui.painter().text(
        rect.left_center() - egui::vec2(0.0, 8.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        text,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(0.0, 9.0),
        egui::Align2::LEFT_CENTER,
        detail,
        egui::FontId::proportional(11.0),
        colors.secondary,
    );
    draw_switch(
        ui,
        &response,
        *on,
        rect.right_center() - egui::vec2(SWITCH.x * 0.5, 0.0),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Botão de filtros do campo de busca; azul e com ponto quando há filtro ativo.
pub fn filter_button(ui: &mut egui::Ui, active: bool) -> Response {
    let colors = palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let background = Rect::from_center_size(rect.center(), Vec2::splat(24.0));
    let fill = if open {
        colors.nav_selected
    } else {
        hover_mix(
            ui,
            response.id,
            response.hovered(),
            Color32::TRANSPARENT,
            colors.nav_hover,
        )
    };
    ui.painter().rect_filled(background, 6, fill);
    let color = if active {
        colors.blue
    } else if open || response.hovered() {
        colors.text
    } else {
        colors.secondary
    };
    draw_symbol(ui.painter(), rect.shrink(0.5), NavIcon::Filters, color);
    if active {
        ui.painter()
            .circle_filled(rect.right_top() + egui::vec2(1.0, -1.0), 3.0, colors.blue);
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, active, "Filtros")
    });
    let response = response.on_hover_text(if active {
        "Filtros: AUR incluído"
    } else {
        "Filtros"
    });
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Botão discreto da barra lateral com ícone e rótulo, no estilo da navegação.
pub fn sidebar_button(ui: &mut egui::Ui, icon: NavIcon, label: &str) -> Response {
    let colors = palette(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), egui::Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
    let fill = if open {
        colors.nav_selected
    } else {
        hover_mix(
            ui,
            response.id,
            response.hovered(),
            Color32::TRANSPARENT,
            colors.nav_hover,
        )
    };
    ui.painter().rect_filled(rect, 8, fill);
    let color = if open || response.hovered() {
        colors.text
    } else {
        colors.secondary
    };
    draw_symbol(
        ui.painter(),
        Rect::from_center_size(
            rect.left_center() + egui::vec2(18.5, 0.0),
            Vec2::splat(16.0),
        ),
        icon,
        color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(37.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        color,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Item de menu com marca de seleção à esquerda.
pub fn menu_item(ui: &mut egui::Ui, label: &str, checked: bool) -> Response {
    let colors = palette(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, checked, label)
    });
    let fill = hover_mix(
        ui,
        response.id,
        response.hovered(),
        Color32::TRANSPARENT,
        colors.nav_hover,
    );
    ui.painter().rect_filled(rect, 6, fill);
    if checked {
        draw_symbol(
            ui.painter(),
            Rect::from_center_size(
                rect.left_center() + egui::vec2(15.0, 0.0),
                Vec2::splat(12.0),
            ),
            NavIcon::Check,
            colors.blue,
        );
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(30.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        colors.text,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Contador em cápsula azul.
pub fn badge(ui: &mut egui::Ui, text: &str, emphasized: bool) -> Response {
    let colors = palette(ui.ctx());
    let font = bold_font(11.0);
    let (fill, color) = if emphasized {
        (colors.blue, Color32::WHITE)
    } else {
        (colors.accent_soft, colors.blue)
    };
    let galley = ui.painter().layout_no_wrap(text.into(), font, color);
    let size = egui::vec2((galley.size().x + 12.0).max(20.0), 18.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 9, fill);
    ui.painter()
        .galley(rect.center() - galley.size() * 0.5, galley, color);
    response
}

#[derive(Clone, Copy)]
pub enum Notice {
    Info,
    Warning,
    Error,
}

/// Aviso em faixa colorida com ícone circular.
pub fn callout(ui: &mut egui::Ui, kind: Notice, text: &str) -> Response {
    let colors = palette(ui.ctx());
    let (fill, accent, glyph) = match kind {
        Notice::Info => (colors.accent_soft, colors.blue, "i"),
        Notice::Warning => (colors.warning_soft, colors.warning, "!"),
        Notice::Error => (colors.error_soft, colors.error, "!"),
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 9.0, accent);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    bold_font(12.0),
                    if ui.visuals().dark_mode {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    },
                );
                ui.add(egui::Label::new(RichText::new(text).size(13.0).color(colors.text)).wrap());
            });
        })
        .response
}

/// Estado vazio centralizado no espaço restante.
pub fn empty_state(ui: &mut egui::Ui, icon: NavIcon, title: &str, message: &str) {
    let colors = palette(ui.ctx());
    let area = ui.available_rect_before_wrap();
    let height = 64.0 + 16.0 + 24.0 + 6.0 + 40.0;
    let top = area.top() + ((area.height() - height) * 0.42).max(24.0);
    let width = area.width().min(380.0);
    let column = Rect::from_min_size(
        egui::pos2(area.center().x - width * 0.5, top),
        egui::vec2(width, height),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(column)
            .layout(egui::Layout::top_down(egui::Align::Center)),
    );
    let (rect, _) = child.allocate_exact_size(Vec2::splat(64.0), egui::Sense::hover());
    child
        .painter()
        .circle_filled(rect.center(), 32.0, colors.card);
    draw_symbol(
        child.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(28.0)),
        icon,
        colors.secondary,
    );
    child.add_space(6.0);
    child.label(heading(title, 17.0));
    child.add(
        egui::Label::new(RichText::new(message).size(13.0).color(colors.secondary))
            .halign(egui::Align::Center)
            .wrap(),
    );
}

/// Botão de voltar com seta e rótulo.
pub fn back_button(ui: &mut egui::Ui) -> Response {
    let colors = palette(ui.ctx());
    let font = egui::FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap("Voltar".into(), font, colors.blue);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + 36.0, 28.0),
        egui::Sense::click(),
    );
    let fill = hover_mix(
        ui,
        response.id,
        response.hovered(),
        Color32::TRANSPARENT,
        colors.accent_soft,
    );
    ui.painter().rect_filled(rect, 14, fill);
    draw_symbol(
        ui.painter(),
        Rect::from_center_size(
            rect.left_center() + egui::vec2(15.0, 0.0),
            Vec2::splat(13.0),
        ),
        NavIcon::Back,
        colors.blue,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 26.0, rect.center().y - galley.size().y * 0.5),
        galley,
        colors.blue,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Retângulo arredondado preenchido com uma cor por posição.
/// Um polígono sólido por baixo fornece a borda suavizada.
pub fn gradient_rect(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    color_at: impl Fn(f32, f32) -> Color32,
) {
    let radius = radius.min(rect.width() * 0.5).min(rect.height() * 0.5);
    let points = rounded_points(rect.shrink(0.5), (radius - 0.5).max(0.0));
    painter.rect_filled(rect, radius, color_at(0.5, 0.5));
    let mut mesh = egui::Mesh::default();
    let color = |p: egui::Pos2| {
        color_at(
            (p.x - rect.left()) / rect.width(),
            (p.y - rect.top()) / rect.height(),
        )
    };
    mesh.colored_vertex(rect.center(), color(rect.center()));
    for &point in &points {
        mesh.colored_vertex(point, color(point));
    }
    let count = points.len() as u32;
    for index in 0..count {
        mesh.add_triangle(0, 1 + index, 1 + (index + 1) % count);
    }
    painter.add(mesh);
}

fn rounded_points(rect: Rect, radius: f32) -> Vec<egui::Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let corners = [
        (
            egui::pos2(rect.right() - radius, rect.top() + radius),
            -FRAC_PI_2,
        ),
        (
            egui::pos2(rect.right() - radius, rect.bottom() - radius),
            0.0,
        ),
        (
            egui::pos2(rect.left() + radius, rect.bottom() - radius),
            FRAC_PI_2,
        ),
        (egui::pos2(rect.left() + radius, rect.top() + radius), PI),
    ];
    let mut points = Vec::with_capacity(corners.len() * 9);
    for (center, start) in corners {
        for step in 0..=8 {
            let angle = start + step as f32 / 8.0 * FRAC_PI_2;
            points.push(center + radius * egui::vec2(angle.cos(), angle.sin()));
        }
    }
    points
}

/// Cor estável derivada do nome, para ícones genéricos com identidade própria.
pub fn identity_color(name: &str) -> Color32 {
    const HUES: [(u8, u8, u8); 8] = [
        (70, 120, 240),
        (130, 92, 230),
        (226, 84, 128),
        (235, 128, 52),
        (32, 160, 130),
        (40, 150, 210),
        (205, 78, 72),
        (98, 140, 60),
    ];
    let hash = name.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    let (r, g, b) = HUES[hash as usize % HUES.len()];
    Color32::from_rgb(r, g, b)
}

#[derive(Clone, Copy)]
pub enum NavIcon {
    Discover,
    Installed,
    Updates,
    Categories,
    Search,
    Back,
    Filters,
    Check,
    Appearance,
}

pub fn line_icon(ui: &mut egui::Ui, kind: NavIcon, size: f32) -> Response {
    let is_back = matches!(kind, NavIcon::Back);
    let clickable = matches!(kind, NavIcon::Back | NavIcon::Filters);
    let sense = if clickable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), sense);
    if clickable && response.hovered() {
        let fill = if response.is_pointer_button_down_on() {
            palette(ui.ctx()).nav_selected
        } else {
            palette(ui.ctx()).nav_hover
        };
        ui.painter().circle_filled(rect.center(), size * 0.5, fill);
    }
    draw_symbol(
        ui.painter(),
        rect.shrink(if is_back { 4.0 } else { 0.0 }),
        kind,
        ui.visuals().text_color(),
    );
    if clickable {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

fn draw_symbol(p: &egui::Painter, rect: Rect, kind: NavIcon, color: Color32) {
    let at = |x: f32, y: f32| rect.min + egui::vec2(x * rect.width(), y * rect.height());
    let stroke = Stroke::new((rect.width() / 17.0).clamp(1.1, 2.0), color);
    let line = |points: &[(f32, f32)]| {
        p.add(egui::Shape::line(
            points.iter().map(|&(x, y)| at(x, y)).collect(),
            stroke,
        ))
    };
    match kind {
        NavIcon::Discover => {
            let points = (0..11)
                .map(|i| {
                    let angle =
                        -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                    let r = if i % 2 == 0 { 0.44 } else { 0.19 };
                    at(0.5 + r * angle.cos(), 0.5 + r * angle.sin())
                })
                .collect();
            p.add(egui::Shape::line(points, stroke));
        }
        NavIcon::Categories => {
            for x in [0.12, 0.57] {
                for y in [0.12, 0.57] {
                    p.rect_stroke(
                        Rect::from_min_max(at(x, y), at(x + 0.31, y + 0.31)),
                        1,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        NavIcon::Updates => {
            // Seta circular com abertura no topo e ponta tangente ao arco.
            let (start, end) = (-0.4 * std::f32::consts::PI, 1.22 * std::f32::consts::PI);
            let radius = 0.36;
            let arc: Vec<_> = (0..=24)
                .map(|step| {
                    let angle = start + (end - start) * step as f32 / 24.0;
                    at(0.5 + radius * angle.cos(), 0.5 + radius * angle.sin())
                })
                .collect();
            p.add(egui::Shape::line(arc, stroke));
            let tip = egui::vec2(end.cos(), end.sin()) * radius + egui::vec2(0.5, 0.5);
            let forward = egui::vec2(-end.sin(), end.cos());
            let normal = egui::vec2(-forward.y, forward.x);
            let back = |side: f32| tip - forward * 0.2 + normal * side * 0.17;
            let (a, b) = (back(1.0), back(-1.0));
            line(&[(a.x, a.y), (tip.x, tip.y), (b.x, b.y)]);
        }
        NavIcon::Appearance => {
            // Círculo de contraste: metade esquerda preenchida.
            let center = at(0.5, 0.5);
            let radius = rect.width() * 0.4;
            p.circle_stroke(center, radius, stroke);
            let half: Vec<_> = (0..=16)
                .map(|step| {
                    let angle =
                        std::f32::consts::FRAC_PI_2 + step as f32 / 16.0 * std::f32::consts::PI;
                    center + radius * egui::vec2(angle.cos(), angle.sin())
                })
                .collect();
            p.add(egui::Shape::convex_polygon(half, color, Stroke::NONE));
        }
        NavIcon::Check => {
            line(&[(0.18, 0.52), (0.41, 0.74), (0.84, 0.28)]);
        }
        NavIcon::Installed => {
            line(&[(0.5, 0.1), (0.5, 0.64)]);
            line(&[(0.28, 0.44), (0.5, 0.66), (0.72, 0.44)]);
            line(&[(0.13, 0.48), (0.13, 0.87), (0.87, 0.87), (0.87, 0.48)]);
        }
        NavIcon::Search => {
            p.circle_stroke(at(0.41, 0.41), rect.width() * 0.28, stroke);
            line(&[(0.63, 0.63), (0.91, 0.91)]);
        }
        NavIcon::Filters => {
            for (y, knob) in [(0.22, 0.65), (0.5, 0.35), (0.78, 0.6)] {
                line(&[(0.1, y), (0.9, y)]);
                p.circle_filled(at(knob, y), rect.width() * 0.07, color);
            }
        }
        NavIcon::Back => {
            line(&[(0.66, 0.15), (0.3, 0.5), (0.66, 0.85)]);
        }
    }
}

fn pacman_shapes(rect: Rect, color: Color32, background: Color32) -> [egui::Shape; 2] {
    let radius = rect.width() * 0.5;
    let half_mouth = std::f32::consts::PI * 0.22;
    // Native circle and convex-path tessellation supply a one-pixel alpha fringe.
    let mouth_height = radius * half_mouth.tan();
    [
        egui::Shape::circle_filled(rect.center(), radius, color),
        egui::Shape::convex_polygon(
            vec![
                rect.center(),
                egui::pos2(rect.right(), rect.center().y - mouth_height),
                egui::pos2(rect.right(), rect.center().y + mouth_height),
            ],
            background,
            Stroke::NONE,
        ),
    ]
}

pub fn pacman_logo(ui: &mut egui::Ui, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let color = palette(ui.ctx()).text;
    ui.painter()
        .extend(pacman_shapes(rect, color, palette(ui.ctx()).sidebar));
    response
}

pub fn app_icon_static(ui: &mut egui::Ui, package: &Package, size: f32) -> Response {
    render_app_icon(ui, package, size, false)
}

fn icon_feedback(ui: &egui::Ui, response: Response, clickable: bool) -> Response {
    if clickable {
        if response.hovered() {
            ui.painter().rect_stroke(
                response.rect,
                response.rect.width() * 0.2,
                Stroke::new(1.5, palette(ui.ctx()).blue.gamma_multiply(0.45)),
                egui::StrokeKind::Inside,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

fn render_app_icon(ui: &mut egui::Ui, package: &Package, size: f32, clickable: bool) -> Response {
    let sense = if clickable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let dimensions = Vec2::splat(size);
    if let Some(path) = &package.icon_path {
        let image = egui::Image::new(format!("file://{path}"))
            .fit_to_exact_size(dimensions)
            .sense(sense);
        if image.load_for_size(ui.ctx(), dimensions).is_ok() {
            let response = ui.add(image);
            return icon_feedback(ui, response, clickable);
        }
    }
    let (rect, response) = ui.allocate_exact_size(dimensions, sense);
    monogram(ui.painter(), rect, package);
    icon_feedback(ui, response, clickable)
}

/// Ícone genérico: inicial do nome sobre um gradiente com a cor do pacote.
pub fn monogram(painter: &egui::Painter, rect: Rect, package: &Package) {
    let size = rect.width();
    let base = identity_color(&package.name);
    let top = base.lerp_to_gamma(Color32::WHITE, 0.22);
    let bottom = base.lerp_to_gamma(Color32::BLACK, 0.12);
    let shape = rect.shrink(size * 0.04);
    gradient_rect(painter, shape, size * 0.22, |x, y| {
        top.lerp_to_gamma(bottom, (y * 0.8 + x * 0.2).clamp(0.0, 1.0))
    });
    let letter: String = package
        .display_name
        .chars()
        .find(|c| c.is_alphanumeric())
        .unwrap_or('?')
        .to_uppercase()
        .collect();
    painter.text(
        shape.center() + egui::vec2(0.0, size * 0.01),
        egui::Align2::CENTER_CENTER,
        letter,
        bold_font(size * 0.44),
        Color32::WHITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hover_cursor(enabled: bool) -> egui::CursorIcon {
        let ctx = egui::Context::default();
        install(&ctx);
        let mut target = egui::Pos2::ZERO;
        let mut initial = ctx.run_ui(egui::RawInput::default(), |ui| {
            let response = pill(ui, "Instalar", enabled, false);
            target = response.rect.center();
        });
        initial.textures_delta.clear();
        let input = egui::RawInput {
            events: vec![egui::Event::PointerMoved(target)],
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let response = pill(ui, "Instalar", enabled, false);
            assert!(!response.clicked());
        });
        output.textures_delta.clear();
        output.platform_output.cursor_icon
    }

    fn luminance(color: Color32) -> f32 {
        let channels = [color.r(), color.g(), color.b()].map(|channel| {
            let value = f32::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
    }

    #[test]
    fn both_palettes_keep_text_readable_on_content_and_navigation() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let colors = theme_palette(theme);
            for background in [
                colors.background,
                colors.sidebar,
                colors.search,
                colors.nav_selected,
                colors.nav_hover,
                colors.row_hover,
                colors.card,
                colors.control,
                colors.accent_soft,
                colors.warning_soft,
                colors.error_soft,
            ] {
                let foreground = luminance(colors.text);
                let background = luminance(background);
                assert!(
                    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
                        >= 7.0
                );
            }
        }
    }

    #[test]
    fn logo_keeps_its_open_mouth_and_antialiased_edges_at_both_pixel_scales() {
        let ctx = egui::Context::default();
        let mut initial = ctx.run_ui(egui::RawInput::default(), |_| {});
        initial.textures_delta.clear();
        let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(28.0));
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let colors = theme_palette(theme);
            let gray = Color32::from_gray(if theme == egui::Theme::Dark { 205 } else { 128 });
            let contrast = (luminance(gray).max(luminance(colors.sidebar)) + 0.05)
                / (luminance(gray).min(luminance(colors.sidebar)) + 0.05);
            assert!(contrast > 3.0);
            for scale in [1.0, 2.0] {
                let shapes = pacman_shapes(rect, gray, colors.sidebar);
                let egui::Shape::Circle(circle) = &shapes[0] else {
                    panic!("logo body must be a circle")
                };
                assert_eq!(circle.fill, gray);
                let egui::Shape::Path(mouth) = &shapes[1] else {
                    panic!("mouth must be a convex path")
                };
                assert_eq!(mouth.fill, colors.sidebar);
                assert_eq!(mouth.points[0], rect.center());
                assert!(mouth.points[1].y < rect.center().y);
                assert!(mouth.points[2].y > rect.center().y);
                assert_eq!(mouth.points[1].x, rect.right());
                let primitives = ctx.tessellate(
                    shapes
                        .into_iter()
                        .map(|shape| egui::epaint::ClippedShape {
                            clip_rect: rect.expand(2.0),
                            shape,
                        })
                        .collect(),
                    scale,
                );
                let vertices: Vec<_> = primitives
                    .iter()
                    .flat_map(|primitive| {
                        let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
                            panic!("expected mesh")
                        };
                        assert!(mesh.is_valid());
                        &mesh.vertices
                    })
                    .collect();
                assert!(
                    vertices.iter().any(|vertex| vertex.color.a() == 0),
                    "alpha fringe missing at scale {scale}"
                );
                assert!(
                    vertices.iter().any(|vertex| vertex.color == gray),
                    "opaque logo fill missing"
                );
            }
        }
    }

    #[test]
    fn preferences_accept_saved_values_and_recover_from_invalid_input() {
        assert_eq!(parse_preference("dark\n"), egui::ThemePreference::Dark);
        assert_eq!(parse_preference("light"), egui::ThemePreference::Light);
        assert_eq!(parse_preference("system"), egui::ThemePreference::System);
        assert_eq!(parse_preference("invalid"), egui::ThemePreference::System);
    }

    #[test]
    fn logo_is_static_and_filter_control_offers_pointer_in_both_themes() {
        for theme in [egui::ThemePreference::Light, egui::ThemePreference::Dark] {
            for filter in [false, true] {
                let ctx = egui::Context::default();
                install(&ctx);
                set_theme(&ctx, theme, false).unwrap();
                let mut target = egui::Pos2::ZERO;
                let draw = |ui: &mut egui::Ui| {
                    if filter {
                        line_icon(ui, NavIcon::Filters, 24.0)
                    } else {
                        pacman_logo(ui, 32.0)
                    }
                };
                let mut initial = ctx.run_ui(egui::RawInput::default(), |ui| {
                    target = draw(ui).rect.center()
                });
                initial.textures_delta.clear();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events: vec![egui::Event::PointerMoved(target)],
                        ..Default::default()
                    },
                    |ui| {
                        let response = draw(ui);
                        assert_eq!(response.sense.senses_click(), filter);
                    },
                );
                output.textures_delta.clear();
                assert_eq!(
                    output.platform_output.cursor_icon == egui::CursorIcon::PointingHand,
                    filter
                );
            }
        }
    }

    fn click_at(ctx: &egui::Context, pos: egui::Pos2, mut draw: impl FnMut(&mut egui::Ui)) {
        for pressed in [true, false] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                &mut draw,
            );
            output.textures_delta.clear();
        }
    }

    #[test]
    fn toggle_flips_and_reports_change() {
        let ctx = egui::Context::default();
        install(&ctx);
        let mut on = false;
        let mut target = egui::Pos2::ZERO;
        let mut initial = ctx.run_ui(egui::RawInput::default(), |ui| {
            target = toggle(ui, &mut on, "AUR").rect.right_center() - egui::vec2(8.0, 0.0);
        });
        initial.textures_delta.clear();
        let mut changed = false;
        click_at(&ctx, target, |ui| {
            changed |= toggle(ui, &mut on, "AUR").changed()
        });
        assert!(on && changed);
    }

    #[test]
    fn menu_items_click_and_report_selection() {
        let ctx = egui::Context::default();
        install(&ctx);
        let mut target = egui::Pos2::ZERO;
        let mut initial = ctx.run_ui(egui::RawInput::default(), |ui| {
            target = menu_item(ui, "Escuro", true).rect.center();
        });
        initial.textures_delta.clear();
        let mut clicked = false;
        click_at(&ctx, target, |ui| {
            clicked |= menu_item(ui, "Escuro", true).clicked()
        });
        assert!(clicked);
    }

    #[test]
    fn identity_colors_are_stable_and_varied() {
        assert_eq!(identity_color("gimp"), identity_color("gimp"));
        let names = ["gimp", "vlc", "blender", "inkscape", "krita", "obs-studio"];
        let distinct: std::collections::HashSet<_> =
            names.iter().map(|name| identity_color(name)).collect();
        assert!(distinct.len() >= 3);
    }

    #[test]
    fn enabled_actions_offer_pointer_cursor() {
        assert_eq!(hover_cursor(true), egui::CursorIcon::PointingHand);
    }

    #[test]
    fn disabled_action_does_not_offer_pointer_cursor() {
        assert_ne!(hover_cursor(false), egui::CursorIcon::PointingHand);
    }
}
