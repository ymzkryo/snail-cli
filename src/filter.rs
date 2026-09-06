use anyhow::{bail, Result};
use crate::note::Note;

/// Filter keys the parser accepts, listed in error messages.
const KEYS: &[&str] = &["status", "project", "context", "due", "review"];

/// Vocabulary shared by the two date fields, listed in error messages.
const DATE_WORDS: &[&str] = &["today", "overdue", "reached", "missing"];

/// How a date field is compared against today.
#[derive(Debug, PartialEq)]
enum DateMatch {
    /// Exactly today.
    Today,
    /// Set and strictly before today.
    Overdue,
    /// Set and today or earlier — the date has arrived.
    Reached,
    /// Not set.
    Missing,
    /// An exact `YYYY-MM-DD` date.
    On(String),
}

impl DateMatch {
    fn parse(key: &str, value: &str) -> Result<Self> {
        match value {
            "today" => Ok(Self::Today),
            "overdue" => Ok(Self::Overdue),
            "reached" => Ok(Self::Reached),
            "missing" | "none" => Ok(Self::Missing),
            _ if is_date(value) => Ok(Self::On(value.to_string())),
            _ => bail!(
                "unsupported value for `{key}:`: {value:?}\n  \
                 expected a YYYY-MM-DD date or one of: {}",
                DATE_WORDS.join(", ")
            ),
        }
    }

    fn matches(&self, field: &str, today: &str) -> bool {
        match self {
            Self::Today => field == today,
            Self::Overdue => !field.is_empty() && field < today,
            Self::Reached => !field.is_empty() && field <= today,
            Self::Missing => field.is_empty(),
            Self::On(date) => field == date,
        }
    }
}

/// How a free-text field is compared.
#[derive(Debug, PartialEq)]
enum TextMatch {
    /// Case-insensitive exact match.
    Is(String),
    /// Not set.
    Missing,
}

impl TextMatch {
    fn parse(value: &str) -> Self {
        match value {
            "missing" | "none" => Self::Missing,
            _ => Self::Is(value.to_lowercase()),
        }
    }

