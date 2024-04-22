use nom::sequence::{pair, preceded};
use nom::error::context;
use nom::branch::alt;
use nom::combinator::{map, map_res, opt, cut};
use nom::multi::separated_list1;
use nom::character::is_digit;
use nom::bytes::complete::{tag, take_while1};

use crate::grammar::{comma, semicolon};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

use crate::values::date_time::DateTime;
use crate::values::integer::Integer;
use crate::values::list::List;

#[macro_export]
macro_rules! build_ical_param {
    ($struct_name:ident, $key_str:expr, $value_parser:expr, $value_type:ty $(,)*) => {
        #[derive(Debug, Clone, Eq, PartialEq)]
        pub struct $struct_name(pub $value_type);

        impl ICalendarEntity for $struct_name {
            fn parse_ical(input: ParserInput) -> ParserResult<Self> {
                map(
                    pair(
                        tag($key_str),
                        preceded(tag("="), cut($value_parser)),
                    ),
                    |(_key, value)| Self(value)
                )(input)
            }

            fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
                format!("{}={}", $key_str, self.0.render_ical())
            }
        }

        impl_icalendar_entity_traits!($struct_name);
    }
}

build_ical_param!(FreqParam, "FREQ", freq, Frequency);
build_ical_param!(UntilParam, "UNTIL", enddate, DateTime);
build_ical_param!(CountParam, "COUNT", count, Integer);
build_ical_param!(IntervalParam, "INTERVAL", interval, Integer);
build_ical_param!(BysecondParam, "BYSECOND", byseclist, List<Integer>);
build_ical_param!(ByminuteParam, "BYMINUTE", byminlist, List<Integer>);
build_ical_param!(ByhourParam, "BYHOUR", byhrlist, List<Integer>);
build_ical_param!(BydayParam, "BYDAY", bywdaylist, List<WeekDayNum>);
build_ical_param!(BymonthdayParam, "BYMONTHDAY", bymodaylist, List<Integer>);
build_ical_param!(ByyeardayParam, "BYYEARDAY", byyrdaylist, List<Integer>);
build_ical_param!(ByweeknoParam, "BYWEEKNO", bywknolist, List<Integer>);
build_ical_param!(BymonthParam, "BYMONTH", bymolist, List<Integer>);
build_ical_param!(BysetposParam, "BYSETPOS", bysplist, List<Integer>);
build_ical_param!(WkstParam, "WKST", WeekDay::parse_ical, WeekDay);

/// recur-rule-part = ( "FREQ" "=" freq )
///                 / ( "UNTIL" "=" enddate )
///                 / ( "COUNT" "=" 1*DIGIT )
///                 / ( "INTERVAL" "=" 1*DIGIT )
///                 / ( "BYSECOND" "=" byseclist )
///                 / ( "BYMINUTE" "=" byminlist )
///                 / ( "BYHOUR" "=" byhrlist )
///                 / ( "BYDAY" "=" bywdaylist )
///                 / ( "BYMONTHDAY" "=" bymodaylist )
///                 / ( "BYYEARDAY" "=" byyrdaylist )
///                 / ( "BYWEEKNO" "=" bywknolist )
///                 / ( "BYMONTH" "=" bymolist )
///                 / ( "BYSETPOS" "=" bysplist )
///                 / ( "WKST" "=" weekday )
enum RecurRulePart {
    Freq(FreqParam),
    Until(UntilParam),
    Count(CountParam),
    Interval(IntervalParam),
    Bysecond(BysecondParam),
    Byminute(ByminuteParam),
    Byhour(ByhourParam),
    Byday(BydayParam),
    Bymonthday(BymonthdayParam),
    Byyearday(ByyeardayParam),
    Byweekno(ByweeknoParam),
    Bymonth(BymonthParam),
    Bysetpos(BysetposParam),
    Wkst(WkstParam),
}

