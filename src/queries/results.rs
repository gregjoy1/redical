use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::data_types::EventInstance;

use super::query::ResultOrdering;

pub struct QueryResults<O>
where
    O: QueryResultOrdering,
{
    pub results: BinaryHeap<QueryResult<O>>,
}

impl<O> QueryResults<O>
where
    O: QueryResultOrdering,
{
    pub fn new() -> QueryResults<O> {
        QueryResults {
            results: BinaryHeap::new(),
        }
    }

    fn push(&mut self, event_instance: EventInstance) {
        let result_ordering = O::new_from_event_instance(&event_instance);

        let result = QueryResult {
            result_ordering,
            event_instance,
        };

        self.results.push(result);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QueryResult<O>
where
    O: QueryResultOrdering,
{
    pub result_ordering: O,
    pub event_instance:  EventInstance,
}

impl<O> PartialOrd for QueryResult<O>
where
    O: QueryResultOrdering,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.result_ordering.partial_cmp(&other.result_ordering)
    }
}

impl<O> Ord for QueryResult<O>
where
    O: QueryResultOrdering,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.result_ordering.cmp(&other.result_ordering)
    }
}

pub trait QueryResultItem: Ord + Eq + PartialEq {}

impl<O> QueryResultItem for QueryResult<O>
where
    O: QueryResultOrdering
{}

pub trait QueryResultOrdering: Ord + Eq + PartialEq {
    fn new_from_event_instance(event_instance: &EventInstance) -> Self;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct QueryResultDtstartOrdering(i64);

impl QueryResultOrdering for QueryResultDtstartOrdering {
    fn new_from_event_instance(event_instance: &EventInstance) -> Self {
        QueryResultDtstartOrdering(event_instance.dtstart_timestamp.clone())
    }
}

// TODO:
// #[derive(Debug, PartialEq, Eq, Clone)]
// pub struct QueryResultDtstartGeoDistOrdering(i64, Option<f64>);
// 
// impl Ord for QueryResultDtstartGeoDistOrdering {
//     fn cmp(&self, other: &Self) -> Ordering {
//         let key_comparison = self.key.cmp(&other.key);

//         if key_comparison.is_eq() {
//             self.value.cmp(&other.value)
//         } else {
//             key_comparison
//         }
//     }
// }

// #[derive(Debug, PartialEq, Eq, Clone)]
// pub struct QueryResultGeoDistDtstartOrdering(Option<f64>, i64);

// impl Ord for QueryResultGeoDistDtstartOrdering {
//     fn cmp(&self, other: &Self) -> Ordering {
//         let key_comparison = self.key.cmp(&other.key);

//         if key_comparison.is_eq() {
//             self.value.cmp(&other.value)
//         } else {
//             key_comparison
//         }
//     }
// }

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    use std::collections::BTreeSet;

    #[test]
    fn test_query_results_dtstart_ordering() {
        let event_instance_one = EventInstance {
            uuid:               String::from("UUID_ONE"),
            dtstart_timestamp:  100,
            dtend_timestamp:    110,
            duration:           10,
            categories:         None,
            related_to:         None,
            passive_properties: BTreeSet::new(),
        };

        let event_instance_two = EventInstance {
            uuid:               String::from("UUID_TWO"),
            dtstart_timestamp:  200,
            dtend_timestamp:    210,
            duration:           10,
            categories:         None,
            related_to:         None,
            passive_properties: BTreeSet::new(),
        };

        let event_instance_three = EventInstance {
            uuid:               String::from("UUID_THREE"),
            dtstart_timestamp:  300,
            dtend_timestamp:    310,
            duration:           10,
            categories:         None,
            related_to:         None,
            passive_properties: BTreeSet::new(),
        };

        let event_instance_four = EventInstance {
            uuid:               String::from("UUID_FOUR"),
            dtstart_timestamp:  400,
            dtend_timestamp:    410,
            duration:           10,
            categories:         None,
            related_to:         None,
            passive_properties: BTreeSet::new(),
        };

        let mut query_results: QueryResults<QueryResultDtstartOrdering> = QueryResults::new();

        assert!(query_results.results.is_empty());

        query_results.push(event_instance_four.clone());

        assert_eq!(
            query_results.results.clone().into_sorted_vec(),
            vec![
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(400),
                    event_instance:  event_instance_four.clone(),
                },
            ]
        );

        query_results.push(event_instance_one.clone());

        assert_eq!(
            query_results.results.clone().into_sorted_vec(),
            vec![
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(100),
                    event_instance:  event_instance_one.clone(),
                },
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(400),
                    event_instance:  event_instance_four.clone(),
                },
            ]
        );

        query_results.push(event_instance_three.clone());
        query_results.push(event_instance_two.clone());

        assert_eq!(
            query_results.results.clone().into_sorted_vec(),
            vec![
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(100),
                    event_instance:  event_instance_one.clone(),
                },
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(200),
                    event_instance:  event_instance_two.clone(),
                },
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(300),
                    event_instance:  event_instance_three.clone(),
                },
                QueryResult {
                    result_ordering: QueryResultDtstartOrdering(400),
                    event_instance:  event_instance_four.clone(),
                },
            ]
        );
    }
}
