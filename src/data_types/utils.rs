use std::collections::HashSet;
use std::hash::Hash;

#[derive(Debug, Eq, PartialEq)]
pub struct UpdatedSetMembers<T>
where
    T: Eq + PartialEq + Hash + Clone
{
    pub removed:    HashSet<T>,
    pub maintained: HashSet<T>,
    pub added:      HashSet<T>,
}

impl<T> UpdatedSetMembers<T>
where
    T: Eq + PartialEq + Hash + Clone
{
    pub fn new(original: Option<&HashSet<T>>, updated: Option<&HashSet<T>>) -> Self
        where
            T: Eq + PartialEq + Hash + Clone
    {
        match (original, updated) {
            (None, None) => {
                UpdatedSetMembers {
                    removed:    HashSet::from([]),
                    maintained: HashSet::from([]),
                    added:      HashSet::from([]),
                }
            },

            (Some(original_set), None) => {
                UpdatedSetMembers {
                    removed:    original_set.clone(),
                    maintained: HashSet::from([]),
                    added:      HashSet::from([]),
                }
            },

            (None, Some(updated_set)) => {
                UpdatedSetMembers {
                    removed:    HashSet::from([]),
                    maintained: HashSet::from([]),
                    added:      updated_set.clone(),
                }
            },

            (Some(original_set), Some(updated_set)) => {
                let removed    = original_set.difference(&updated_set).map(|value| value.clone()).collect();
                let added      = updated_set.difference(&original_set).map(|value| value.clone()).collect();
                let maintained = original_set.intersection(&updated_set).map(|value| value.clone()).collect();

                UpdatedSetMembers {
                    removed,
                    maintained,
                    added,
                }
            },
        }
    }

    pub fn all_present_members(&self) -> HashSet<T> {
        let mut present_members_set = self.maintained.clone();

        present_members_set.extend(self.added.clone().into_iter());

        present_members_set
    }
}

mod test {
    use super::*;

    #[test]
    fn test_updated_set_members() {
        assert_eq!(
            UpdatedSetMembers::<String>::new(
                None,
                None
            ),
            UpdatedSetMembers {
                removed:    HashSet::from([]),
                maintained: HashSet::from([]),
                added:      HashSet::from([]),
            }
        );

        assert_eq!(
            UpdatedSetMembers::<String>::new(
                Some(
                    &HashSet::from([
                        String::from("REMOVED")
                    ])
                ),
                None
            ),
            UpdatedSetMembers {
                removed:    HashSet::from([String::from("REMOVED")]),
                maintained: HashSet::from([]),
                added:      HashSet::from([]),
            }
        );

        assert_eq!(
            UpdatedSetMembers::<String>::new(
                None,
                Some(
                    &HashSet::from([
                        String::from("ADDED")
                    ])
                ),
            ),
            UpdatedSetMembers {
                removed:    HashSet::from([]),
                maintained: HashSet::from([]),
                added:      HashSet::from([String::from("ADDED")]),
            }
        );

        assert_eq!(
            UpdatedSetMembers::<String>::new(
                Some(
                    &HashSet::from([
                        String::from("REMOVED"),
                        String::from("MAINTAINED"),
                    ])
                ),
                Some(
                    &HashSet::from([
                        String::from("MAINTAINED"),
                        String::from("ADDED"),
                    ])
                ),
            ),
            UpdatedSetMembers {
                removed:    HashSet::from([String::from("REMOVED")]),
                maintained: HashSet::from([String::from("MAINTAINED")]),
                added:      HashSet::from([String::from("ADDED")]),
            }
        );
    }
}