impl ICalendarEntity for RecurRulePart {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RECUR-RULE-PART",
            alt((
                map(FreqParam::parse_ical, |param| Self::Freq(param)),
                map(UntilParam::parse_ical, |param| Self::Until(param)),
                map(CountParam::parse_ical, |param| Self::Count(param)),
                map(IntervalParam::parse_ical, |param| Self::Interval(param)),
                map(BysecondParam::parse_ical, |param| Self::Bysecond(param)),
                map(ByminuteParam::parse_ical, |param| Self::Byminute(param)),
                map(ByhourParam::parse_ical, |param| Self::Byhour(param)),
                map(BydayParam::parse_ical, |param| Self::Byday(param)),
                map(BymonthdayParam::parse_ical, |param| Self::Bymonthday(param)),
                map(ByyeardayParam::parse_ical, |param| Self::Byyearday(param)),
                map(ByweeknoParam::parse_ical, |param| Self::Byweekno(param)),
                map(BymonthParam::parse_ical, |param| Self::Bymonth(param)),
                map(BysetposParam::parse_ical, |param| Self::Bysetpos(param)),
                map(WkstParam::parse_ical, |param| Self::Wkst(param)),
            ))
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::Freq(param) => param.render_ical(),
            Self::Until(param) => param.render_ical(),
            Self::Count(param) => param.render_ical(),
            Self::Interval(param) => param.render_ical(),
            Self::Bysecond(param) => param.render_ical(),
            Self::Byminute(param) => param.render_ical(),
            Self::Byhour(param) => param.render_ical(),
            Self::Byday(param) => param.render_ical(),
            Self::Bymonthday(param) => param.render_ical(),
            Self::Byyearday(param) => param.render_ical(),
            Self::Byweekno(param) => param.render_ical(),
            Self::Bymonth(param) => param.render_ical(),
            Self::Bysetpos(param) => param.render_ical(),
            Self::Wkst(param) => param.render_ical(),
        }
    }
}

/// freq        = "SECONDLY" / "MINUTELY" / "HOURLY" / "DAILY"
///             / "WEEKLY" / "MONTHLY" / "YEARLY"
pub fn freq(input: ParserInput) -> ParserResult<Frequency> {
    Frequency::parse_ical(input)
}

/// interval       = 1*DIGIT
pub fn interval(input: ParserInput) -> ParserResult<Integer> {
    let (remaining, interval) = take_while1(|value| is_digit(value as u8))(input)?;

    let Ok(parsed_interval) = interval.to_string().parse::<u64>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("Invalid interval"), input)
            )
        );
    };

    Ok((remaining, Integer::from(parsed_interval)))
}

/// count       = 1*DIGIT
pub fn count(input: ParserInput) -> ParserResult<Integer> {
    let (remaining, count) = take_while1(|value| is_digit(value as u8))(input)?;

    let Ok(parsed_count) = count.to_string().parse::<u64>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("Invalid count"), input)
            )
        );
    };

    Ok((remaining, Integer::from(parsed_count)))
}

/// enddate     = date / date-time
pub fn enddate(input: ParserInput) -> ParserResult<DateTime> {
    DateTime::parse_ical(input)
}

/// byseclist   = ( seconds *("," seconds) )
pub fn byseclist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, seconds), List::from)(input)
}

/// seconds     = 1*2DIGIT       ;0 to 60
pub fn seconds(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_unsigned_m_n(1, 2, 0, 60)(input)
}

/// byminlist   = ( minutes *("," minutes) )
pub fn byminlist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, minutes), List::from)(input)
}

/// minutes     = 1*2DIGIT       ;0 to 59
pub fn minutes(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_unsigned_m_n(1, 2, 0, 59)(input)
}

/// byhrlist    = ( hour *("," hour) )
pub fn byhrlist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, hour), List::from)(input)
}

/// hour        = 1*2DIGIT       ;0 to 23
pub fn hour(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_unsigned_m_n(1, 2, 0, 23)(input)
}

/// bywdaylist  = ( weekdaynum *("," weekdaynum) )
pub fn bywdaylist(input: ParserInput) -> ParserResult<List<WeekDayNum>> {
    map(separated_list1(comma, weekdaynum), List::from)(input)
}

