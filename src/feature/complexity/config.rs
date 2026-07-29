//! Optional smell.toml: named rules an invocation can select with --rule.

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

pub const FILE_NAME: &str = "smell.toml";
pub const DEFAULT_RULE: &str = "default";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, rename = "rule")]
    pub rules: Vec<RuleConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    #[serde(default = "default_rule_name")]
    pub name: String,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub branches: Vec<String>,
}

fn default_rule_name() -> String {
    DEFAULT_RULE.to_string()
}

impl Default for RuleConfig {
    fn default() -> RuleConfig {
        RuleConfig {
            name: default_rule_name(),
            include: Vec::new(),
            exclude: Vec::new(),
            branches: Vec::new(),
        }
    }
}

/// Reads `smell.toml` from `dir`. A missing file is not an error: `Ok(None)`
/// means "no config", which resolves to the built-in defaults.
pub fn load(dir: &Path) -> io::Result<Option<Config>> {
    let path = dir.join(FILE_NAME);
    match fs::read_to_string(&path) {
        Ok(text) => parse(&text, &path).map(Some),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Parses config text, reporting the file it came from. Separate from
/// [`load`] so parse and validation failures are testable without fixtures.
pub fn parse(text: &str, path: &Path) -> io::Result<Config> {
    let config: Config = toml::from_str(text).map_err(|error| invalid(path, &error.to_string()))?;
    validate(&config).map_err(|message| invalid(path, &message))?;
    Ok(config)
}

/// Catches rules that share a name (including two that both omit `name`),
/// which `deny_unknown_fields` can't: one definition would be silently dead,
/// and the file's meaning would depend on declaration order.
fn validate(config: &Config) -> Result<(), String> {
    let mut seen: Vec<&str> = Vec::new();
    for rule in &config.rules {
        if seen.contains(&rule.name.as_str()) {
            return Err(format!("duplicate rule name: {:?}", rule.name));
        }
        seen.push(&rule.name);
    }
    Ok(())
}

/// toml's `Display` is multi-line and newline-terminated, which would
/// otherwise leave a stray blank line after the cli's `error: {error}`.
fn invalid(path: &Path, message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{}: {}", path.display(), message.trim_end()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixture_path;

    fn path() -> std::path::PathBuf {
        std::path::PathBuf::from("smell.toml")
    }

    #[test]
    fn minimal_rule_defaults_name_and_empty_vecs() {
        let config = parse("[[rule]]\n", &path()).expect("valid config");
        assert_eq!(config.rules.len(), 1);
        let rule = &config.rules[0];
        assert_eq!(rule.name, DEFAULT_RULE);
        assert!(rule.include.is_empty());
        assert!(rule.exclude.is_empty());
        assert!(rule.branches.is_empty());
    }

    #[test]
    fn all_fields_parse() {
        let text = "[[rule]]\n\
                     name = \"swift\"\n\
                     include = [\"*.swift\"]\n\
                     exclude = [\"**/generated/**\"]\n\
                     branches = [\"switch\"]\n";
        let config = parse(text, &path()).expect("valid config");
        let rule = &config.rules[0];
        assert_eq!(rule.name, "swift");
        assert_eq!(rule.include, vec!["*.swift"]);
        assert_eq!(rule.exclude, vec!["**/generated/**"]);
        assert_eq!(rule.branches, vec!["switch"]);
    }

    #[test]
    fn unknown_field_on_rule_errors() {
        let error = parse("[[rule]]\nbogus = true\n", &path()).expect_err("unknown field");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("bogus"));
    }

    #[test]
    fn unknown_top_level_key_names_rule() {
        let error = parse("[[rules]]\n", &path()).expect_err("plural typo");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("rule"));
    }

    #[test]
    fn single_table_instead_of_array_errors() {
        let error = parse("[rule]\nname = \"default\"\n", &path()).expect_err("wrong shape");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn empty_text_has_no_rules_and_no_error() {
        let config = parse("", &path()).expect("empty config is valid");
        assert!(config.rules.is_empty());
    }

    #[test]
    fn duplicate_names_error() {
        let text = "[[rule]]\nname = \"a\"\n\n[[rule]]\nname = \"a\"\n";
        let error = parse(text, &path()).expect_err("duplicate name");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("duplicate rule name"));
    }

    #[test]
    fn two_unnamed_rules_collide_on_the_default_name() {
        let text = "[[rule]]\ninclude = [\"*.rs\"]\n\n[[rule]]\ninclude = [\"*.swift\"]\n";
        let error = parse(text, &path()).expect_err("both default to \"default\"");
        assert!(error.to_string().contains("duplicate rule name"));
    }

    #[test]
    fn load_reads_fixture_config() {
        let config = load(&fixture_path("config"))
            .expect("io succeeds")
            .expect("config exists");
        let names: Vec<&str> = config.rules.iter().map(|rule| rule.name.as_str()).collect();
        assert_eq!(names, vec![DEFAULT_RULE, "swift"]);
    }

    #[test]
    fn load_returns_none_when_missing() {
        let config = load(&fixture_path("java")).expect("io succeeds");
        assert!(config.is_none());
    }

    #[test]
    fn parse_error_names_the_path_with_no_trailing_newline() {
        let error = parse("[[rule]]\nbogus = true\n", &path()).expect_err("unknown field");
        let message = error.to_string();
        assert!(message.starts_with("smell.toml:"));
        assert!(!message.ends_with('\n'));
    }
}
