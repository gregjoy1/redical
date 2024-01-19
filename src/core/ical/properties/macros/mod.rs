#[macro_use]
pub mod build_rrule_property_macro;

#[macro_use]
pub mod build_date_string_property_macro;

#[macro_export]
macro_rules! implement_property_ord_partial_ord_and_hash_traits {
    ($property_name:ident) => {
        impl PartialOrd for $property_name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.serialize_to_split_ical(None).partial_cmp(&other.serialize_to_split_ical(None))
            }
        }

        impl Ord for $property_name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.serialize_to_split_ical(None).cmp(&other.serialize_to_split_ical(None))
            }
        }

        impl std::hash::Hash for $property_name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.serialize_to_ical(None).hash(state);
            }
        }
    }
}

pub use implement_property_ord_partial_ord_and_hash_traits;
