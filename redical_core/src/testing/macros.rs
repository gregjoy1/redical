#[macro_export]
macro_rules! build_property_from_ical {
    ($property_struct: ident, $property_ical: expr) => {
        match $property_struct::from_str($property_ical) {
            Ok(property) => property,
            Err(error) => {
                panic!("Error building property with ical: '{}' error: '{}'", $property_ical, error);
            },
        }
    };
}

pub use build_property_from_ical;
