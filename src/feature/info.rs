//! Topic registry for `--info`, so AI agents can walk documentation one
//! topic at a time instead of reading a single giant page.

mod branches;
mod checks;
mod config;
mod filters;
mod languages;
mod usage;

pub struct Topic {
    pub name: &'static str,
    pub summary: &'static str,
    render: fn() -> String,
}

pub const TOPICS: &[Topic] = &[
    Topic {
        name: "usage",
        summary: "Invoking smell, stdin/diff piping, output modes, exit codes",
        render: usage::render,
    },
    Topic {
        name: "config",
        summary: "smell.toml rules and flag precedence",
        render: config::render,
    },
    Topic {
        name: "languages",
        summary: "Supported languages and per-language node kinds",
        render: languages::render,
    },
    Topic {
        name: "branches",
        summary: "Friendly branch kinds and the raw node-kind escape hatch",
        render: branches::render,
    },
    Topic {
        name: "filters",
        summary: "File globs and --implements type filtering",
        render: filters::render,
    },
    Topic {
        name: "checks",
        summary: "--max-* limit checks and quiet mode",
        render: checks::render,
    },
];

/// The short directory page listed by a bare `--info`: one line per topic,
/// plus a pointer to drill into one.
pub fn directory() -> String {
    let mut page = String::from("TOPICS\n");
    for topic in TOPICS {
        page.push_str(&format!("  {:<12}{}\n", topic.name, topic.summary));
    }
    page.push_str("\nrun: smell --info <topic>\n");
    page
}

/// Renders the named topic's page, or `None` if it isn't registered.
pub fn topic(name: &str) -> Option<String> {
    TOPICS
        .iter()
        .find(|topic| topic.name == name)
        .map(|topic| (topic.render)())
}

/// Every registered topic name, in directory order, for unknown-topic
/// errors.
pub fn names() -> Vec<&'static str> {
    TOPICS.iter().map(|topic| topic.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_lists_every_topic() {
        let page = directory();
        for topic in TOPICS {
            assert!(page.contains(topic.name), "missing topic: {}", topic.name);
            assert!(
                page.contains(topic.summary),
                "missing summary for: {}",
                topic.name
            );
        }
    }

    #[test]
    fn every_directory_topic_resolves() {
        for registered in TOPICS {
            assert!(
                topic(registered.name).is_some(),
                "topic() can't resolve registered name: {}",
                registered.name
            );
        }
    }

    #[test]
    fn topic_pages_are_non_empty() {
        for registered in TOPICS {
            assert!(
                !topic(registered.name).unwrap().is_empty(),
                "empty page for topic: {}",
                registered.name
            );
        }
    }

    #[test]
    fn unknown_topic_returns_none() {
        assert!(topic("bogus").is_none());
    }

    #[test]
    fn directory_stays_short() {
        assert!(
            directory().lines().count() <= 15,
            "directory grew past a quick scan"
        );
    }
}