/// weekdaynum  = [[plus / minus] ordwk] weekday
/// ordwk       = 1*2DIGIT       ;1 to 53
/// weekday     = "SU" / "MO" / "TU" / "WE" / "TH" / "FR" / "SA"
/// ;Corresponding to SUNDAY, MONDAY, TUESDAY, WEDNESDAY, THURSDAY,
/// ;FRIDAY, and SATURDAY days of the week.
pub fn weekdaynum(input: ParserInput) -> ParserResult<WeekDayNum> {
    WeekDayNum::parse_ical(input)
}

/// bymodaylist = ( monthdaynum *("," monthdaynum) )
pub fn bymodaylist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, monthdaynum), List::from)(input)
}

/// monthdaynum = [plus / minus] ordmoday
/// ordmoday    = 1*2DIGIT       ;1 to 31
pub fn monthdaynum(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_signed_m_n(1, 2, 1, 31)(input)
}

/// byyrdaylist = ( yeardaynum *("," yeardaynum) )
pub fn byyrdaylist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, yeardaynum), List::from)(input)
}

/// yeardaynum  = [plus / minus] ordyrday
/// ordyrday    = 1*3DIGIT      ;1 to 366
pub fn yeardaynum(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_signed_m_n(1, 3, 1, 366)(input)
}

/// bywknolist  = ( weeknum *("," weeknum) )
pub fn bywknolist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, weeknum), List::from)(input)
}

/// weeknum     = [plus / minus] ordwk
/// ordwk       = 1*2DIGIT       ;1 to 53
pub fn weeknum(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_signed_m_n(1, 2, 1, 53)(input)
}

/// bymolist    = ( monthnum *("," monthnum) )
pub fn bymolist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, monthnum), List::from)(input)
}

/// monthnum    = 1*2DIGIT       ;1 to 12
pub fn monthnum(input: ParserInput) -> ParserResult<Integer> {
    Integer::parse_unsigned_m_n(1, 2, 1, 12)(input)
}

/// bysplist    = ( setposday *("," setposday) )
/// setposday   = yeardaynum
pub fn bysplist(input: ParserInput) -> ParserResult<List<Integer>> {
    map(separated_list1(comma, yeardaynum), List::from)(input)
}

