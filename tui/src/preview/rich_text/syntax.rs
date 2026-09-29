//! Syntax highlighting using tree-sitter (primary) and syntect (fallback).

use ratatui::style::{Color, Modifier, Style};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock, RwLock};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use tree_sitter_highlight::{HighlightConfiguration, Highlighter};

static HIGHLIGHTER: OnceLock<RwLock<Highlighter>> = OnceLock::new();
static CONFIGS: OnceLock<BTreeMap<String, Arc<HighlightConfiguration>>> = OnceLock::new();
static REGISTRY: OnceLock<SyntaxRegistry> = OnceLock::new();
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

const LANGUAGE_KEYS: [&str; 13] = [
    "rust",
    "python",
    "javascript",
    "typescript",
    "json",
    "toml",
    "yaml",
    "html",
    "css",
    "c",
    "cpp",
    "go",
    "csharp",
];

pub fn get_highlighter() -> &'static RwLock<Highlighter> {
    HIGHLIGHTER.get_or_init(|| RwLock::new(Highlighter::new()))
}

// Compile only the requested grammar's highlight query. Each slot is initialized
// once, including failed configurations (which continue to use the fallback).
static LANGUAGE_CONFIGS: [OnceLock<Option<Arc<HighlightConfiguration>>>; LANGUAGE_KEYS.len()] =
    [const { OnceLock::new() }; LANGUAGE_KEYS.len()];

pub fn get_config(name: &str) -> Option<Arc<HighlightConfiguration>> {
    let index = LANGUAGE_KEYS.iter().position(|key| *key == name)?;
    LANGUAGE_CONFIGS[index]
        .get_or_init(|| {
            let query = match name {
                "rust" => tree_sitter_rust::HIGHLIGHTS_QUERY,
                "python" => tree_sitter_python::HIGHLIGHTS_QUERY,
                "javascript" => tree_sitter_javascript::HIGHLIGHT_QUERY,
                "typescript" => tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "json" => tree_sitter_json::HIGHLIGHTS_QUERY,
                "toml" => tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
                "yaml" => tree_sitter_yaml::HIGHLIGHTS_QUERY,
                "html" => tree_sitter_html::HIGHLIGHTS_QUERY,
                "css" => tree_sitter_css::HIGHLIGHTS_QUERY,
                "c" => tree_sitter_c::HIGHLIGHT_QUERY,
                "cpp" => tree_sitter_cpp::HIGHLIGHT_QUERY,
                "go" => tree_sitter_go::HIGHLIGHTS_QUERY,
                "csharp" => include_str!("../../../queries/csharp-highlights.scm"),
                _ => return None,
            };
            let mut config =
                HighlightConfiguration::new(language_from_key(name)?, "utf-8", query, "", "")
                    .ok()?;
            config.configure(&HIGHLIGHT_NAMES);
            Some(Arc::new(config))
        })
        .clone()
}

/// Return all configurations for callers that explicitly need the full registry.
pub fn get_configs() -> &'static BTreeMap<String, Arc<HighlightConfiguration>> {
    CONFIGS.get_or_init(|| {
        LANGUAGE_KEYS
            .iter()
            .filter_map(|name| get_config(name).map(|config| ((*name).to_string(), config)))
            .collect()
    })
}

pub const HIGHLIGHT_NAMES: [&str; 25] = [
    "attribute",
    "constant",
    "function.builtin",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "comment",
    "constructor",
    "label",
    "namespace",
    "number",
    "escape",
    "embedded",
];

pub fn get_style(highlight_idx: usize) -> Style {
    let name = HIGHLIGHT_NAMES.get(highlight_idx).unwrap_or(&"");
    style_for_capture(name)
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "attribute" => Style::default().fg(Color::Cyan),
        "constant" => Style::default().fg(Color::Red),
        "function.builtin" => Style::default().fg(Color::LightBlue),
        "function" => Style::default().fg(Color::Blue),
        "keyword" => Style::default().fg(Color::Magenta),
        "operator" => Style::default().fg(Color::White),
        "property" => Style::default().fg(Color::LightCyan),
        "punctuation" => Style::default().fg(Color::DarkGray),
        "punctuation.bracket" => Style::default().fg(Color::DarkGray),
        "punctuation.delimiter" => Style::default().fg(Color::DarkGray),
        "string" => Style::default().fg(Color::Green),
        "string.special" => Style::default().fg(Color::Green),
        "tag" => Style::default().fg(Color::LightRed),
        "type" => Style::default().fg(Color::Yellow),
        "type.builtin" => Style::default().fg(Color::Yellow),
        "variable" => Style::default().fg(Color::White),
        "variable.builtin" => Style::default().fg(Color::Red),
        "variable.parameter" => Style::default().fg(Color::LightRed),
        "comment" => Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
        "constructor" => Style::default().fg(Color::Yellow),
        "label" => Style::default().fg(Color::LightGreen),
        "namespace" => Style::default().fg(Color::Yellow),
        "number" => Style::default().fg(Color::Red),
        "escape" => Style::default().fg(Color::Magenta),
        "embedded" => Style::default(),
        _ => Style::default(),
    }
}

