use chrono::prelude::TimeZone;
use chrono::LocalResult;
use chrono_tz::Tz;

use nom::error::context;
use nom::sequence::pair;
use nom::combinator::{opt, map_res, recognize};
use nom::bytes::complete::take_while1;

use crate::grammar::{is_safe_char, is_wsp_char, solidus};

use crate::values::date_time::DateTime;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzid(pub Tz);

impl ICalendarEntity for Tzid {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZID",
            map_res(
                recognize(
                    pair(
                        opt(solidus),
                        // Small hack that allows paramtext chars except whitespace.
                        map_err_message!(
                            take_while1(|input: char| {
                                is_safe_char(input) && !is_wsp_char(input)
                            }),
                            "expected iCalendar RFC-5545 TZID",
                        ),
                    )
                ),
                |tzid: ParserInput| {
                    if let Ok(tz) = tzid.to_string().parse::<Tz>() {
                        Ok(Self(tz))
                    } else {
                        Err(String::from("invalid timezone"))
                    }
                }
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        self.0.to_string()
    }
}

impl Tzid {
    /// Validates the given `DateTime` value against the timezone represented by `Tzid`.
    ///
    /// This function checks if the provided `DateTime` can be represented in the timezone
    /// without ambiguity, such as during daylight saving time transitions.
    ///
    /// # Arguments
    ///
    /// * `date_time` - A reference to a `DateTime` object to be validated.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the `DateTime` is valid within the timezone.
    /// * `Err(String)` with an error message if the `DateTime` is invalid, possibly due to
    ///   being on a daylight savings threshold.
    pub fn validate_with_datetime_value(&self, date_time: &DateTime) -> Result<(), String> {
        match self.0.offset_from_local_datetime(&date_time.into()) {
            LocalResult::Single(_) => Ok(()),

            _ => Err(String::from("invalid date time with timezone (possibly daylight savings threshold)")),
        }
    }
}

impl From<Tzid> for Tz {
    fn from(tzid: Tzid) -> Self {
        tzid.0.to_owned()
    }
}

impl From<&Tzid> for Tz {
    fn from(tzid: &Tzid) -> Self {
        Tz::from(tzid.to_owned())
    }
}

