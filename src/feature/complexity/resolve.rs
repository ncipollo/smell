//! Resolves CLI flags against smell.toml into the options an analysis runs with.

use std::io;
use std::path::Path;

use crate::code::branch::{BranchFilter, BranchSpec};
use crate::feature::complexity::config::{self, Config, DEFAULT_RULE, RuleConfig};
use crate::feature::complexity::filter::{FileFilter, TypeFilter};
use crate::feature::complexity::options::AnalysisOptions;

/// Raw flag values, before any config merging or compilation.
#[derive(Default)]
pub struct Overrides {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub branches: Vec<String>,
    pub implements: Vec<String>,
    pub rule: Option<String>,
}

/// Loads `smell.toml` from `config_dir` (the invocation directory, not the
/// analyzed path) and merges the selected rule with the given flags.
pub fn resolve(config_dir: &Path, overrides: &Overrides) -> io::Result<AnalysisOptions> {
    let config = config::load(config_dir)?;
    merge(config, overrides, config_dir)
}

/// Split from [`resolve`] so the precedence matrix is testable without fixtures.
fn merge(config: Option<Config>, overrides: &Overrides, dir: &Path) -> io::Result<AnalysisOptions> {
    let rule = select_rule(config, overrides.rule.as_deref(), dir)?;
    let specs: Vec<BranchSpec> = selected(&overrides.branches, &rule.branches)
        .iter()
        .map(|branch| BranchSpec::parse(branch))
        .collect();
    Ok(AnalysisOptions {
        files: FileFilter::new(
            selected(&overrides.include, &rule.include),
            selected(&overrides.exclude, &rule.exclude),
        )?,
        types: TypeFilter::new(selected(&overrides.implements, &rule.implements)),
        branches: BranchFilter::from_specs(&specs),
    })
}

/// A non-empty flag replaces the rule's value for that field outright: no
/// concatenation, so a flag can widen a rule's filter as well as narrow it.
fn selected<'a>(flag: &'a [String], rule: &'a [String]) -> &'a [String] {
    if flag.is_empty() { rule } else { flag }
}

fn select_rule(
    config: Option<Config>,
    requested: Option<&str>,
    dir: &Path,
) -> io::Result<RuleConfig> {
    match (config, requested) {
        (None, None) => Ok(RuleConfig::default()),
        (None, Some(name)) => Err(no_config_error(dir, name)),
        (Some(config), None) => Ok(take_rule(config, DEFAULT_RULE).unwrap_or_default()),
        (Some(config), Some(name)) => {
            let available = rule_names(&config);
            take_rule(config, name).ok_or_else(|| unknown_rule_error(name, &available))
        }
    }
}

fn take_rule(config: Config, name: &str) -> Option<RuleConfig> {
    config.rules.into_iter().find(|rule| rule.name == name)
}

fn rule_names(config: &Config) -> Vec<String> {
    config.rules.iter().map(|rule| rule.name.clone()).collect()
}

fn no_config_error(dir: &Path, name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "--rule {name:?} was given but no {} exists in {}",
            config::FILE_NAME,
            dir.display()
        ),
    )
}

