//! Why a skill changed on this device (Y8), which the commit that publishes it carries to other
//! devices.

use serde::{Deserialize, Serialize};

/// Why a skill changed on this device. A change SkillBinder did not make is `Outside`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Reason {
    /// Imported from a found copy, with the agent or workspace it was found in, when known.
    Imported {
        /// The agent's or workspace's name.
        from: Option<String>,
    },
    /// Installed from Explore.
    Installed,
    /// Updated from its source.
    Updated,
    /// Reinstalled from its record.
    Reinstalled,
    /// Restored from History to a past version, made at `at`, in Unix seconds.
    Restored {
        /// When the restored version was made.
        at: u64,
    },
    /// Its record was written, such as with a source learned after import.
    Record,
    /// Changed outside SkillBinder.
    Outside,
}

impl Reason {
    /// The word the database and commit trailers store it as.
    pub fn word(&self) -> &'static str {
        match self {
            Self::Imported { .. } => "imported",
            Self::Installed => "installed",
            Self::Updated => "updated",
            Self::Reinstalled => "reinstalled",
            Self::Restored { .. } => "restored",
            Self::Record => "record",
            Self::Outside => "outside",
        }
    }

    /// What the database and commit trailers store after the word.
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Imported { from } => from.clone(),
            Self::Restored { at } => Some(at.to_string()),
            Self::Installed | Self::Updated | Self::Reinstalled | Self::Record | Self::Outside => {
                None
            }
        }
    }

    /// The reason stored as `word` and `detail`; an unknown word, such as one a newer version
    /// wrote, or a restore without its time, reads as `Outside`.
    pub fn stored(word: &str, detail: Option<String>) -> Self {
        match word {
            "imported" => Self::Imported { from: detail },
            "installed" => Self::Installed,
            "updated" => Self::Updated,
            "reinstalled" => Self::Reinstalled,
            "restored" => match detail.and_then(|at| at.trim().parse().ok()) {
                Some(at) => Self::Restored { at },
                None => Self::Outside,
            },
            "record" => Self::Record,
            _ => Self::Outside,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Reason;

    #[test]
    fn every_reason_reads_back_as_stored() {
        for reason in [
            Reason::Imported {
                from: Some("Claude Code".into()),
            },
            Reason::Imported { from: None },
            Reason::Installed,
            Reason::Updated,
            Reason::Reinstalled,
            Reason::Restored { at: 1_763_000_000 },
            Reason::Record,
            Reason::Outside,
        ] {
            let stored = Reason::stored(reason.word(), reason.detail());
            assert_eq!(stored, reason);
        }
        assert_eq!(Reason::stored("renamed", None), Reason::Outside);
        assert_eq!(Reason::stored("restored", None), Reason::Outside);
    }
}
