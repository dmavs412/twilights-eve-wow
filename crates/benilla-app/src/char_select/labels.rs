//! What a row's info line names instead of the class: a Mode whose characters are more than their
//! class fills it per guid (Twilight's Eve's Callings, "Level 7 Basic Warrior"). Without it, or
//! for a guid it does not name, the row shows the class, as the stock screen does.

use std::collections::HashMap;

use bevy::prelude::*;

/// A character's label by guid.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub(crate) struct ClassLabels(pub(crate) HashMap<u64, String>);

impl ClassLabels {
    /// The label for `guid`, or `class` when there is none.
    pub(crate) fn label<'a>(labels: Option<&'a Self>, guid: u64, class: &'a str) -> &'a str {
        labels
            .and_then(|l| l.0.get(&guid))
            .map_or(class, String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_replaces_the_class_only_for_its_guid() {
        let labels = ClassLabels(HashMap::from([(7, "Basic Warrior".to_string())]));
        assert_eq!(
            ClassLabels::label(Some(&labels), 7, "Warrior"),
            "Basic Warrior"
        );
        assert_eq!(ClassLabels::label(Some(&labels), 8, "Warrior"), "Warrior");
        assert_eq!(ClassLabels::label(None, 7, "Warrior"), "Warrior");
    }
}