/// Frequency enum
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use redical_ical::values::recur::Frequency;
/// use redical_ical::ICalendarEntity;
///
/// assert_eq!(Frequency::from_str("SECONDLY"), Ok(Frequency::Secondly));
/// assert_eq!(Frequency::from_str("MINUTELY"), Ok(Frequency::Minutely));
/// assert_eq!(Frequency::from_str("HOURLY"), Ok(Frequency::Hourly));
/// assert_eq!(Frequency::from_str("DAILY"), Ok(Frequency::Daily));
/// assert_eq!(Frequency::from_str("WEEKLY"), Ok(Frequency::Weekly));
/// assert_eq!(Frequency::from_str("MONTHLY"), Ok(Frequency::Monthly));
/// assert_eq!(Frequency::from_str("YEARLY"), Ok(Frequency::Yearly));
///
/// assert_eq!(Frequency::Secondly.render_ical(), String::from("SECONDLY"));
/// assert_eq!(Frequency::Minutely.render_ical(), String::from("MINUTELY"));
/// assert_eq!(Frequency::Hourly.render_ical(), String::from("HOURLY"));
/// assert_eq!(Frequency::Daily.render_ical(), String::from("DAILY"));
/// assert_eq!(Frequency::Weekly.render_ical(), String::from("WEEKLY"));
/// assert_eq!(Frequency::Monthly.render_ical(), String::from("MONTHLY"));
/// assert_eq!(Frequency::Yearly.render_ical(), String::from("YEARLY"));
/// ```
///
/// freq        = "SECONDLY" / "MINUTELY" / "HOURLY" / "DAILY"
///             / "WEEKLY" / "MONTHLY" / "YEARLY"
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Frequency {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl ICalendarEntity for Frequency {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        context(
            "FREQ",
            alt((
                map(tag("SECONDLY"), |_| Self::Secondly),
                map(tag("MINUTELY"), |_| Self::Minutely),
                map(tag("HOURLY"), |_| Self::Hourly),
                map(tag("DAILY"), |_| Self::Daily),
                map(tag("WEEKLY"), |_| Self::Weekly),
                map(tag("MONTHLY"), |_| Self::Monthly),
                map(tag("YEARLY"), |_| Self::Yearly),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::Secondly => String::from("SECONDLY"),
            Self::Minutely => String::from("MINUTELY"),
            Self::Hourly => String::from("HOURLY"),
            Self::Daily => String::from("DAILY"),
            Self::Weekly => String::from("WEEKLY"),
            Self::Monthly => String::from("MONTHLY"),
            Self::Yearly => String::from("YEARLY"),
        }
    }
}

impl_icalendar_entity_traits!(Frequency);

/// Week-day num struct
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use redical_ical::values::recur::{WeekDayNum, WeekDay};
/// use redical_ical::values::integer::Integer;
/// use redical_ical::ICalendarEntity;
///
/// assert_eq!(WeekDayNum::from_str("SU"), Ok(WeekDayNum(None, WeekDay::Sunday)));
/// assert_eq!(WeekDayNum::from_str("FR"), Ok(WeekDayNum(None, WeekDay::Friday)));
/// assert_eq!(WeekDayNum::from_str("SA"), Ok(WeekDayNum(None, WeekDay::Saturday)));
///
/// assert_eq!(WeekDayNum::from_str("-1SU"), Ok(WeekDayNum(Some(Integer(-1_i64)), WeekDay::Sunday)));
/// assert_eq!(WeekDayNum::from_str("31MO"), Ok(WeekDayNum(Some(Integer(31_i64)), WeekDay::Monday)));
/// assert_eq!(WeekDayNum::from_str("+1SA"), Ok(WeekDayNum(Some(Integer(1_i64)), WeekDay::Saturday)));
///
/// assert_eq!(WeekDayNum(None, WeekDay::Sunday).render_ical(), String::from("SU"));
/// assert_eq!(WeekDayNum(None, WeekDay::Friday).render_ical(), String::from("FR"));
/// assert_eq!(WeekDayNum(None, WeekDay::Saturday).render_ical(), String::from("SA"));
///
/// assert_eq!(WeekDayNum(Some(Integer(-1_i64)), WeekDay::Sunday).render_ical(), String::from("-1SU"));
/// assert_eq!(WeekDayNum(Some(Integer(31_i64)), WeekDay::Monday).render_ical(), String::from("31MO"));
/// assert_eq!(WeekDayNum(Some(Integer(1_i64)), WeekDay::Saturday).render_ical(), String::from("1SA"));
/// ```
///
/// weekdaynum  = [[plus / minus] ordwk] weekday
/// ordwk       = 1*2DIGIT       ;1 to 53
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct WeekDayNum(pub Option<Integer>, pub WeekDay);

impl ICalendarEntity for WeekDayNum {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        // weekday     = "SU" / "MO" / "TU" / "WE" / "TH" / "FR" / "SA"
        // ;Corresponding to SUNDAY, MONDAY, TUESDAY, WEDNESDAY, THURSDAY,
        // ;FRIDAY, and SATURDAY days of the week.
        //
        // weekdaynum  = [[plus / minus] ordwk] weekday
        // ordwk       = 1*2DIGIT       ;1 to 53
        context(
            "WEEKDAYNUM",
            |input| {
                let (remaining, ordwk) = opt(Integer::parse_signed_m_n(1, 2, 1, 53))(input)?;
                let (remaining, weekday) = WeekDay::parse_ical(remaining)?;

                Ok((remaining, WeekDayNum(ordwk, weekday)))
            }
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        let mut output = String::new();

        if let Some(ordwk) = self.0.as_ref() {
            output.push_str(ordwk.render_ical().as_str());
        }

        output.push_str(self.1.render_ical().as_str());

        output
    }
}

impl_icalendar_entity_traits!(WeekDayNum);

/// Week-day enum
///
/// # Examples
///
/// ```rust
/// use std::str::FromStr;
/// use redical_ical::values::recur::WeekDay;
/// use redical_ical::ICalendarEntity;
///
/// assert_eq!(WeekDay::from_str("SU"), Ok(WeekDay::Sunday));
/// assert_eq!(WeekDay::from_str("MO"), Ok(WeekDay::Monday));
/// assert_eq!(WeekDay::from_str("TU"), Ok(WeekDay::Tuesday));
/// assert_eq!(WeekDay::from_str("WE"), Ok(WeekDay::Wednesday));
/// assert_eq!(WeekDay::from_str("TH"), Ok(WeekDay::Thursday));
/// assert_eq!(WeekDay::from_str("FR"), Ok(WeekDay::Friday));
/// assert_eq!(WeekDay::from_str("SA"), Ok(WeekDay::Saturday));
///
/// assert_eq!(WeekDay::Sunday.render_ical(), String::from("SU"));
/// assert_eq!(WeekDay::Monday.render_ical(), String::from("MO"));
/// assert_eq!(WeekDay::Tuesday.render_ical(), String::from("TU"));
/// assert_eq!(WeekDay::Wednesday.render_ical(), String::from("WE"));
/// assert_eq!(WeekDay::Thursday.render_ical(), String::from("TH"));
/// assert_eq!(WeekDay::Friday.render_ical(), String::from("FR"));
/// assert_eq!(WeekDay::Saturday.render_ical(), String::from("SA"));
/// ```
///
/// weekday     = "SU" / "MO" / "TU" / "WE" / "TH" / "FR" / "SA"
/// ;Corresponding to SUNDAY, MONDAY, TUESDAY, WEDNESDAY, THURSDAY,
/// ;FRIDAY, and SATURDAY days of the week.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WeekDay {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl ICalendarEntity for WeekDay {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        // weekday     = "SU" / "MO" / "TU" / "WE" / "TH" / "FR" / "SA"
        // ;Corresponding to SUNDAY, MONDAY, TUESDAY, WEDNESDAY, THURSDAY,
        // ;FRIDAY, and SATURDAY days of the week.
        context(
            "WEEKDAYNUM",
            alt((
                map(tag("SU"), |_| Self::Sunday),
                map(tag("MO"), |_| Self::Monday),
                map(tag("TU"), |_| Self::Tuesday),
                map(tag("WE"), |_| Self::Wednesday),
                map(tag("TH"), |_| Self::Thursday),
                map(tag("FR"), |_| Self::Friday),
                map(tag("SA"), |_| Self::Saturday),
            ))
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::Sunday => String::from("SU"),
            Self::Monday => String::from("MO"),
            Self::Tuesday => String::from("TU"),
            Self::Wednesday => String::from("WE"),
            Self::Thursday => String::from("TH"),
            Self::Friday => String::from("FR"),
            Self::Saturday => String::from("SA"),
        }
    }
}

