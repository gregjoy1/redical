use crate::data_types::EventInstance;

pub trait QueryResultOrdering: Ord + Eq + PartialEq {
    fn new_from_event_instance(event_instance: &EventInstance) -> Self;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct QueryResultDtstartOrdering(pub i64);

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
