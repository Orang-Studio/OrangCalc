use crate::engine::formatter::{OutTok, TokType};
use crate::engine::*;
use crate::xml::Node;
use std::collections::BTreeMap;
use std::path::PathBuf;
macro_rules! ui_enum {
    ($name:ident { $($v:ident),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($v),* }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$v),*];
            pub fn index(self) -> usize { Self::ALL.iter().position(|x| *x == self).unwrap() }
            pub fn from_index(i: usize) -> $name { Self::ALL[i.min(Self::ALL.len() - 1)] }
            pub fn name(self) -> &'static str { match self { $($name::$v => stringify!($v)),* } }
            pub fn parse(s: &str) -> Option<$name> { Self::ALL.iter().copied().find(|x| x.name() == s) }
        }
    };
}
ui_enum!(AfterCalc { Keep, Clear, ReplaceWithResult, ReplaceIfShorter });
ui_enum!(TitleMode { AppName, AppNameAndResult, AppNameAndWorkspace, ResultOnly });
ui_enum!(CompletionMode { Off, Full, Strict, LimitedFull, LimitedStrict });
ui_enum!(KeypadMode { General, Programming, Algebra, Custom, NumberPadOnly, None });
ui_enum!(HistoryExprMode { Entered, Parsed, EnteredAndParsed });
pub trait Field {
    fn to_text(&self) -> String;
    fn from_text(&mut self, s: &str);
}

impl Field for bool {
    fn to_text(&self) -> String {
        if *self { "1" } else { "0" }.into()
    }
    fn from_text(&mut self, s: &str) {
        *self = s == "1" || s.eq_ignore_ascii_case("true");
    }
}
impl Field for i32 {
    fn to_text(&self) -> String {
        self.to_string()
    }
    fn from_text(&mut self, s: &str) {
        if let Ok(v) = s.parse() {
            *self = v;
        }
    }
}
impl Field for String {
    fn to_text(&self) -> String {
        self.clone()
    }
    fn from_text(&mut self, s: &str) {
        *self = s.to_string();
    }
}
impl Field for RoundingMode {
    fn to_text(&self) -> String {
        format!("{:?}", self)
    }
    fn from_text(&mut self, s: &str) {
        if let Some(v) = RoundingMode::ALL.iter().find(|x| format!("{:?}", x) == s) {
            *self = *v;
        }
    }
}

macro_rules! enum_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            fn to_text(&self) -> String { self.name().to_string() }
            fn from_text(&mut self, s: &str) { if let Some(v) = <$t>::parse(s) { *self = v; } }
        }
    )*};
}
enum_field!(ApproxMode, AngleMode, ParseMode, StructMode, NumberDisplay, FractionMode, ComplexForm, IntervalDisplay, IntervalCalc, DigitGrouping, RepeatingMode, UnitConversion, PrefixMode, TemperatureMode, AssumeType, AssumeSign, MultSign, DivSign, TwosComplement);
enum_field!(AfterCalc, TitleMode, CompletionMode, KeypadMode, HistoryExprMode);
macro_rules! fields {
    ($ty:ty, $tag:literal, { $($f:ident : $n:literal),* $(,)? }) => {
        impl Persist for $ty {
            fn write_fields(&self, parent: &mut Node) {
                $( parent.push(Node::new($tag).with("name", $n).with("value", self.$f.to_text())); )*
            }
            fn read_fields(&mut self, parent: &Node) {
                for e in parent.all($tag) {
                    let v = e.attr("value");
                    match e.attr("name") {
                        $( $n => self.$f.from_text(v), )*
                        _ => {}
                    }
                }
            }
        }
    };
}