impl_icalendar_entity_traits!(Tzid);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzid::parse_ical("America/New_York TESTING".into()),
            (
                " TESTING",
                Tzid(Tz::America__New_York),
            )
        );

        assert_parser_output!(
            Tzid::parse_ical("Etc/GMT+12 TESTING".into()),
            (
                " TESTING",
                Tzid(Tz::Etc__GMTPlus12),
            )
        );

        assert_parser_output!(
            Tzid::parse_ical("UTC TESTING".into()),
            (
                " TESTING",
                Tzid(Tz::UTC),
            )
        );

        assert_parser_error!(
            Tzid::parse_ical("INVALID TESTING".into()),
            nom::Err::Error(
                span: "INVALID TESTING",
                message: "invalid timezone",
                context: ["TZID"],
            )
        );

        macro_rules! assert_tzid_parser_output {
            ($timezone_identifier:expr, $expected_tz:expr $(,)*) => {
                assert_parser_output!(
                    Tzid::parse_ical($timezone_identifier.into()),
                    (
                        "",
                        Tzid($expected_tz),
                    )
                );
            }
        }

        assert_tzid_parser_output!("Africa/Algiers", Tz::Africa__Algiers);
        assert_tzid_parser_output!("Africa/Cairo", Tz::Africa__Cairo);
        assert_tzid_parser_output!("Africa/Casablanca", Tz::Africa__Casablanca);
        assert_tzid_parser_output!("Africa/Harare", Tz::Africa__Harare);
        assert_tzid_parser_output!("Africa/Johannesburg", Tz::Africa__Johannesburg);
        assert_tzid_parser_output!("Africa/Monrovia", Tz::Africa__Monrovia);
        assert_tzid_parser_output!("Africa/Nairobi", Tz::Africa__Nairobi);
        assert_tzid_parser_output!("America/Antigua", Tz::America__Antigua);
        assert_tzid_parser_output!("America/Argentina/Buenos_Aires", Tz::America__Argentina__Buenos_Aires);
        assert_tzid_parser_output!("America/Bogota", Tz::America__Bogota);
        assert_tzid_parser_output!("America/Caracas", Tz::America__Caracas);
        assert_tzid_parser_output!("America/Chicago", Tz::America__Chicago);
        assert_tzid_parser_output!("America/Chihuahua", Tz::America__Chihuahua);
        assert_tzid_parser_output!("America/Denver", Tz::America__Denver);
        assert_tzid_parser_output!("America/Godthab", Tz::America__Godthab);
        assert_tzid_parser_output!("America/Guatemala", Tz::America__Guatemala);
        assert_tzid_parser_output!("America/Guyana", Tz::America__Guyana);
        assert_tzid_parser_output!("America/Halifax", Tz::America__Halifax);
        assert_tzid_parser_output!("America/Indiana/Indianapolis", Tz::America__Indiana__Indianapolis);
        assert_tzid_parser_output!("America/Juneau", Tz::America__Juneau);
        assert_tzid_parser_output!("America/La_Paz", Tz::America__La_Paz);
        assert_tzid_parser_output!("America/Lima", Tz::America__Lima);
        assert_tzid_parser_output!("America/Los_Angeles", Tz::America__Los_Angeles);
        assert_tzid_parser_output!("America/Mazatlan", Tz::America__Mazatlan);
        assert_tzid_parser_output!("America/Mexico_City", Tz::America__Mexico_City);
        assert_tzid_parser_output!("America/Monterrey", Tz::America__Monterrey);
        assert_tzid_parser_output!("America/Montevideo", Tz::America__Montevideo);
        assert_tzid_parser_output!("America/New_York", Tz::America__New_York);
        assert_tzid_parser_output!("America/Phoenix", Tz::America__Phoenix);
        assert_tzid_parser_output!("America/Puerto_Rico", Tz::America__Puerto_Rico);
        assert_tzid_parser_output!("America/Regina", Tz::America__Regina);
        assert_tzid_parser_output!("America/Santiago", Tz::America__Santiago);
        assert_tzid_parser_output!("America/Sao_Paulo", Tz::America__Sao_Paulo);
        assert_tzid_parser_output!("America/St_Johns", Tz::America__St_Johns);
        assert_tzid_parser_output!("America/Tijuana", Tz::America__Tijuana);
        assert_tzid_parser_output!("America/Winnipeg", Tz::America__Winnipeg);
        assert_tzid_parser_output!("Asia/Almaty", Tz::Asia__Almaty);
        assert_tzid_parser_output!("Asia/Amman", Tz::Asia__Amman);
        assert_tzid_parser_output!("Asia/Baghdad", Tz::Asia__Baghdad);
        assert_tzid_parser_output!("Asia/Baku", Tz::Asia__Baku);
        assert_tzid_parser_output!("Asia/Bangkok", Tz::Asia__Bangkok);
        assert_tzid_parser_output!("Asia/Beirut", Tz::Asia__Beirut);
        assert_tzid_parser_output!("Asia/Chongqing", Tz::Asia__Chongqing);
        assert_tzid_parser_output!("Asia/Colombo", Tz::Asia__Colombo);
        assert_tzid_parser_output!("Asia/Dhaka", Tz::Asia__Dhaka);
        assert_tzid_parser_output!("Asia/Ho_Chi_Minh", Tz::Asia__Ho_Chi_Minh);
        assert_tzid_parser_output!("Asia/Hong_Kong", Tz::Asia__Hong_Kong);
        assert_tzid_parser_output!("Asia/Irkutsk", Tz::Asia__Irkutsk);
        assert_tzid_parser_output!("Asia/Jakarta", Tz::Asia__Jakarta);
        assert_tzid_parser_output!("Asia/Jerusalem", Tz::Asia__Jerusalem);
        assert_tzid_parser_output!("Asia/Kabul", Tz::Asia__Kabul);
        assert_tzid_parser_output!("Asia/Kamchatka", Tz::Asia__Kamchatka);
        assert_tzid_parser_output!("Asia/Karachi", Tz::Asia__Karachi);
        assert_tzid_parser_output!("Asia/Kathmandu", Tz::Asia__Kathmandu);
        assert_tzid_parser_output!("Asia/Kolkata", Tz::Asia__Kolkata);
        assert_tzid_parser_output!("Asia/Krasnoyarsk", Tz::Asia__Krasnoyarsk);
        assert_tzid_parser_output!("Asia/Kuala_Lumpur", Tz::Asia__Kuala_Lumpur);
        assert_tzid_parser_output!("Asia/Kuwait", Tz::Asia__Kuwait);
        assert_tzid_parser_output!("Asia/Magadan", Tz::Asia__Magadan);
        assert_tzid_parser_output!("Asia/Manila", Tz::Asia__Manila);
        assert_tzid_parser_output!("Asia/Muscat", Tz::Asia__Muscat);
        assert_tzid_parser_output!("Asia/Novosibirsk", Tz::Asia__Novosibirsk);
        assert_tzid_parser_output!("Asia/Qatar", Tz::Asia__Qatar);
        assert_tzid_parser_output!("Asia/Rangoon", Tz::Asia__Rangoon);
        assert_tzid_parser_output!("Asia/Riyadh", Tz::Asia__Riyadh);
        assert_tzid_parser_output!("Asia/Seoul", Tz::Asia__Seoul);
        assert_tzid_parser_output!("Asia/Shanghai", Tz::Asia__Shanghai);
        assert_tzid_parser_output!("Asia/Singapore", Tz::Asia__Singapore);
        assert_tzid_parser_output!("Asia/Srednekolymsk", Tz::Asia__Srednekolymsk);
        assert_tzid_parser_output!("Asia/Taipei", Tz::Asia__Taipei);
        assert_tzid_parser_output!("Asia/Tashkent", Tz::Asia__Tashkent);
        assert_tzid_parser_output!("Asia/Tbilisi", Tz::Asia__Tbilisi);
        assert_tzid_parser_output!("Asia/Tehran", Tz::Asia__Tehran);
        assert_tzid_parser_output!("Asia/Tokyo", Tz::Asia__Tokyo);
        assert_tzid_parser_output!("Asia/Ulaanbaatar", Tz::Asia__Ulaanbaatar);
        assert_tzid_parser_output!("Asia/Urumqi", Tz::Asia__Urumqi);
        assert_tzid_parser_output!("Asia/Vladivostok", Tz::Asia__Vladivostok);
        assert_tzid_parser_output!("Asia/Yakutsk", Tz::Asia__Yakutsk);
        assert_tzid_parser_output!("Asia/Yekaterinburg", Tz::Asia__Yekaterinburg);
        assert_tzid_parser_output!("Asia/Yerevan", Tz::Asia__Yerevan);
        assert_tzid_parser_output!("Atlantic/Azores", Tz::Atlantic__Azores);
        assert_tzid_parser_output!("Atlantic/Cape_Verde", Tz::Atlantic__Cape_Verde);
        assert_tzid_parser_output!("Atlantic/South_Georgia", Tz::Atlantic__South_Georgia);
        assert_tzid_parser_output!("Australia/Adelaide", Tz::Australia__Adelaide);
        assert_tzid_parser_output!("Australia/Brisbane", Tz::Australia__Brisbane);
        assert_tzid_parser_output!("Australia/Canberra", Tz::Australia__Canberra);
        assert_tzid_parser_output!("Australia/Darwin", Tz::Australia__Darwin);
        assert_tzid_parser_output!("Australia/Hobart", Tz::Australia__Hobart);
        assert_tzid_parser_output!("Australia/Melbourne", Tz::Australia__Melbourne);
        assert_tzid_parser_output!("Australia/NSW", Tz::Australia__NSW);
        assert_tzid_parser_output!("Australia/Perth", Tz::Australia__Perth);
        assert_tzid_parser_output!("Australia/Queensland", Tz::Australia__Queensland);
        assert_tzid_parser_output!("Australia/Sydney", Tz::Australia__Sydney);
        assert_tzid_parser_output!("CET", Tz::CET);
        assert_tzid_parser_output!("Etc/GMT+12", Tz::Etc__GMTPlus12);
        assert_tzid_parser_output!("Etc/UTC", Tz::Etc__UTC);
        assert_tzid_parser_output!("Europe/Amsterdam", Tz::Europe__Amsterdam);
        assert_tzid_parser_output!("Europe/Athens", Tz::Europe__Athens);
        assert_tzid_parser_output!("Europe/Belgrade", Tz::Europe__Belgrade);
        assert_tzid_parser_output!("Europe/Berlin", Tz::Europe__Berlin);
        assert_tzid_parser_output!("Europe/Bratislava", Tz::Europe__Bratislava);
        assert_tzid_parser_output!("Europe/Brussels", Tz::Europe__Brussels);
        assert_tzid_parser_output!("Europe/Bucharest", Tz::Europe__Bucharest);
        assert_tzid_parser_output!("Europe/Budapest", Tz::Europe__Budapest);
        assert_tzid_parser_output!("Europe/Copenhagen", Tz::Europe__Copenhagen);
        assert_tzid_parser_output!("Europe/Dublin", Tz::Europe__Dublin);
        assert_tzid_parser_output!("Europe/Helsinki", Tz::Europe__Helsinki);
        assert_tzid_parser_output!("Europe/Istanbul", Tz::Europe__Istanbul);
        assert_tzid_parser_output!("Europe/Kaliningrad", Tz::Europe__Kaliningrad);
        assert_tzid_parser_output!("Europe/Kiev", Tz::Europe__Kiev);
        assert_tzid_parser_output!("Europe/Lisbon", Tz::Europe__Lisbon);
        assert_tzid_parser_output!("Europe/Ljubljana", Tz::Europe__Ljubljana);
        assert_tzid_parser_output!("Europe/London", Tz::Europe__London);
        assert_tzid_parser_output!("Europe/Madrid", Tz::Europe__Madrid);
        assert_tzid_parser_output!("Europe/Minsk", Tz::Europe__Minsk);
        assert_tzid_parser_output!("Europe/Moscow", Tz::Europe__Moscow);
        assert_tzid_parser_output!("Europe/Paris", Tz::Europe__Paris);
        assert_tzid_parser_output!("Europe/Prague", Tz::Europe__Prague);
        assert_tzid_parser_output!("Europe/Riga", Tz::Europe__Riga);
        assert_tzid_parser_output!("Europe/Rome", Tz::Europe__Rome);
        assert_tzid_parser_output!("Europe/Samara", Tz::Europe__Samara);
        assert_tzid_parser_output!("Europe/Sarajevo", Tz::Europe__Sarajevo);
        assert_tzid_parser_output!("Europe/Skopje", Tz::Europe__Skopje);
        assert_tzid_parser_output!("Europe/Sofia", Tz::Europe__Sofia);
        assert_tzid_parser_output!("Europe/Stockholm", Tz::Europe__Stockholm);
        assert_tzid_parser_output!("Europe/Tallinn", Tz::Europe__Tallinn);
        assert_tzid_parser_output!("Europe/Vienna", Tz::Europe__Vienna);
        assert_tzid_parser_output!("Europe/Vilnius", Tz::Europe__Vilnius);
        assert_tzid_parser_output!("Europe/Volgograd", Tz::Europe__Volgograd);
        assert_tzid_parser_output!("Europe/Warsaw", Tz::Europe__Warsaw);
        assert_tzid_parser_output!("Europe/Zagreb", Tz::Europe__Zagreb);
        assert_tzid_parser_output!("Europe/Zurich", Tz::Europe__Zurich);
        assert_tzid_parser_output!("GMT", Tz::GMT);
        assert_tzid_parser_output!("Pacific/Apia", Tz::Pacific__Apia);
        assert_tzid_parser_output!("Pacific/Auckland", Tz::Pacific__Auckland);
        assert_tzid_parser_output!("Pacific/Chatham", Tz::Pacific__Chatham);
        assert_tzid_parser_output!("Pacific/Fakaofo", Tz::Pacific__Fakaofo);
        assert_tzid_parser_output!("Pacific/Fiji", Tz::Pacific__Fiji);
        assert_tzid_parser_output!("Pacific/Guadalcanal", Tz::Pacific__Guadalcanal);
        assert_tzid_parser_output!("Pacific/Guam", Tz::Pacific__Guam);
        assert_tzid_parser_output!("Pacific/Honolulu", Tz::Pacific__Honolulu);
        assert_tzid_parser_output!("Pacific/Majuro", Tz::Pacific__Majuro);
        assert_tzid_parser_output!("Pacific/Midway", Tz::Pacific__Midway);
        assert_tzid_parser_output!("Pacific/Noumea", Tz::Pacific__Noumea);
        assert_tzid_parser_output!("Pacific/Pago_Pago", Tz::Pacific__Pago_Pago);
        assert_tzid_parser_output!("Pacific/Port_Moresby", Tz::Pacific__Port_Moresby);
        assert_tzid_parser_output!("Pacific/Tongatapu", Tz::Pacific__Tongatapu);
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Tzid(Tz::America__New_York).render_ical(),
            String::from("America/New_York"),
        );
    }
}
