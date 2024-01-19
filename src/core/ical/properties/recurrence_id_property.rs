use crate::core::ical::properties::macros::build_date_string_property_macro::build_date_string_property;
use crate::core::ical::properties::DTStartProperty;

// TODO: Cater to RANGE param:
//       - https://icalendar.org/iCalendar-RFC-5545/3-2-13-recurrence-identifier-range.html
//       - https://icalendar.org/iCalendar-RFC-5545/3-8-4-4-recurrence-id.html
build_date_string_property!("RECURRENCE-ID", RecurrenceIDProperty);

// Copy the contents of the DTStartProperty into RecurrenceIDProperty as it serves
// essentially the same purpose.
//
// TODO: Verify that reckless assertion above:
//       - https://icalendar.org/iCalendar-RFC-5545/3-8-4-4-recurrence-id.html
impl From<&DTStartProperty> for RecurrenceIDProperty {
    fn from(dtstart_property: &DTStartProperty) -> Self {
        let timezone = dtstart_property.timezone.to_owned();
        let utc_timestamp = dtstart_property.utc_timestamp.to_owned();

        let value_type = if dtstart_property.is_date_value_type() {
            Some(String::from("DATE"))
        } else {
            Some(String::from("DATE-TIME"))
        };

        RecurrenceIDProperty {
            timezone,
            value_type,
            utc_timestamp,
            x_params: None,
        }
    }
}
