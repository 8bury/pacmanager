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
        style.visuals.selection.bg_fill = colors.nav_selected;
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

fn bold_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("store-bold".into()))
}

pub fn heading(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).font(bold_font(size))
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
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
        let colors = palette(ui.ctx());
        let dark = ui.visuals().dark_mode;
        let fill = if !enabled {
            colors.search
        } else if primary {
            if dark {
                if response.is_pointer_button_down_on() {
                    Color32::from_rgb(25, 81, 155)
                } else if response.hovered() {
                    Color32::from_rgb(43, 115, 211)
                } else {
                    Color32::from_rgb(35, 104, 194)
                }
            } else if response.is_pointer_button_down_on() {
                Color32::from_rgb(0, 70, 164)
            } else if response.hovered() {
                Color32::from_rgb(0, 87, 193)
            } else {
                colors.blue
            }
        } else if response.is_pointer_button_down_on() {
            if dark {
                colors.nav_selected
            } else {
                Color32::from_rgb(210, 226, 248)
            }
        } else if response.hovered() {
            if dark {
                colors.nav_hover
            } else {
                Color32::from_rgb(226, 236, 251)
            }
        } else if dark {
            colors.search
        } else {
            Color32::from_rgb(240, 240, 246)
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 16, fill);
        let font = bold_font(12.0);
        let color = if !enabled {
            colors.secondary
        } else if primary {
            Color32::WHITE
        } else {
            colors.blue
        };
        let galley = painter.layout_no_wrap(label.into(), font, color);
        painter
            .with_clip_rect(rect.shrink2(egui::vec2(6.0, 0.0)))
            .galley(rect.center() - galley.size() * 0.5, galley, color);
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
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
                        Rect::from_min_max(at(x, y), at(x + 0.3, y + 0.3)),
                        2,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        NavIcon::Updates | NavIcon::Installed => {
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
    let color = if ui.visuals().dark_mode {
        Color32::from_gray(205)
    } else {
        Color32::from_gray(128)
    };
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
    ui.painter().rect_filled(
        rect.shrink(size * 0.025),
        size * 0.21,
        palette(ui.ctx()).search,
    );
    draw_symbol(
        ui.painter(),
        rect.shrink(size * 0.24),
        NavIcon::Categories,
        palette(ui.ctx()).blue,
    );
    icon_feedback(ui, response, clickable)
        .on_hover_text("Ícone genérico: o pacote não forneceu um ícone de aplicativo")
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

    #[test]
    fn enabled_actions_offer_pointer_cursor() {
        assert_eq!(hover_cursor(true), egui::CursorIcon::PointingHand);
    }

    #[test]
    fn disabled_action_does_not_offer_pointer_cursor() {
        assert_ne!(hover_cursor(false), egui::CursorIcon::PointingHand);
    }
}