impl_icalendar_entity_traits!(WeekDay);

/// recur           = recur-rule-part *( ";" recur-rule-part )
///                 ;
///                 ; The rule parts are not ordered in any
///                 ; particular sequence.
///                 ;
///                 ; The FREQ rule part is REQUIRED,
///                 ; but MUST NOT occur more than once.
///                 ;
///                 ; The UNTIL or COUNT rule parts are OPTIONAL,
///                 ; but they MUST NOT occur in the same 'recur'.
///                 ;
///                 ; The other rule parts are OPTIONAL,
///                 ; but MUST NOT occur more than once.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Recur {
    pub freq: Option<FreqParam>,
    pub until: Option<UntilParam>,
    pub count: Option<CountParam>,
    pub interval: Option<IntervalParam>,
    pub bysecond: Option<BysecondParam>,
    pub byminute: Option<ByminuteParam>,
    pub byhour: Option<ByhourParam>,
    pub byday: Option<BydayParam>,
    pub bymonthday: Option<BymonthdayParam>,
    pub byyearday: Option<ByyeardayParam>,
    pub byweekno: Option<ByweeknoParam>,
    pub bymonth: Option<BymonthParam>,
    pub bysetpos: Option<BysetposParam>,
    pub wkst: Option<WkstParam>,
}

