//! The `config` topic: `smell.toml` rule declarations and flag precedence.

const CONFIG_EXAMPLE: &str = "\
[[rule]]
name = \"default\"
include = [\"*.rs\"]
exclude = [\"**/generated/**\"]
branches = [\"switch\", \"boolean-operator\"]
implements = [\"Labeled\"]
max_complexity = 10
max_methods = 8
max_lines = 300
max_declarations = 20
";

pub fn render() -> String {
    format!(
        "CONFIG FILE\n\
         An optional smell.toml in the directory smell is invoked from (not\n\
         necessarily the analyzed path) declares named [[rule]] entries.\n\
         --rule <NAME> selects one; without it, the rule named \"default\" is\n\
         used if present, else the built-in defaults (a config file's mere\n\
         presence does not change a bare `smell <path>` invocation). Explicit\n\
         --include/--exclude/--branches/--implements/--max-complexity/\n\
         --max-methods/--max-lines/--max-declarations flags replace a\n\
         rule's value for that field entirely rather than merging with\n\
         it.\n\n{CONFIG_EXAMPLE}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::branch::BranchKind;
    use crate::feature::complexity::config::Config;

    #[test]
    fn config_example_deserializes() {
        let config: Config = toml::from_str(CONFIG_EXAMPLE).expect("example is valid config");
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "default");
    }

    #[test]
    fn config_example_cites_real_branch_kinds() {
        let config: Config = toml::from_str(CONFIG_EXAMPLE).expect("example is valid config");
        for branch in &config.rules[0].branches {
            assert!(
                BranchKind::from_name(branch).is_some(),
                "not a branch kind: {branch}"
            );
        }
    }

    #[test]
    fn page_includes_the_config_example() {
        assert!(render().contains(CONFIG_EXAMPLE));
    }
}
