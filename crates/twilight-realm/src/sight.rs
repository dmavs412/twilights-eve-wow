//! Sight by quest state (#596, ADR 0035): a Hub NPC or a friendly spawn that only the characters
//! who have, or have not, handed in a quest see. A thing with no rule is seen by everyone.

use serde::Deserialize;

/// Who sees a thing, by the quests they have handed in (`quests.completed`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sight {
    /// Seen only by a character who has handed this quest in.
    #[serde(default)]
    pub(crate) after: Option<u32>,
    /// Seen only by a character who has not handed this quest in.
    #[serde(default)]
    pub(crate) before: Option<u32>,
}

impl Sight {
    /// Whether a character who has handed in `completed` sees it.
    pub(crate) fn sees(&self, completed: &[u32]) -> bool {
        self.after.is_none_or(|q| completed.contains(&q))
            && self.before.is_none_or(|q| !completed.contains(&q))
    }
}

/// Whether a thing with `sight` is seen by a character who has handed in `completed`: no rule is
/// seen by everyone.
pub(crate) fn sees(sight: &Option<Sight>, completed: &[u32]) -> bool {
    sight.as_ref().is_none_or(|s| s.sees(completed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn after_and_before_split_the_characters_by_one_hand_in() {
        let after = Sight {
            after: Some(7),
            before: None,
        };
        let before = Sight {
            after: None,
            before: Some(7),
        };
        assert!(!after.sees(&[]) && after.sees(&[7]));
        assert!(before.sees(&[]) && !before.sees(&[1, 7]));
        let both = Sight {
            after: Some(1),
            before: Some(2),
        };
        assert!(both.sees(&[1]) && !both.sees(&[1, 2]) && !both.sees(&[]));
        assert!(sees(&None, &[]));
    }
}