    fn matches(&self, field: &str) -> bool {
        match self {
            Self::Is(expected) => field.to_lowercase() == *expected,
            Self::Missing => field.is_empty(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Term {
    Status(TextMatch),
    Project(TextMatch),
    /// Compared with the leading `@` stripped from both sides, so
    /// `context:computer` and `context:@computer` are the same filter.
    Context(TextMatch),
    Due(DateMatch),
    Review(DateMatch),
}

impl Term {
    fn matches(&self, note: &Note, today: &str) -> bool {
        match self {
            Self::Status(m) => m.matches(&note.status),
            Self::Project(m) => m.matches(&note.project),
            Self::Context(m) => m.matches(note.context.trim_start_matches('@')),
            Self::Due(m) => m.matches(&note.due_date, today),
            Self::Review(m) => m.matches(&note.review_date, today),
        }
    }
}

/// A conjunction of filter terms: a note matches when every term matches.
#[derive(Debug, Default)]
pub struct Filter {
    terms: Vec<Term>,
}

impl Filter {
    /// Parse `--filter` arguments. Each argument may hold several
    /// comma-separated `key:value` terms, and every term must match (AND).
    ///
    /// Unknown keys and unsupported values are errors rather than silently
    /// ignored, so a filter that cannot work never looks like "0 results".
    pub fn parse(specs: &[String]) -> Result<Self> {
        let mut terms = Vec::new();

        for spec in specs {
            for raw in spec.split(',') {
                let raw = raw.trim();
                if raw.is_empty() {
                    continue;
                }

                let Some((key, value)) = raw.split_once(':') else {
                    bail!(
                        "invalid filter: {raw:?}\n  expected `key:value`, one of: {}",
                        KEYS.join(", ")
                    );
                };

                let key = key.trim().to_lowercase();
                let value = value.trim();
                if value.is_empty() {
                    bail!("invalid filter: {raw:?}\n  `{key}:` needs a value");
                }

                terms.push(match key.as_str() {
                    "status" => Term::Status(TextMatch::parse(value)),
                    "project" => Term::Project(TextMatch::parse(value)),
                    "context" => Term::Context(TextMatch::parse(value.trim_start_matches('@'))),
                    "due" => Term::Due(DateMatch::parse("due", value)?),
                    "review" => Term::Review(DateMatch::parse("review", value)?),
                    _ => bail!(
                        "unknown filter key: {key:?}\n  known keys: {}",
                        KEYS.join(", ")
                    ),
                });
            }
        }

        Ok(Self { terms })
    }

    pub fn matches(&self, note: &Note, today: &str) -> bool {
        self.terms.iter().all(|term| term.matches(note, today))
    }
}

/// Field to order the listing by.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum SortKey {
    /// Creation date, newest first (the default).
    Created,
    /// Due date, nearest first; notes with no due date come last.
    Due,
    /// Review date, nearest first; notes with no review date come last.
    Review,
}

impl SortKey {
    pub fn apply(self, notes: &mut [Note]) {
        match self {
            Self::Created => notes.sort_by(|a, b| b.created.cmp(&a.created)),
            Self::Due => sort_by_date(notes, |n| &n.due_date),
            Self::Review => sort_by_date(notes, |n| &n.review_date),
        }
    }
}

/// Ascending by date with unset dates pushed to the end, then newest-created
/// first so ties keep a stable, meaningful order.
fn sort_by_date(notes: &mut [Note], field: fn(&Note) -> &String) {
    notes.sort_by(|a, b| {
        let key = |n: &Note| {
            let value = field(n);
            (value.is_empty(), value.clone())
        };
        key(a).cmp(&key(b)).then_with(|| b.created.cmp(&a.created))
    });
}

/// True for a `YYYY-MM-DD` string.
fn is_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 3
        && [4, 2, 2] == [parts[0].len(), parts[1].len(), parts[2].len()]
        && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const TODAY: &str = "2026-09-06";

    fn note(frontmatter: &str) -> Note {
        let content = format!("---\nstatus: next\n{}\n---\n\n# title\n", frontmatter);
        Note::from_content(PathBuf::from("a.md"), &content).unwrap()
    }

    fn matches(spec: &str, frontmatter: &str) -> bool {
        Filter::parse(&[spec.to_string()])
            .unwrap()
            .matches(&note(frontmatter), TODAY)
    }

    #[test]
    fn no_filter_matches_everything() {
        let filter = Filter::parse(&[]).unwrap();
        assert!(filter.matches(&note("project: anything"), TODAY));
    }

    #[test]
    fn project_filter_compares_the_frontmatter_value() {
        assert!(matches("project:private", "project: private"));
        assert!(matches("project:private", "project: \"private\""));
        assert!(!matches("project:private", "project: outarc"));
        assert!(!matches("project:private", "project:"));
    }

    #[test]
    fn project_and_status_matching_is_case_insensitive() {
        assert!(matches("project:outarc", "project: Outarc"));
        assert!(matches("status:NEXT", "project:"));
    }

    #[test]
    fn context_matching_ignores_the_at_sign_on_either_side() {
        assert!(matches("context:@computer", "context: \"@computer\""));
        assert!(matches("context:computer", "context: \"@computer\""));
        assert!(matches("context:@computer", "context: computer"));
        assert!(!matches("context:@phone", "context: \"@computer\""));
    }

    #[test]
    fn missing_matches_an_absent_or_empty_value() {
        assert!(matches("context:missing", "context:"));
        assert!(matches("context:missing", "project: p"));
        assert!(!matches("context:missing", "context: \"@computer\""));
        assert!(matches("review:missing", "review_date:"));
        assert!(matches("project:none", "project: \"\""));
    }

    #[test]
    fn due_vocabulary_compares_against_today() {
        assert!(matches("due:today", "due_date: 2026-09-06"));
        assert!(!matches("due:today", "due_date: 2026-09-07"));
        assert!(matches("due:overdue", "due_date: 2026-09-05"));
        assert!(!matches("due:overdue", "due_date: 2026-09-06"));
        assert!(!matches("due:overdue", "due_date:"));
        assert!(matches("due:2026-09-30", "due_date: 2026-09-30"));
    }

    #[test]
    fn reached_covers_today_and_earlier() {
        assert!(matches("due:reached", "due_date: 2026-09-06"));
        assert!(matches("due:reached", "due_date: 2026-09-05"));
        assert!(!matches("due:reached", "due_date: 2026-09-07"));
        assert!(!matches("due:reached", "due_date:"));
    }

    #[test]
    fn review_filters_the_review_date_not_the_due_date() {
        assert!(matches("review:overdue", "review_date: 2026-09-01\ndue_date: 2026-12-01"));
        assert!(!matches("review:overdue", "review_date: 2026-12-01\ndue_date: 2026-09-01"));
    }

    #[test]
    fn comma_separated_terms_are_anded() {
        let frontmatter = "project: 151ero\ncontext: \"@computer\"";
        assert!(matches("status:next,context:@computer", frontmatter));
        assert!(matches("status:next, project:151ero", frontmatter));
        assert!(!matches("status:next,context:@phone", frontmatter));
        assert!(!matches("status:waiting,context:@computer", frontmatter));
    }

    #[test]
    fn repeated_flags_are_anded_too() {
        let filter = Filter::parse(&["status:next".into(), "context:@computer".into()]).unwrap();
        assert!(filter.matches(&note("context: \"@computer\""), TODAY));
        assert!(!filter.matches(&note("context: \"@phone\""), TODAY));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = Filter::parse(&["ctx:@computer".into()]).unwrap_err().to_string();
        assert!(err.contains("unknown filter key"), "{err}");
        assert!(err.contains("context"), "{err}");
    }

    #[test]
    fn unsupported_date_words_are_rejected() {
        let err = Filter::parse(&["due:week".into()]).unwrap_err().to_string();
        assert!(err.contains("unsupported value for `due:`"), "{err}");
        assert!(Filter::parse(&["review:week".into()]).is_err());
        assert!(Filter::parse(&["due:2026-9-6".into()]).is_err());
    }

    #[test]
    fn malformed_terms_are_rejected() {
        assert!(Filter::parse(&["status".into()]).is_err());
        assert!(Filter::parse(&["status:".into()]).is_err());
    }

    #[test]
    fn empty_segments_are_ignored() {
        let filter = Filter::parse(&["status:next,".into()]).unwrap();
        assert!(filter.matches(&note("project:"), TODAY));
        // A wholly empty spec parses to a filter that keeps everything.
        assert!(Filter::parse(&["".into()])
            .unwrap()
            .matches(&note("status: waiting"), TODAY));
    }

    #[test]
    fn unknown_status_values_simply_match_nothing() {
        assert!(!matches("status:bogus", "project:"));
    }

    #[test]
    fn sorting_by_due_puts_the_nearest_first_and_undated_last() {
        let mut notes = vec![
            note("due_date:\ndate: 2026-01-01"),
            note("due_date: 2026-09-30\ndate: 2026-01-01"),
            note("due_date: 2026-09-07\ndate: 2026-01-01"),
        ];
        SortKey::Due.apply(&mut notes);
        let dates: Vec<&str> = notes.iter().map(|n| n.due_date.as_str()).collect();
        assert_eq!(dates, vec!["2026-09-07", "2026-09-30", ""]);
    }

    #[test]
    fn sorting_by_created_puts_the_newest_first() {
        let mut notes = vec![note("date: 2026-01-01"), note("date: 2026-09-01")];
        SortKey::Created.apply(&mut notes);
        let dates: Vec<&str> = notes.iter().map(|n| n.created.as_str()).collect();
        assert_eq!(dates, vec!["2026-09-01", "2026-01-01"]);
    }
}