impl ICalendarEntity for Recur {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RECUR",
            map_res(
                separated_list1(
                    semicolon,
                    RecurRulePart::parse_ical,
                ),
                |recur_rule_parts| {
                    let mut recur = Recur::default();

                    for recur_rule_part in recur_rule_parts {
                        recur.insert(recur_rule_part);
                    }

                    if let Err(error) = recur.validate() {
                        return Err(ParserError::new(error, input));
                    }

                    Ok(recur)
                }
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        fn push_rendered_ical_if_present<T: ICalendarEntity>(property: &Option<T>, parts: &mut Vec<String>) {
            if let Some(property) = property {
                parts.push(property.render_ical());
            }
        }

        let mut parts: Vec<String> = Vec::new();

        push_rendered_ical_if_present(&self.byday, &mut parts);
        push_rendered_ical_if_present(&self.byhour, &mut parts);
        push_rendered_ical_if_present(&self.byminute, &mut parts);
        push_rendered_ical_if_present(&self.bymonth, &mut parts);
        push_rendered_ical_if_present(&self.bymonthday, &mut parts);
        push_rendered_ical_if_present(&self.bysecond, &mut parts);
        push_rendered_ical_if_present(&self.bysetpos, &mut parts);
        push_rendered_ical_if_present(&self.byweekno, &mut parts);
        push_rendered_ical_if_present(&self.byyearday, &mut parts);
        push_rendered_ical_if_present(&self.count, &mut parts);
        push_rendered_ical_if_present(&self.freq, &mut parts);
        push_rendered_ical_if_present(&self.interval, &mut parts);
        push_rendered_ical_if_present(&self.until, &mut parts);
        push_rendered_ical_if_present(&self.wkst, &mut parts);

        parts.join(";")
    }

    fn validate(&self) -> Result<(), String> {
        if self.freq.is_none() {
            return Err(String::from("FREQ required"));
        }

        if self.interval.is_none() {
            return Err(String::from("INTERVAL required"));
        }

        Ok(())
    }
}

impl Default for Recur {
    fn default() -> Self {
        Recur {
            freq: None,
            until: None,
            count: None,
            interval: None,
            bysecond: None,
            byminute: None,
            byhour: None,
            byday: None,
            bymonthday: None,
            byyearday: None,
            byweekno: None,
            bymonth: None,
            bysetpos: None,
            wkst: None,
        }
    }
}

impl Recur {
    fn insert(&mut self, recur_rule_part: RecurRulePart) {
        match recur_rule_part {
            RecurRulePart::Freq(param) => self.freq = Some(param),
            RecurRulePart::Until(param) => self.until = Some(param),
            RecurRulePart::Count(param) => self.count = Some(param),
            RecurRulePart::Interval(param) => self.interval = Some(param),
            RecurRulePart::Bysecond(param) => self.bysecond = Some(param),
            RecurRulePart::Byminute(param) => self.byminute = Some(param),
            RecurRulePart::Byhour(param) => self.byhour = Some(param),
            RecurRulePart::Byday(param) => self.byday = Some(param),
            RecurRulePart::Bymonthday(param) => self.bymonthday = Some(param),
            RecurRulePart::Byyearday(param) => self.byyearday = Some(param),
            RecurRulePart::Byweekno(param) => self.byweekno = Some(param),
            RecurRulePart::Bymonth(param) => self.bymonth = Some(param),
            RecurRulePart::Bysetpos(param) => self.bysetpos = Some(param),
            RecurRulePart::Wkst(param) => self.wkst = Some(param),
        };
    }
}

