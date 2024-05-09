#[macro_use]
extern crate afl;
extern crate redical_ical;

use redical_ical::properties::QueryProperties;

use std::str::FromStr;

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(fuzz_input) = std::str::from_utf8(data) {
            let _ = QueryProperties::from_str(fuzz_input);
        }
    });
}