pub fn detect_language(path: &std::path::Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?;
    detect_language_from_extension(ext)
}

fn detect_language_from_extension(ext: &str) -> Option<&'static str> {
    if ext.eq_ignore_ascii_case("rs") {
        return Some("rust");
    }
    if ext.eq_ignore_ascii_case("py") {
        return Some("python");
    }
    if ext.eq_ignore_ascii_case("js") || ext.eq_ignore_ascii_case("jsx") {
        return Some("javascript");
    }
    if ext.eq_ignore_ascii_case("ts") || ext.eq_ignore_ascii_case("tsx") {
        return Some("typescript");
    }
    if ext.eq_ignore_ascii_case("json") {
        return Some("json");
    }
    if ext.eq_ignore_ascii_case("toml") {
        return Some("toml");
    }
    if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
        return Some("yaml");
    }
    if ext.eq_ignore_ascii_case("html") {
        return Some("html");
    }
    if ext.eq_ignore_ascii_case("css") {
        return Some("css");
    }
    if ext.eq_ignore_ascii_case("c") || ext.eq_ignore_ascii_case("h") {
        return Some("c");
    }
    if ext.eq_ignore_ascii_case("cpp")
        || ext.eq_ignore_ascii_case("cc")
        || ext.eq_ignore_ascii_case("cxx")
        || ext.eq_ignore_ascii_case("hpp")
    {
        return Some("cpp");
    }
    if ext.eq_ignore_ascii_case("go") {
        return Some("go");
    }
    if ext.eq_ignore_ascii_case("cs") {
        return Some("csharp");
    }
    None
}

pub struct SyntaxRegistry {
    languages: HashMap<&'static str, tree_sitter::Language>,
}

impl SyntaxRegistry {
    pub fn instance() -> &'static SyntaxRegistry {
        REGISTRY.get_or_init(Self::new)
    }

    fn new() -> Self {
        let mut languages = HashMap::new();
        for lang in LANGUAGE_KEYS {
            if let Some(language) = language_from_key(lang) {
                languages.insert(lang, language);
            }
        }

        Self { languages }
    }

    pub fn get_language(&self, lang_name: &str) -> Option<tree_sitter::Language> {
        self.languages.get(lang_name).cloned()
    }
}

pub fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

pub fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

fn language_from_key(name: &str) -> Option<tree_sitter::Language> {
    match name {
        "rust" => Some(tree_sitter::Language::from(tree_sitter_rust::LANGUAGE)),
        "python" => Some(tree_sitter::Language::from(tree_sitter_python::LANGUAGE)),
        "javascript" => Some(tree_sitter::Language::from(
            tree_sitter_javascript::LANGUAGE,
        )),
        "typescript" => Some(tree_sitter::Language::from(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        )),
        "json" => Some(tree_sitter::Language::from(tree_sitter_json::LANGUAGE)),
        "toml" => Some(tree_sitter::Language::from(tree_sitter_toml_ng::LANGUAGE)),
        "yaml" => Some(tree_sitter::Language::from(tree_sitter_yaml::LANGUAGE)),
        "html" => Some(tree_sitter::Language::from(tree_sitter_html::LANGUAGE)),
        "css" => Some(tree_sitter::Language::from(tree_sitter_css::LANGUAGE)),
        "c" => Some(tree_sitter::Language::from(tree_sitter_c::LANGUAGE)),
        "cpp" => Some(tree_sitter::Language::from(tree_sitter_cpp::LANGUAGE)),
        "go" => Some(tree_sitter::Language::from(tree_sitter_go::LANGUAGE)),
        "csharp" => Some(tree_sitter::Language::from(tree_sitter_c_sharp::LANGUAGE)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_keeps_its_configuration_and_cached_identity() {
        for name in LANGUAGE_KEYS {
            let config = get_config(name).expect(name);
            assert!(Arc::ptr_eq(&config, &get_config(name).unwrap()));
            assert!(Arc::ptr_eq(&config, &get_configs()[name]));
        }
        assert!(get_config("unsupported").is_none());
    }
}
