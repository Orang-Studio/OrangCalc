// colors for result tokens and a little CSS on top of the system GTK theme
use crate::engine::formatter::TokType;
use gtk::gdk;
use gtk::prelude::*;
use gtk::glib::translate::IntoGlib;
pub const DEFAULT_ACCENT: &str = "F08A3C";

thread_local! {
    static ACCENT: std::cell::Cell<(u8, u8, u8)> = const { std::cell::Cell::new((240, 138, 60)) };
}

pub fn accent_rgb() -> (u8, u8, u8) {
    ACCENT.with(|a| a.get())
}

// text drawn on top of the accent key/button: black on light accents, white on dark ones
pub fn accent_text_rgb() -> (u8, u8, u8) {
    let (r, g, b) = accent_rgb();
    let luma = (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64) / 255.0;
    if luma > 0.5 { (255, 255, 255) } else { (20, 20, 20) }
}

fn parse_accent(a: &str) -> (u8, u8, u8) {
    let h = |r: std::ops::Range<usize>| u8::from_str_radix(&a[r], 16).unwrap_or(0);
    (h(0..2), h(2..4), h(4..6))
}

fn hex(c: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

const APP_ICON_PNG: &[u8] = include_bytes!("../../orange.png");

pub fn app_icon() -> Option<gdk::Texture> {
    gdk::Texture::from_bytes(&gtk::glib::Bytes::from_static(APP_ICON_PNG)).ok()
}

pub fn token_color(t: TokType, dark: bool) -> String {
    if dark {
        match t {
            TokType::Number => "#eceef1".into(),
            TokType::Operator | TokType::Paren | TokType::Plain => "#969ea8".into(),
            TokType::Unit => hex(mix(accent_rgb(), (255, 255, 255), 0.30)),
            TokType::Variable => "#84bae8".into(),
            TokType::Function => "#c4a8e8".into(),
            TokType::Symbol => "#84ceac".into(),
            TokType::Text => "#d6c68c".into(),
            TokType::Error => "#ee6868".into(),
        }
    } else {
        match t {
            TokType::Number => "#1d1f23".into(),
            TokType::Operator | TokType::Paren | TokType::Plain => "#5c636c".into(),
            TokType::Unit => hex(mix(accent_rgb(), (0, 0, 0), 0.28)),
            TokType::Variable => "#1f6fb2".into(),
            TokType::Function => "#7a46b8".into(),
            TokType::Symbol => "#1f8a5a".into(),
            TokType::Text => "#8a6d12".into(),
            TokType::Error => "#c43c3c".into(),
        }
    }
}

thread_local! {
    static DARK: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

pub fn is_dark() -> bool {
    DARK.with(|d| d.get()) || gtk::Settings::default().and_then(|s| s.gtk_theme_name()).map(|n| n.to_lowercase().contains("dark")).unwrap_or(false)
}

pub const ELEMENT_COLORS: [&str; 12] = ["#7b8794", "#e0645c", "#e89a4c", "#c9b458", "#c78ac9", "#5c9ee0", "#6cb38a", "#5bb7b0", "#7fbf5a", "#5cc0d6", "#9c8ae0", "#8a8f99"];

fn css(accent: &str, dark: bool) -> String {
    let (err, comment) = if dark { ("#e06c6c", "#c9b458") } else { ("#c43c3c", "#8a6d12") };
    let at = hex(accent_text_rgb());
    let mut s = format!(
        "
@define-color oc_accent #{accent};
@define-color oc_accent_text {at};
@define-color theme_selected_bg_color_breeze #{accent};
@define-color theme_hovering_selected_bg_color_breeze #{accent};
@define-color link_color_breeze #{accent};
@define-color theme_button_decoration_focus_breeze #{accent};
@define-color theme_button_decoration_focus_backdrop_breeze #{accent};
@define-color theme_button_decoration_hover_breeze #{accent};
@define-color theme_button_decoration_hover_backdrop_breeze #{accent};
@define-color theme_view_active_decoration_color_breeze #{accent};
@define-color theme_view_hover_decoration_color_breeze #{accent};
@define-color theme_selected_bg_color #{accent};
@define-color accent_bg_color #{accent};
@define-color accent_color #{accent};
@define-color theme_unfocused_selected_bg_color_breeze alpha(#{accent}, 0.3);
@define-color theme_unfocused_selected_bg_color_alt_breeze alpha(#{accent}, 0.3);
@define-color theme_selected_fg_color_breeze {at};
row:selected, row:selected:hover, row:selected:focus, menubar > item:selected, menubar > item:hover, popover.menu modelbutton:hover, popover.menu modelbutton:selected, selection, text selection, entry selection {{ background-color: @oc_accent; color: @oc_accent_text; }}
check:checked, check:indeterminate, radio:checked {{ background-color: @oc_accent; border-color: @oc_accent; color: @oc_accent_text; }}
button:hover, button:focus, button:checked, entry:focus-within, spinbutton:focus-within, dropdown > button:focus, combobox button:focus, check:hover, radio:hover {{ border-color: @oc_accent; outline-color: @oc_accent; }}
link, link:link, link:visited, button.link label, label link {{ color: @oc_accent; }}
.oc-expr {{ font-size: 17pt; padding: 10px; }}
.oc-expr text {{ background: transparent; }}
.oc-result {{ font-size: 26pt; }}
.oc-result-small {{ font-size: 17pt; }}
.oc-parsed {{ opacity: 0.6; }}
.oc-status {{ opacity: 0.65; font-size: 10pt; }}
.oc-status.error {{ color: {err}; opacity: 1; }}
.oc-error {{ color: {err}; font-size: 15pt; }}
.oc-key {{ min-height: 30px; padding: 2px 4px; }}
.oc-key .oc-hint {{ font-size: 7.5pt; opacity: 0.6; }}
.oc-key .oc-label {{ font-size: 12pt; }}
.oc-key.accent {{ background-image: none; background-color: @oc_accent; color: @oc_accent_text; }}
.oc-key.accent .oc-hint {{ color: @oc_accent_text; }}
.oc-key.number .oc-label {{ font-weight: bold; }}
.oc-chip {{ min-height: 24px; padding: 0 8px; font-size: 9.5pt; }}
.oc-chip .oc-hint {{ font-size: 7.5pt; opacity: 0.6; }}
.oc-history-expr {{ opacity: 0.65; }}
.oc-history-result {{ font-size: 13pt; }}
.oc-history-comment {{ color: {comment}; font-size: 9pt; }}
.oc-protected {{ border-left: 3px solid @oc_accent; }}
.oc-mono {{ font-family: monospace; }}
.oc-bit {{ min-width: 12px; min-height: 20px; padding: 0 1px; font-family: monospace; font-size: 9pt; }}
.oc-bit:checked {{ background-image: none; background-color: alpha(@oc_accent, 0.45); }}
.oc-matrix {{ font-size: 14pt; }}
.oc-page-title {{ font-size: 15pt; font-weight: bold; }}
.oc-element {{ min-width: 46px; min-height: 46px; padding: 1px; background-image: none; color: #111; }}
.oc-element .oc-sym {{ font-weight: bold; font-size: 13pt; }}
.oc-element .oc-num {{ font-size: 7pt; }}
.oc-element.dim {{ opacity: 0.25; }}
.oc-nav row {{ padding: 6px 10px; }}
"
    );
    for (i, c) in ELEMENT_COLORS.iter().enumerate() {
        s += &format!(".oc-el{} {{ background-color: {}; }}\n", i, c);
    }
    s
}

thread_local! {
    static PROVIDER: gtk::CssProvider = gtk::CssProvider::new();
}

pub fn apply(accent: &str, dark: bool) {
    DARK.with(|d| d.set(dark));
    if let Some(s) = gtk::Settings::default() {
        // gtk 4.20 replaced the dark theme
        let scheme = s.find_property("gtk-interface-color-scheme").and_then(|p| gtk::glib::EnumClass::with_type(p.value_type())).and_then(|e| e.to_value_by_nick(if dark { "dark" } else { "default" }));
        if let Some(v) = scheme {
            s.set_property_from_value("gtk-interface-color-scheme", &v);
        } else {
            s.set_gtk_application_prefer_dark_theme(dark);
        }
    }
    let accent = if accent.len() == 6 && accent.chars().all(|c| c.is_ascii_hexdigit()) { accent } else { DEFAULT_ACCENT };
    ACCENT.with(|a| a.set(parse_accent(accent)));
    PROVIDER.with(|p| {
        p.load_from_string(&css(accent, dark));
        if let Some(d) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&d, p, gtk::STYLE_PROVIDER_PRIORITY_USER + 1);
        }
    });
}

pub fn markup_tokens(toks: &[crate::engine::formatter::OutTok], colorize: bool) -> String {
    let dark = is_dark();
    toks.iter()
        .map(|t| {
            let text = gtk::glib::markup_escape_text(&t.text);
            if colorize {
                format!("<span foreground=\"{}\">{}</span>", token_color(t.ty, dark), text)
            } else {
                text.to_string()
            }
        })
        .collect()
}

thread_local! {
    static FONTS: gtk::CssProvider = gtk::CssProvider::new();
}

fn font_css(selector: &str, spec: &str) -> String {
    if spec.is_empty() {
        return String::new();
    }
    let fd = gtk::pango::FontDescription::from_string(spec);
    let mut s = format!("{} {{", selector);
    if let Some(f) = fd.family() {
        s += &format!(" font-family: \"{}\";", f);
    }
    if fd.size() > 0 {
        s += &format!(" font-size: {}pt;", fd.size() as f64 / gtk::pango::SCALE as f64);
    }
    if fd.weight() != gtk::pango::Weight::Normal {
        s += &format!(" font-weight: {};", fd.weight().into_glib());
    }
    if fd.style() == gtk::pango::Style::Italic {
        s += " font-style: italic;";
    }
    s + " }\n"
}

// user font choices overrides defaults
pub fn apply_fonts(s: &crate::ui::settings::AppSettings) {
    let css = [
        font_css(".oc-expr", &s.expression_font),
        font_css(".oc-result, .oc-result-small, .oc-matrix", &s.result_font),
        font_css(".oc-history-result", &s.history_font),
        font_css(".oc-key .oc-label", &s.keypad_font),
        font_css(".oc-status", &s.status_font),
    ]
    .concat();
    FONTS.with(|p| {
        p.load_from_string(&css);
        if let Some(d) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&d, p, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
        }
    });
}