pub trait Persist {
    fn write_fields(&self, parent: &mut Node);
    fn read_fields(&mut self, parent: &Node);
}

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub win_w: i32,
    pub win_h: i32,
    pub maximized: bool,
    pub side_width: i32,
    pub side_visible: bool,
    pub side_left: bool,
    pub side_tab: i32,
    pub keypad: KeypadMode,
    pub numpad_visible: bool,
    pub keypad_visible: bool,
    pub keypad_height: i32,
    pub always_on_top: bool,
    pub multiple_instances: bool,
    pub close_with_escape: bool,
    pub clear_history_on_exit: bool,
    pub max_history: i32,
    pub expression_after: AfterCalc,
    pub auto_copy_result: bool,
    pub copy_ascii_default: bool,
    pub copy_ascii_no_units: bool,
    pub colorize_result: bool,
    pub show_status: bool,
    pub calc_as_you_type: bool,
    pub as_you_type_delay: i32,
    pub adaptive_delay: bool,
    pub auto_calculate_selection: bool,
    pub completion: CompletionMode,
    pub completion_min_chars: i32,
    pub completion_delay: i32,
    pub title: TitleMode,
    pub history_expression: HistoryExprMode,
    pub disable_blinking: bool,
    pub rpn_keys: bool,
    pub rpn_mode: bool,
    pub rates_update_days: i32,
    pub last_rates_update: String,
    pub default_currency: String,
    pub app_font: String,
    pub expression_font: String,
    pub result_font: String,
    pub keypad_font: String,
    pub history_font: String,
    pub status_font: String,
    pub tooltips: bool,
    pub hide_units_in_constants: bool,
    pub current_page: i32,
    pub percent_in_numpad: bool,
    pub last_unit_button: String,
    pub confirm_quit: bool,
    pub ask_save_workspace: bool,
    pub accent_color: String,
    pub dark_theme: bool,
    pub workspace: String,

    pub favorite_functions: Vec<String>,
    pub favorite_units: Vec<String>,
    pub favorite_variables: Vec<String>,
    pub recent_functions: Vec<String>,
    pub recent_units: Vec<String>,
    pub recent_variables: Vec<String>,
    pub recent_workspaces: Vec<String>,
    pub custom_buttons: Vec<Vec<String>>,
    pub shortcuts: BTreeMap<String, String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let sv = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
        AppSettings {
            win_w: 1180,
            win_h: 760,
            maximized: false,
            side_width: 340,
            side_visible: true,
            side_left: false,
            side_tab: 0,
            keypad: KeypadMode::General,
            numpad_visible: true,
            keypad_visible: true,
            keypad_height: 250,
            always_on_top: false,
            multiple_instances: false,
            close_with_escape: false,
            clear_history_on_exit: false,
            max_history: 300,
            expression_after: AfterCalc::Keep,
            auto_copy_result: false,
            copy_ascii_default: false,
            copy_ascii_no_units: false,
            colorize_result: true,
            show_status: true,
            calc_as_you_type: true,
            as_you_type_delay: 250,
            adaptive_delay: true,
            auto_calculate_selection: false,
            completion: CompletionMode::Full,
            completion_min_chars: 2,
            completion_delay: 0,
            title: TitleMode::AppName,
            history_expression: HistoryExprMode::Entered,
            disable_blinking: false,
            rpn_keys: true,
            rpn_mode: false,
            rates_update_days: 7,
            last_rates_update: String::new(),
            default_currency: "EUR".into(),
            app_font: String::new(),
            expression_font: String::new(),
            result_font: String::new(),
            keypad_font: String::new(),
            history_font: String::new(),
            status_font: String::new(),
            tooltips: true,
            hide_units_in_constants: false,
            current_page: 0,
            percent_in_numpad: false,
            last_unit_button: "m".into(),
            confirm_quit: false,
            ask_save_workspace: false,
            accent_color: "F08A3C".into(),
            dark_theme: true,
            workspace: String::new(),
            favorite_functions: sv(&["sqrt", "sin", "ln", "solve", "integrate"]),
            favorite_units: sv(&["m", "kg", "s", "°C"]),
            favorite_variables: sv(&["pi", "e", "c"]),
            recent_functions: vec![],
            recent_units: vec![],
            recent_variables: vec![],
            recent_workspaces: vec![],
            custom_buttons: vec![],
            shortcuts: BTreeMap::new(),
        }
    }
}

