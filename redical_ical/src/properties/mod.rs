pub mod uid;
pub mod passive;
pub mod indexed;
// pub mod schedule;

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
