mod uid;
pub mod event;

pub use uid::*;

use crate::ICalendarEntity;

pub trait ICalendarProperty {}

use crate::property_parameters::tzid::Tzid;
use crate::property_parameters::value_type::ValueType;
use crate::property_value_data_types::date_time::DateTime;

pub trait ICalendarDateTimeProperty {

    fn get_tzid(&self) -> Option<Tzid>;

    fn get_value_type(&self) -> Option<ValueType>;

    fn get_date_time(&self) -> DateTime;

    fn validate(&self) -> Result<(), String> {
        self.get_date_time().validate()?;

        if let Some(tzid) = self.get_tzid().as_ref() {
            tzid.validate()?;
        };

        if let Some(value_type) = self.get_value_type().as_ref() {
            value_type.validate_against_date_time(&self.get_date_time())?;
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! define_property_params_ical_parser {
    ($struct_name:ident, $(($parser_expr:expr, $handler:expr $(,)*), $(,)*)+ $(,)*) => {
        fn parse_ical(input: ParserInput) -> ParserResult<$struct_name> {
            let mut remaining = input;
            let mut params = $struct_name::default();

            loop {
                let Ok((new_remaining, _)) = semicolon(remaining) else {
                    break;
                };

                remaining = new_remaining;

                $(
                    match $parser_expr(remaining) {
                        Ok((new_remaining, key_value)) => {
                            remaining = new_remaining;

                            let handler = $handler;

                            handler(&mut params, key_value);

                            continue;
                        },

                        Err(nom::Err::Failure(error)) => {
                            return Err(nom::Err::Failure(error));
                        },

                        _ => {},
                    }
                )+

                break;
            }

            Ok((remaining, params))
        }
    }
}

pub use define_property_params_ical_parser;