impl_icalendar_entity_traits!(Recur);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Recur::parse_ical("FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 TESTING".into()),
            (
                " TESTING",
                Recur {
                    freq: Some(FreqParam(Frequency::Yearly)),
                    until: None,
                    count: None,
                    interval: Some(IntervalParam(Integer(2))),
                    bysecond: None,
                    byminute: Some(ByminuteParam(List::from(vec![Integer(30)]))),
                    byhour: Some(ByhourParam(List::from(vec![Integer(8), Integer(9)]))),
                    byday: Some(BydayParam(List::from(vec![WeekDayNum(Some(Integer(-1)), WeekDay::Monday), WeekDayNum(None, WeekDay::Sunday)]))),
                    bymonthday: None,
                    byyearday: None,
                    byweekno: None,
                    bymonth: Some(BymonthParam(List::from(vec![Integer(1)]))),
                    bysetpos: None,
                    wkst: None,
                },
            ),
        );

        assert_parser_output!(
            Recur::parse_ical("FREQ=DAILY;COUNT=10;INTERVAL=2 TESTING".into()),
            (
                " TESTING",
                Recur {
                    freq: Some(FreqParam(Frequency::Daily)),
                    until: None,
                    count: Some(CountParam(Integer(10))),
                    interval: Some(IntervalParam(Integer(2))),
                    bysecond: None,
                    byminute: None,
                    byhour: None,
                    byday: None,
                    bymonthday: None,
                    byyearday: None,
                    byweekno: None,
                    bymonth: None,
                    bysetpos: None,
                    wkst: None,
                },
            ),
        );

        assert!(Recur::parse_ical("OTHER".into()).is_err());
        assert!(Recur::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Recur {
                freq: Some(FreqParam(Frequency::Yearly)),
                until: None,
                count: None,
                interval: Some(IntervalParam(Integer(2))),
                bysecond: None,
                byminute: Some(ByminuteParam(List::from(vec![Integer(30)]))),
                byhour: Some(ByhourParam(List::from(vec![Integer(8), Integer(9)]))),
                byday: Some(BydayParam(List::from(vec![WeekDayNum(Some(Integer(-1)), WeekDay::Monday), WeekDayNum(None, WeekDay::Sunday)]))),
                bymonthday: None,
                byyearday: None,
                byweekno: None,
                bymonth: Some(BymonthParam(List::from(vec![Integer(1)]))),
                bysetpos: None,
                wkst: None,
            }.render_ical(),
            String::from("BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30;BYMONTH=1;FREQ=YEARLY;INTERVAL=2"),
        );

        assert_eq!(
            Recur {
                freq: Some(FreqParam(Frequency::Daily)),
                until: None,
                count: Some(CountParam(Integer(10))),
                interval: Some(IntervalParam(Integer(2))),
                bysecond: None,
                byminute: None,
                byhour: None,
                byday: None,
                bymonthday: None,
                byyearday: None,
                byweekno: None,
                bymonth: None,
                bysetpos: None,
                wkst: None,
            }.render_ical(),
            String::from("COUNT=10;FREQ=DAILY;INTERVAL=2"),
        );
    }

    #[test]
    fn validate() {
        assert_eq!(
            Recur {
                freq: None,
                until: None,
                count: None,
                interval: None,
                bysecond: None,
                byminute: None,
                byhour: None,
                byday: None,
                bymonthday: None,
                byyearday: None,
                byweekno: None,
                bymonth: None,
                bysetpos: None,
                wkst: None,
            }.validate(),
            Err(String::from("FREQ required")),
        );

        assert_eq!(
            Recur {
                freq: Some(FreqParam(Frequency::Daily)),
                until: None,
                count: None,
                interval: None,
                bysecond: None,
                byminute: None,
                byhour: None,
                byday: None,
                bymonthday: None,
                byyearday: None,
                byweekno: None,
                bymonth: None,
                bysetpos: None,
                wkst: None,
            }.validate(),
            Err(String::from("INTERVAL required")),
        );

        assert_eq!(
            Recur {
                freq: Some(FreqParam(Frequency::Daily)),
                until: None,
                count: None,
                interval: Some(IntervalParam(Integer(2))),
                bysecond: None,
                byminute: None,
                byhour: None,
                byday: None,
                bymonthday: None,
                byyearday: None,
                byweekno: None,
                bymonth: None,
                bysetpos: None,
                wkst: None,
            }.validate(),
            Ok(()),
        );
    }
}
