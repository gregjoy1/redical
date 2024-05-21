#[macro_use]
extern crate afl;
extern crate redical_ical;

use redical_ical::properties::{EventProperty, CalendarProperty, QueryProperty};

use std::str::FromStr;

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(fuzz_input) = std::str::from_utf8(data) {
            let _ = EventProperty::from_str(fuzz_input);
            let _ = CalendarProperty::from_str(fuzz_input);
            let _ = QueryProperty::from_str(fuzz_input);
        }
    });
}
