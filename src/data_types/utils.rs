use std::collections::{HashMap, HashSet, BTreeSet};
use std::hash::Hash;
use std::fmt::Debug;

use std::cmp::Ordering;

use serde::{Serialize, Deserialize};

pub fn hashmap_to_hashset(hash_map: Option<&HashMap<String, HashSet<String>>>) -> Option<HashSet<(String, String)>> {
    hash_map.and_then(|hash_map| {
        let mut set_members = HashSet::<(String, String)>::new();

        for (key, values) in hash_map.iter() {
            for value in values.iter() {
                set_members.insert(
                    (
                        key.clone(),
                        value.clone()
                    )
                );
            }
        }

        if set_members.is_empty() {
            None
        } else {
            Some(set_members)
        }
    })
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
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

    pub fn is_unchanged(&self) -> bool {
        self.removed.is_empty() && self.added.is_empty()
    }

    pub fn is_changed(&self) -> bool {
        !self.is_unchanged()
    }
}

#[derive(Debug)]
struct MergedIteratorBufferItem<T: Ord + Debug, I: Iterator<Item = T> + Debug> (String, T, I);

impl<T: Ord + Debug, I: Iterator<Item = T> + Debug> Ord for MergedIteratorBufferItem<T, I> {
    // Set equality also relies on this alongside the ordering, so:
    // Compare self<T> with other<T>
    // If these are equal - fall back to comparing tag Strings 
    // This ensures that two equal inserted buffer items with different
    // tag strings are regarded as distinct by the BTreeSet and not mistakenly
    // de-deuplicated.
    fn cmp(&self, other: &Self) -> Ordering {
        let comparison = self.1.cmp(&other.1);

        match comparison {
            Ordering::Equal => self.0.cmp(&other.0),

            _ => comparison
        }
    }
}

impl<T: Ord + Debug, I: Iterator<Item = T> + Debug> PartialOrd for MergedIteratorBufferItem<T, I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord + Debug, I: Iterator<Item = T> + Debug> PartialEq for MergedIteratorBufferItem<T, I> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<T: Ord + Debug, I: Iterator<Item = T> + Debug> Eq for MergedIteratorBufferItem<T, I> {}

#[derive(Debug)]
pub struct MergedIterator<T, I>
where
    T: Ord + Debug,
    I: Iterator<Item = T> + Debug,
{
    buffer: BTreeSet<MergedIteratorBufferItem<T, I>>
}

impl<T, I> MergedIterator<T, I>
where
    T: Ord + Debug,
    I: Iterator<Item = T> + Debug,
{
    pub fn new() -> Self {
        MergedIterator {
            buffer: BTreeSet::new(),
        }
    }

    pub fn add_iter(&mut self, tag: String, mut iterator: I) -> Result<&Self, String> {
        if let Some(value) = iterator.next() {
            self.buffer.insert(
                MergedIteratorBufferItem(tag, value, iterator)
            );
        }

        Ok(self)
    }

}

impl<T, I> Iterator for MergedIterator<T, I>
where
    T: Ord + Debug,
    I: Iterator<Item = T> + Debug,
{
    type Item = (String, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer
            .pop_first()
            .and_then(|MergedIteratorBufferItem(tag, item, iterator)| {
                let _result = self.add_iter(tag.clone(), iterator);

                Some((tag, item))
            })
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

    #[test]
    fn test_merged_iterator() {
        #[derive(Debug)]
        struct IteratorValue (i64, i32, String);

        impl Ord for IteratorValue {
            // Sort first by first value, then falling back to second value.
            fn cmp(&self, other: &Self) -> Ordering {
                let comparison = self.0.cmp(&other.0);

                match comparison {
                    Ordering::Equal => self.1.cmp(&other.1),

                    _ => comparison
                }
            }
        }

        impl PartialOrd for IteratorValue {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for IteratorValue {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0 && self.1 == other.1
            }
        }

        impl Eq for IteratorValue {}

        let mut merged_iterator = MergedIterator::new();

        let iterator_values_one = vec![
            IteratorValue(100, 0,  String::from("one-100-0")),
            IteratorValue(200, 0,  String::from("one-200-0")),
            IteratorValue(200, 10, String::from("one-200-10")),
            IteratorValue(400, 10, String::from("one-400-10")),
        ];

        let iterator_values_two = vec![
            IteratorValue(10, 0,  String::from("two-10-0")),
            IteratorValue(20, 0,  String::from("two-20-0")),
            IteratorValue(20, 10, String::from("two-20-10")),
            IteratorValue(40, 10, String::from("two-40-10")),
        ];

        let iterator_values_three = vec![
            IteratorValue(100, 0,  String::from("three-100-0")),
            IteratorValue(800, 0,  String::from("three-800-0")),
        ];

        let iterator_values_four = vec![];

        assert!(merged_iterator.add_iter(String::from("ONE"),   iterator_values_one.into_iter()).is_ok());
        assert!(merged_iterator.add_iter(String::from("TWO"),   iterator_values_two.into_iter()).is_ok());
        assert!(merged_iterator.add_iter(String::from("THREE"), iterator_values_three.into_iter()).is_ok());
        assert!(merged_iterator.add_iter(String::from("FOUR"),  iterator_values_four.into_iter()).is_ok());

        assert_eq!(merged_iterator.next(), Some((String::from("TWO"),   IteratorValue(10,  0,  String::from("two-10-0")))));
        assert_eq!(merged_iterator.next(), Some((String::from("TWO"),   IteratorValue(20,  0,  String::from("two-20-0")))));
        assert_eq!(merged_iterator.next(), Some((String::from("TWO"),   IteratorValue(20,  10, String::from("two-20-10")))));
        assert_eq!(merged_iterator.next(), Some((String::from("TWO"),   IteratorValue(40,  10, String::from("two-40-10")))));
        assert_eq!(merged_iterator.next(), Some((String::from("ONE"),   IteratorValue(100, 0,  String::from("one-100-0")))));
        assert_eq!(merged_iterator.next(), Some((String::from("THREE"), IteratorValue(100, 0,  String::from("three-100-0")))));
        assert_eq!(merged_iterator.next(), Some((String::from("ONE"),   IteratorValue(200, 0,  String::from("one-200-0")))));
        assert_eq!(merged_iterator.next(), Some((String::from("ONE"),   IteratorValue(200, 10, String::from("one-200-10")))));
        assert_eq!(merged_iterator.next(), Some((String::from("ONE"),   IteratorValue(400, 10, String::from("one-400-10")))));
        assert_eq!(merged_iterator.next(), Some((String::from("THREE"), IteratorValue(800, 0,  String::from("three-800-0")))));

        assert_eq!(merged_iterator.next(), None);
    }
}
