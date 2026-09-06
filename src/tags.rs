use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// The `[tags]` section of the vault's own config, which is the single source
/// of truth for the tag vocabulary. Unknown keys in that file are ignored.
#[derive(Debug, Deserialize)]
struct VaultConfig {
    #[serde(default)]
    tags: Vocabulary,
}

#[derive(Debug, Default, Deserialize)]
pub struct Vocabulary {
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    functional: Vec<String>,
    #[serde(default = "default_max_topics")]
    max_topics: usize,
}

fn default_max_topics() -> usize {
    4
}

impl Vocabulary {
    /// Read the vocabulary from the vault config.
    ///
    /// Returns `None` when the file does not exist, so a vault without one
    /// simply skips validation rather than failing every `new`.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read tag vocabulary: {:?}", path))?;
        let parsed: VaultConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse tag vocabulary: {:?}", path))?;

        if parsed.tags.types.is_empty() && parsed.tags.topics.is_empty() {
            return Ok(None);
        }

        Ok(Some(parsed.tags))
    }

    /// Reject tags outside the vocabulary, so a note is never created with a
    /// tag the vault's own validator would flag.
    pub fn validate(&self, tags: &[String]) -> Result<()> {
        let mut topics = 0;

        for tag in tags {
            if let Some(name) = tag.strip_prefix("type/") {
                ensure_known(tag, name, &self.types, "type/")?;
            } else if let Some(name) = tag.strip_prefix("topic/") {
                ensure_known(tag, name, &self.topics, "topic/")?;
                topics += 1;
            } else if !self.functional.contains(tag) {
                bail!(
                    "unknown tag: {tag:?}\n  expected `type/<...>`, `topic/<...>`, or one of: {}",
                    join_or_none(&self.functional)
                );
            }
        }

        if topics > self.max_topics {
            bail!(
                "too many topic/ tags: {topics} (at most {} allowed)",
                self.max_topics
            );
        }

        Ok(())
    }
}

fn ensure_known(tag: &str, name: &str, allowed: &[String], prefix: &str) -> Result<()> {
    if allowed.iter().any(|a| a == name) {
        return Ok(());
    }
    bail!(
        "unknown tag: {tag:?}\n  known {prefix} values: {}",
        join_or_none(allowed)
    );
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(", ")
    }
}

/// Build the tag list for a new note: `defaults` first, then the tags the user
/// asked for, deduplicated in that order.
///
/// A user-supplied `type/` tag replaces the default one, so
/// `todo new --tag type/meeting` does not end up with two `type/` tags.
pub fn resolve(defaults: &[String], requested: &[String]) -> Vec<String> {
    let user_sets_type = requested.iter().any(|t| t.starts_with("type/"));

    let mut tags: Vec<String> = Vec::new();
    for tag in defaults.iter().chain(requested.iter()) {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if user_sets_type && tag.starts_with("type/") && defaults.iter().any(|d| d == tag) {
            continue;
        }
        if !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_string());
        }
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocabulary() -> Vocabulary {
        Vocabulary {
            types: vec!["todo".into(), "tech".into(), "meeting".into()],
            topics: vec!["rust".into(), "cli".into()],
            functional: vec!["clippings".into()],
            max_topics: 2,
        }
    }

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn known_tags_pass() {
        assert!(vocabulary().validate(&tags(&["type/todo", "topic/rust", "clippings"])).is_ok());
    }

    #[test]
    fn unknown_type_is_rejected_with_the_known_values() {
        let err = vocabulary().validate(&tags(&["type/bogus"])).unwrap_err().to_string();
        assert!(err.contains("unknown tag"), "{err}");
        assert!(err.contains("todo, tech, meeting"), "{err}");
    }

    #[test]
    fn unknown_topic_and_unprefixed_tags_are_rejected() {
        assert!(vocabulary().validate(&tags(&["topic/haskell"])).is_err());
        assert!(vocabulary().validate(&tags(&["random"])).is_err());
    }

    #[test]
    fn topic_count_is_capped() {
        assert!(vocabulary().validate(&tags(&["topic/rust", "topic/cli"])).is_ok());
        let mut vocab = vocabulary();
        vocab.max_topics = 1;
        assert!(vocab.validate(&tags(&["topic/rust", "topic/cli"])).is_err());
    }

    #[test]
    fn defaults_come_first_and_duplicates_collapse() {
        assert_eq!(
            resolve(&tags(&["type/todo"]), &tags(&["topic/rust", "topic/rust"])),
            tags(&["type/todo", "topic/rust"])
        );
    }

    #[test]
    fn an_explicit_type_tag_replaces_the_default() {
        assert_eq!(
            resolve(&tags(&["type/todo"]), &tags(&["type/meeting"])),
            tags(&["type/meeting"])
        );
    }

    #[test]
    fn no_defaults_and_no_request_yields_nothing() {
        assert!(resolve(&[], &[]).is_empty());
        assert_eq!(resolve(&[], &tags(&["type/tech"])), tags(&["type/tech"]));
    }

    #[test]
    fn a_missing_vocabulary_file_disables_validation() {
        assert!(Vocabulary::load(Path::new("/nonexistent/config.toml")).unwrap().is_none());
    }

    #[test]
    fn the_vault_config_shape_is_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "[other]\nfoo = 1\n\n[tags]\ntypes = [\"todo\"]\ntopics = [\"rust\"]\nfunctional = [\"clippings\"]\nmax_topics = 4\n\n[tags.note_type_map]\ndaily = \"type/daily\"\n",
        )
        .unwrap();

        let vocab = Vocabulary::load(&path).unwrap().unwrap();
        assert!(vocab.validate(&tags(&["type/todo", "topic/rust"])).is_ok());
        assert!(vocab.validate(&tags(&["type/meeting"])).is_err());
    }
}
