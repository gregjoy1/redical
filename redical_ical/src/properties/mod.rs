pub mod calendar;
pub mod component;
// pub mod event;
pub mod uid;
pub mod passive;

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
                        Ok((new_remaining, (key, value))) => {
                            remaining = new_remaining;

                            let handler = $handler;

                            handler(&mut params, key, value);

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

#[macro_export]
macro_rules! define_property_params {
    ($struct_name:ident, $enum_name:ident, $ical_name:expr, $(($enum_key:ident, $enum_value:ident, $struct_property:ident, $struct_property_type:ty $(,)*), $(,)*)+ $(,)*) => {
        enum $enum_name {
            $(
                $enum_key($enum_value),
            )+
        }

        impl ICalendarEntity for $enum_name {
            fn parse_ical(input: ParserInput) -> ParserResult<Self> {
                alt((
                    $(
                        map($enum_value::parse_ical, $enum_name::$enum_key),
                    )+
                ))(input)
            }

            fn render_ical(&self) -> String {
                match self {
                    $(
                        Self::$enum_key(param) => param.render_ical(),
                    )+
                }
            }
        }

        #[derive(Debug, Clone, Eq, PartialEq, Default)]
        pub struct $struct_name {
            $(
                pub $struct_property: $struct_property_type,
            )+
        }

        impl $struct_name {
            fn insert(&mut self, param: $enum_name) -> &mut Self {
                match param {
                    $(
                        $enum_name::$enum_key(param) => {
                            let _ = self.$struct_property.insert(param);
                        },
                    )+
                };

                self
            }
        }

        impl ICalendarEntity for $struct_name {
            fn parse_ical(input: ParserInput) -> ParserResult<Self> {
                context(
                    $ical_name,
                    fold_many0(
                        preceded(
                            semicolon,
                            cut($enum_name::parse_ical),
                        ),
                        $struct_name::default,
                        |mut params, param| {
                            params.insert(param);

                            params
                        },
                    ),
                )(input)
            }

            fn render_ical(&self) -> String {
                let mut output = String::new();

                $(
                    if self.$struct_property.is_some() {
                        output.push_str(format!(";{}", self.$struct_property.render_ical()).as_str());
                    }
                )+

                output
            }
        }

        impl_icalendar_entity_traits!($struct_name);
    }
}

pub use define_property_params;
