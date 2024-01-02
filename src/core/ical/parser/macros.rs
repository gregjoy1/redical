#[macro_export]
macro_rules! build_property_params_value_parser {
    ($property_name:tt) => {
        context(
            "$property_name params",
            map(
                separated_list1(
                    common::semicolon_delimeter,
                    context(
                        "param",
                        separated_pair(
                            common::x_name,
                            char('='),
                            common::ParsedValue::parse_list(common::param_value),
                        ),
                    ),
                ),
                |parsed_params| {
                    let params: HashMap<&str, common::ParsedValue> =
                        parsed_params.into_iter()
                                     .map(|(key, value)| (key, value))
                                     .collect();

                    params
                }
            ),
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        context(
            concat!($property_name, " params"),
            map(
                separated_list1(
                    common::semicolon_delimeter,
                    alt(
                        (
                            $(
                                context(
                                    concat!($property_name, " param"),
                                    separated_pair(
                                        tag($param_name),
                                        char('='),
                                        cut($param_parser),
                                    ),
                                ),
                            )+
                            context(
                                "param",
                                separated_pair(
                                    common::x_name,
                                    char('='),
                                    common::ParsedValue::parse_list(common::param_value),
                                ),
                            ),
                        ),
                    ),
                ),
                |parsed_params| {
                    parsed_params.into_iter()
                                 .map(|(key, value)| (key, value))
                                 .collect()
                }
            ),
        )
    }
}

#[macro_export]
macro_rules! build_property_params_parser {
    ($property_name:tt) => {
        opt(
            preceded(
                common::semicolon_delimeter,
                build_property_params_value_parser!($property_name)
            )
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        opt(
            preceded(
                common::semicolon_delimeter,
                build_property_params_value_parser!(
                    $property_name,
                    $(
                        ($param_name, $param_parser),
                    )+
                )
            )
        )
    }
}

#[macro_export]
macro_rules! build_date_time_property_parser {
    (
        $property_name:expr,
        $input_variable:ident
    ) => {
        preceded(
            tag($property_name),
            cut(context(
                $property_name,
                tuple((
                    build_property_params_parser!(
                        $property_name,
                        ("TZID", common::ParsedValue::parse_timezone)
                    ),
                    common::colon_delimeter,
                    common::ParsedValue::parse_date_string,
                )),
            )),
        )($input_variable)
        .map(
            |(remaining, (parsed_params, _colon_delimeter, parsed_value))| {
                let parsed_content_line =
                    common::consumed_input_string($input_variable, remaining, $property_name);

                let parsed_property = common::ParsedPropertyContent {
                    name: Some($property_name),
                    params: parsed_params,
                    value: parsed_value,
                    content_line: parsed_content_line,
                };

                (remaining, parsed_property)
            },
        )
    };
}

pub use build_date_time_property_parser;
pub use build_property_params_parser;
pub use build_property_params_value_parser;
