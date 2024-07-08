mod uid;
mod recurrence_id;
mod last_modified;

pub mod event;
pub mod calendar;
pub mod query;

pub use uid::*;
pub use recurrence_id::*;
pub use last_modified::*;

pub use event::*;
pub use calendar::*;
pub use query::*;

use crate::{ICalendarEntity, RenderingContext};

use crate::values::tzid::Tzid;
use crate::values::date_time::{DateTime, ValueType};

use crate::content_line::{ContentLine, ContentLineParams};

pub trait ICalendarPropertyParams {
    fn to_content_line_params(&self) -> ContentLineParams {
        self.to_content_line_params_with_context(None)
    }

    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams;
}

impl<P> From<&P> for ContentLineParams
where
    P: ICalendarPropertyParams,
{
    fn from(property_params: &P) -> Self {
        property_params.to_content_line_params()
    }
}

pub trait ICalendarProperty {
    fn to_content_line(&self) -> ContentLine {
        self.to_content_line_with_context(None)
    }

    fn to_content_line_with_context(&self, context: Option<&RenderingContext>) -> ContentLine;
}

impl<P> From<&P> for ContentLine
where
    P: ICalendarProperty,
{
    fn from(property: &P) -> Self {
        property.to_content_line()
    }
}

pub trait ICalendarGeoProperty {
    fn get_latitude(&self) -> f64;
    fn get_longitude(&self) -> f64;
}

pub trait ICalendarDateTimeProperty {
    fn new_from<P>(from_property: &P) -> Self
    where
        P: ICalendarDateTimeProperty,
        Self: Sized,
    {
        Self::new(
            from_property.get_value_type(),
            from_property.get_tzid(),
            from_property.get_date_time(),
        )
    }

    fn new_from_utc_timestamp(utc_timestamp: &i64) -> Self
    where
        Self: Sized,
    {
        Self::new(
            None,
            None,
            &DateTime::from(utc_timestamp.to_owned()),
        )
    }

    fn new(value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self;

    fn get_tzid(&self) -> Option<&Tzid>;

    fn get_tz(&self) -> Option<&chrono_tz::Tz> {
        self.get_tzid().map(|tzid| &tzid.0)
    }

    fn get_utc_timestamp(&self) -> i64 {
        self.get_date_time()
            .get_utc_timestamp(self.get_tz())
    }

    fn get_value_type(&self) -> Option<&ValueType>;

    fn get_date_time(&self) -> &DateTime;

    fn validate(&self) -> Result<(), String> {
        self.get_date_time().validate()?;

        if let Some(tzid) = self.get_tzid().as_ref() {
            tzid.validate()?;
        };

        if let Some(value_type) = self.get_value_type().as_ref() {
            value_type.validate_against_date_time(self.get_date_time())?;
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