fn unknown_rule_error(name: &str, available: &[String]) -> io::Error {
    let detail = if available.is_empty() {
        format!("{} defines no rules", config::FILE_NAME)
    } else {
        format!("available rules: {}", available.join(", "))
    };
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("unknown rule {name:?}: {detail}"),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::branch::BranchKind;
    use crate::testing::fixture_path;

    fn dir() -> PathBuf {
        PathBuf::from(".")
    }

    fn config_from(text: &str) -> Config {
        config::parse(text, &PathBuf::from("smell.toml")).expect("valid config")
    }

    /// `AnalysisOptions` has no `Debug`, so `expect_err` won't compile.
    fn err(result: io::Result<AnalysisOptions>) -> io::Error {
        match result {
            Ok(_) => panic!("expected an error"),
            Err(error) => error,
        }
    }

    #[test]
    fn no_config_no_flags_counts_everything() {
        let options = merge(None, &Overrides::default(), &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("notes.md")));
        assert!(options.types.matches(&[]));
        for kind in BranchKind::ALL {
            assert!(options.branches.allows_kind(kind));
        }
    }

    #[test]
    fn no_config_flags_apply() {
        let overrides = Overrides {
            include: vec!["*.rs".to_string()],
            ..Overrides::default()
        };
        let options = merge(None, &overrides, &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("main.rs")));
        assert!(!options.files.matches(Path::new("main.swift")));
    }

    #[test]
    fn no_config_with_rule_errors_naming_the_rule_and_no_config() {
        let overrides = Overrides {
            rule: Some("x".to_string()),
            ..Overrides::default()
        };
        let error = err(merge(None, &overrides, &dir()));
        let message = error.to_string();
        assert!(message.contains("\"x\""));
        assert!(message.contains(config::FILE_NAME));
        assert!(!message.contains("available rules"));
    }

    #[test]
    fn config_with_default_rule_is_auto_selected() {
        let config = config_from("[[rule]]\ninclude = [\"*.rs\"]\n");
        let options = merge(Some(config), &Overrides::default(), &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("main.rs")));
        assert!(!options.files.matches(Path::new("main.swift")));
    }

    #[test]
    fn config_without_default_rule_falls_back_with_no_error() {
        let config = config_from("[[rule]]\nname = \"swift\"\ninclude = [\"*.swift\"]\n");
        let options = merge(Some(config), &Overrides::default(), &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("notes.md")));
    }

    #[test]
    fn config_with_named_rule_selected_by_flag() {
        let config = config_from(
            "[[rule]]\ninclude = [\"*.rs\"]\n\n[[rule]]\nname = \"swift\"\ninclude = [\"*.swift\"]\nbranches = [\"switch\"]\n",
        );
        let overrides = Overrides {
            rule: Some("swift".to_string()),
            ..Overrides::default()
        };
        let options = merge(Some(config), &overrides, &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("main.swift")));
        assert!(!options.files.matches(Path::new("main.rs")));
        assert!(options.branches.allows_kind(BranchKind::Switch));
        assert!(!options.branches.allows_kind(BranchKind::If));
    }

    #[test]
    fn missing_rule_target_errors_listing_available_names() {
        let config = config_from(
            "[[rule]]\ninclude = [\"*.rs\"]\n\n[[rule]]\nname = \"swift\"\ninclude = [\"*.swift\"]\n",
        );
        let overrides = Overrides {
            rule: Some("missing".to_string()),
            ..Overrides::default()
        };
        let error = err(merge(Some(config), &overrides, &dir()));
        let message = error.to_string();
        assert!(message.contains("default"));
        assert!(message.contains("swift"));
    }

    #[test]
    fn zero_rules_with_requested_rule_says_no_rules_defined() {
        let config = config_from("");
        let overrides = Overrides {
            rule: Some("x".to_string()),
            ..Overrides::default()
        };
        let error = err(merge(Some(config), &overrides, &dir()));
        assert!(error.to_string().contains("defines no rules"));
    }

    #[test]
    fn flag_include_replaces_rule_include() {
        let config = config_from("[[rule]]\ninclude = [\"*.rs\"]\n");
        let overrides = Overrides {
            include: vec!["*.swift".to_string()],
            ..Overrides::default()
        };
        let options = merge(Some(config), &overrides, &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("main.swift")));
        assert!(!options.files.matches(Path::new("main.rs")));
    }

    #[test]
    fn flag_exclude_replaces_rule_exclude() {
        let config = config_from("[[rule]]\nexclude = [\"**/generated/**\"]\n");
        let overrides = Overrides {
            exclude: vec!["**/vendor/**".to_string()],
            ..Overrides::default()
        };
        let options = merge(Some(config), &overrides, &dir()).expect("resolves");
        assert!(options.files.matches(Path::new("src/generated/api.rs")));
        assert!(!options.files.matches(Path::new("src/vendor/api.rs")));
    }

    #[test]
    fn flag_branches_replaces_rule_branches() {
        let config = config_from("[[rule]]\nbranches = [\"switch\"]\n");
        let overrides = Overrides {
            branches: vec!["if".to_string()],
            ..Overrides::default()
        };
        let options = merge(Some(config), &overrides, &dir()).expect("resolves");
        assert!(options.branches.allows_kind(BranchKind::If));
        assert!(!options.branches.allows_kind(BranchKind::Switch));
    }

    #[test]
    fn rule_implements_applies() {
        let config = config_from("[[rule]]\nimplements = [\"Labeled\"]\n");
        let options = merge(Some(config), &Overrides::default(), &dir()).expect("resolves");
        assert!(options.types.matches(&["Labeled".to_string()]));
        assert!(!options.types.matches(&["Other".to_string()]));
    }

    #[test]
    fn flag_implements_replaces_rule_implements() {
        let config = config_from("[[rule]]\nimplements = [\"Labeled\"]\n");
        let overrides = Overrides {
            implements: vec!["Describe".to_string()],
            ..Overrides::default()
        };
        let options = merge(Some(config), &overrides, &dir()).expect("resolves");
        assert!(options.types.matches(&["Describe".to_string()]));
        assert!(!options.types.matches(&["Labeled".to_string()]));
    }

    #[test]
    fn invalid_glob_in_rule_errors() {
        let config = config_from("[[rule]]\ninclude = [\"[\"]\n");
        let error = err(merge(Some(config), &Overrides::default(), &dir()));
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn resolve_reads_config_from_disk() {
        let overrides = Overrides {
            rule: Some("swift".to_string()),
            ..Overrides::default()
        };
        let options = resolve(&fixture_path("config"), &overrides).expect("resolves");
        assert!(options.files.matches(Path::new("main.swift")));
        assert!(!options.files.matches(Path::new("main.rs")));
        assert!(options.branches.allows_kind(BranchKind::Switch));
    }
}