fields!(AppSettings, "ui", {
    win_w: "WinW", win_h: "WinH", maximized: "Maximized", side_width: "SideWidth", side_visible: "SideVisible", side_left: "SideLeft",
    side_tab: "SideTab", keypad: "Keypad", numpad_visible: "NumpadVisible", keypad_visible: "KeypadVisible", keypad_height: "KeypadHeight",
    always_on_top: "AlwaysOnTop", multiple_instances: "MultipleInstances", close_with_escape: "CloseWithEscape",
    clear_history_on_exit: "ClearHistoryOnExit", max_history: "MaxHistory", expression_after: "ExpressionAfter",
    auto_copy_result: "AutoCopyResult", copy_ascii_default: "CopyAsciiDefault", copy_ascii_no_units: "CopyAsciiNoUnits",
    colorize_result: "ColorizeResult", show_status: "ShowStatus", calc_as_you_type: "CalcAsYouType", as_you_type_delay: "AsYouTypeDelay",
    adaptive_delay: "AdaptiveDelay", auto_calculate_selection: "AutoCalculateSelection", completion: "Completion",
    completion_min_chars: "CompletionMinChars", completion_delay: "CompletionDelay", title: "Title", history_expression: "HistoryExpression",
    disable_blinking: "DisableBlinking", rpn_keys: "RpnKeys", rpn_mode: "RpnMode", rates_update_days: "RatesUpdateDays",
    last_rates_update: "LastRatesUpdate", default_currency: "DefaultCurrency", app_font: "AppFont", expression_font: "ExpressionFont",
    result_font: "ResultFont", keypad_font: "KeypadFont", history_font: "HistoryFont", status_font: "StatusFont", tooltips: "Tooltips",
    hide_units_in_constants: "HideUnitsInConstants", current_page: "CurrentPage", percent_in_numpad: "PercentInNumpad",
    last_unit_button: "LastUnitButton", confirm_quit: "ConfirmQuit", ask_save_workspace: "AskSaveWorkspace", accent_color: "AccentColor",
    dark_theme: "DarkTheme", workspace: "Workspace"
});

fields!(CalcOptions, "calc", {
    approx: "Approx", angle: "Angle", parse: "Parse", structure: "Structure", units_enabled: "UnitsEnabled", variables_enabled: "VariablesEnabled",
    functions_enabled: "FunctionsEnabled", units_in_constants: "UnitsInConstants", unknown_as_variables: "UnknownAsVariables",
    complex_enabled: "ComplexEnabled", infinite_enabled: "InfiniteEnabled", real_roots: "RealRoots", read_precision: "ReadPrecision",
    limit_implicit_mult: "LimitImplicitMult", caret_xor: "CaretXor", simplified_percent: "SimplifiedPercent",
    concise_uncertainty_input: "ConciseUncertaintyInput", ignore_comma: "IgnoreComma", ignore_dots: "IgnoreDots", comma_decimal: "CommaDecimal",
    use_j: "UseJ", interval: "Interval", temperature: "Temperature", auto_convert: "AutoConvert", prefixes: "Prefixes",
    denominator_prefixes: "DenominatorPrefixes", all_si_prefixes: "AllSiPrefixes", currency_prefixes: "CurrencyPrefixes",
    binary_prefixes: "BinaryPrefixes", mixed_units: "MixedUnits", default_type: "DefaultType", default_sign: "DefaultSign",
    input_base: "InputBase", input_base_custom: "InputBaseCustom", twos_complement_input: "TwosComplementInput",
    hex_twos_complement_input: "HexTwosComplementInput", precision: "Precision", rounding: "Rounding", display: "Display",
    output_base: "OutputBase", output_base_custom: "OutputBaseCustom", min_decimals: "MinDecimals", max_decimals: "MaxDecimals",
    fraction: "Fraction", complex: "Complex", interval_out: "IntervalOut", grouping: "Grouping", repeating: "Repeating",
    e_notation: "ENotation", lower_case_e: "LowerCaseE", lower_case_hex: "LowerCaseHex", duodecimal_symbols: "DuodecimalSymbols",
    unicode: "Unicode", mult: "Mult", div: "Div", unicode_division: "UnicodeDivision", spell_out_logical: "SpellOutLogical",
    abbreviate_names: "AbbreviateNames", twos_out: "TwosOut", binary_bits: "BinaryBits", binary_letter_spacing: "BinaryLetterSpacing",
    implicit_mult_output: "ImplicitMultOutput", excessive_parentheses: "ExcessiveParentheses", show_ending_zeroes: "ShowEndingZeroes"
});

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub expression: String,
    pub parsed: String,
    pub result: String,
    pub result_ascii: String,
    pub exact_text: String,
    pub tokens: Vec<OutTok>,
    pub approx: bool,
    pub is_error: bool,
    pub is_message: bool,
    pub protected: bool,
    pub comment: String,
    pub time: chrono::NaiveDateTime,
}

impl Default for HistoryEntry {
    fn default() -> Self {
        HistoryEntry {
            expression: String::new(),
            parsed: String::new(),
            result: String::new(),
            result_ascii: String::new(),
            exact_text: String::new(),
            tokens: vec![],
            approx: false,
            is_error: false,
            is_message: false,
            protected: false,
            comment: String::new(),
            time: chrono::Local::now().naive_local(),
        }
    }
}

const TOK_TYPES: [TokType; 10] = [TokType::Plain, TokType::Number, TokType::Operator, TokType::Variable, TokType::Function, TokType::Unit, TokType::Symbol, TokType::Paren, TokType::Text, TokType::Error];

pub fn dir() -> PathBuf {
    let d = gtk::glib::user_config_dir().join("OrangStudio").join("OrangCalc");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn settings_file() -> PathBuf {
    dir().join("settings.xml")
}
pub fn history_file() -> PathBuf {
    dir().join("history.xml")
}
pub fn definitions_file() -> PathBuf {
    dir().join("definitions.xml")
}
pub fn rates_file() -> PathBuf {
    dir().join("rates.xml")
}

fn write_list(parent: &mut Node, name: &str, items: impl IntoIterator<Item = String>) {
    let mut e = Node::new("list").with("name", name);
    for i in items {
        let mut it = Node::new("item");
        it.text = i;
        e.push(it);
    }
    parent.push(e);
}

fn read_list(parent: &Node, name: &str) -> Option<Vec<String>> {
    parent.all("list").find(|e| e.attr("name") == name).map(|e| e.all("item").map(|x| x.text.clone()).collect())
}

pub fn write_settings(root: &mut Node, s: Option<&AppSettings>, o: Option<&CalcOptions>) {
    if let Some(s) = s {
        s.write_fields(root);
        write_list(root, "FavoriteFunctions", s.favorite_functions.clone());
        write_list(root, "FavoriteUnits", s.favorite_units.clone());
        write_list(root, "FavoriteVariables", s.favorite_variables.clone());
        write_list(root, "RecentFunctions", s.recent_functions.clone());
        write_list(root, "RecentUnits", s.recent_units.clone());
        write_list(root, "RecentVariables", s.recent_variables.clone());
        write_list(root, "RecentWorkspaces", s.recent_workspaces.clone());
        write_list(root, "CustomButtons", s.custom_buttons.iter().map(|b| b.join("\u{1f}")));
        write_list(root, "Shortcuts", s.shortcuts.iter().map(|(k, v)| format!("{}\u{1f}{}", k, v)));
    }
    if let Some(o) = o {
        o.write_fields(root);
    }
}

pub fn read_settings(root: &Node, s: Option<&mut AppSettings>, o: Option<&mut CalcOptions>) {
    if let Some(s) = s {
        s.read_fields(root);
        let take = |name: &str, dst: &mut Vec<String>| {
            if let Some(l) = read_list(root, name) {
                *dst = l;
            }
        };
        take("FavoriteFunctions", &mut s.favorite_functions);
        take("FavoriteUnits", &mut s.favorite_units);
        take("FavoriteVariables", &mut s.favorite_variables);
        take("RecentFunctions", &mut s.recent_functions);
        take("RecentUnits", &mut s.recent_units);
        take("RecentVariables", &mut s.recent_variables);
        take("RecentWorkspaces", &mut s.recent_workspaces);
        if let Some(cb) = read_list(root, "CustomButtons") {
            s.custom_buttons = cb.iter().map(|x| x.split('\u{1f}').map(|p| p.to_string()).collect::<Vec<_>>()).filter(|a| a.len() >= 2).collect();
        }
        if let Some(sc) = read_list(root, "Shortcuts") {
            s.shortcuts = sc
                .iter()
                .filter_map(|x| {
                    let p: Vec<&str> = x.split('\u{1f}').collect();
                    (p.len() == 2).then(|| (p[0].to_string(), p[1].to_string()))
                })
                .collect();
        }
    }
    if let Some(o) = o {
        o.read_fields(root);
    }
}

fn save_doc(path: PathBuf, root: &Node) {
    let _ = std::fs::write(path, root.to_string_doc());
}

fn load_doc(path: PathBuf) -> Option<Node> {
    let text = std::fs::read_to_string(path).ok()?;
    Node::parse(&text).ok()
}

pub fn save_settings(s: &AppSettings, o: &CalcOptions) {
    let mut root = Node::new("orangcalc");
    write_settings(&mut root, Some(s), Some(o));
    save_doc(settings_file(), &root);
}

pub fn load_settings(s: &mut AppSettings, o: &mut CalcOptions) {
    if let Some(root) = load_doc(settings_file()) {
        read_settings(&root, Some(s), Some(o));
    }
}

pub fn write_history<'a>(parent: &mut Node, entries: impl IntoIterator<Item = &'a HistoryEntry>) {
    let b = |v: bool| if v { "1" } else { "0" };
    for h in entries {
        let mut e = Node::new("entry")
            .with("expression", &h.expression)
            .with("parsed", &h.parsed)
            .with("result", &h.result)
            .with("ascii", &h.result_ascii)
            .with("exact", &h.exact_text)
            .with("approx", b(h.approx))
            .with("error", b(h.is_error))
            .with("message", b(h.is_message))
            .with("protected", b(h.protected))
            .with("comment", &h.comment)
            .with("time", h.time.format("%Y-%m-%dT%H:%M:%S%.f").to_string());
        for t in &h.tokens {
            let mut te = Node::new("t").with("k", (TOK_TYPES.iter().position(|x| *x == t.ty).unwrap_or(0)).to_string());
            te.text = t.text.clone();
            e.push(te);
        }
        parent.push(e);
}   }
pub fn read_history(parent: &Node) -> Vec<HistoryEntry> {
    parent
        .all("entry")
        .map(|e| {
            let time = e.attr("time");
            let parsed_time = chrono::NaiveDateTime::parse_from_str(&time[..time.len().min(26)], "%Y-%m-%dT%H:%M:%S%.f").ok();
            HistoryEntry {
                expression: e.attr("expression").into(),
                parsed: e.attr("parsed").into(),
                result: e.attr("result").into(),
                result_ascii: e.attr("ascii").into(),
                exact_text: e.attr("exact").into(),
                approx: e.attr("approx") == "1",
                is_error: e.attr("error") == "1",
                is_message: e.attr("message") == "1",
                protected: e.attr("protected") == "1",
                comment: e.attr("comment").into(),
                time: parsed_time.unwrap_or_else(|| chrono::Local::now().naive_local()),
                tokens: e.all("t").map(|te| OutTok { text: te.text.clone(), ty: TOK_TYPES[te.attr("k").parse::<usize>().unwrap_or(0).min(9)] }).collect(),
            }
        })
        .collect()
}

pub fn save_history<'a>(entries: impl IntoIterator<Item = &'a HistoryEntry>) {
    let mut root = Node::new("history");
    write_history(&mut root, entries);
    save_doc(history_file(), &root);
}
pub fn load_history() -> Vec<HistoryEntry> {
    load_doc(history_file()).map(|r| read_history(&r)).unwrap_or_default()
}
pub fn save_definitions(c: &Calculator) {
    let mut root = Node::new("definitions");
    useritems::save(c, &mut root);
    save_doc(definitions_file(), &root);
}

pub fn load_definitions(c: &Calculator) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(definitions_file()) else { return vec![] };
    match Node::parse(&text) {
        Ok(root) => useritems::load(c, &root),
        Err(e) => vec![e],
    }
}