use crate::{FilterProperty, LowerBoundFilterCondition, UpperBoundFilterCondition};

use redical_ical::properties::ICalendarDateTimeProperty;
use redical_ical::properties::query::{XFromProperty, XUntilProperty};
use redical_ical::values::where_range_operator::{WhereFromRangeOperator, WhereUntilRangeOperator};
use redical_ical::values::where_range_property::WhereRangeProperty;

#[derive(Debug, PartialEq, Clone)]
pub enum RangeConditionProperty {
    DtStart(i64),
    DtEnd(i64),
}

impl From<RangeConditionProperty> for FilterProperty {
    fn from(range_condition_property: RangeConditionProperty) -> Self {
        match range_condition_property {
            RangeConditionProperty::DtStart(dtstart_timestamp) => {
                FilterProperty::DtStart(dtstart_timestamp)
            }

            RangeConditionProperty::DtEnd(dtend_timestamp) => {
                FilterProperty::DtEnd(dtend_timestamp)
            }
        }
    }
}

impl RangeConditionProperty {
    pub fn get_property_value(&self, dtstart_timestamp: &i64, duration: &i64) -> (i64, i64) {
        match self {
            RangeConditionProperty::DtStart(comparison) => {
                (dtstart_timestamp.to_owned(), comparison.to_owned())
            }
            RangeConditionProperty::DtEnd(comparison) => {
                ((dtstart_timestamp + duration), comparison.to_owned())
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum LowerBoundRangeCondition {
    GreaterThan(RangeConditionProperty),
    GreaterEqualThan(RangeConditionProperty),
}

impl From<XFromProperty> for LowerBoundRangeCondition {
    fn from(x_from_property: XFromProperty) -> Self {
        let range_condition_property =
            match x_from_property.params.prop {
                WhereRangeProperty::DTStart => {
                    RangeConditionProperty::DtStart(x_from_property.get_utc_timestamp())
                },

                WhereRangeProperty::DTEnd => {
                    RangeConditionProperty::DtEnd(x_from_property.get_utc_timestamp())
                },
            };

        match x_from_property.params.op {
            WhereFromRangeOperator::GreaterThan => {
                LowerBoundRangeCondition::GreaterThan(range_condition_property)
            },

            WhereFromRangeOperator::GreaterEqualThan => {
                LowerBoundRangeCondition::GreaterEqualThan(range_condition_property)
            },
        }
    }
}

impl From<&XFromProperty> for LowerBoundRangeCondition {
    fn from(x_from_property: &XFromProperty) -> Self {
        Self::from(x_from_property.to_owned())
    }
}

impl From<LowerBoundRangeCondition> for LowerBoundFilterCondition {
    fn from(lower_bound_range_condition: LowerBoundRangeCondition) -> Self {
        match lower_bound_range_condition {
            LowerBoundRangeCondition::GreaterThan(range_condition_property) => {
                LowerBoundFilterCondition::GreaterThan(range_condition_property.into())
            }

            LowerBoundRangeCondition::GreaterEqualThan(range_condition_property) => {
                LowerBoundFilterCondition::GreaterEqualThan(range_condition_property.into())
            }
        }
    }
}

impl LowerBoundRangeCondition {
    pub fn is_filtered(&self, _event_uid: String, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self {
            LowerBoundRangeCondition::GreaterThan(range_condition_property) => {
                let (value, comparison) =
                    range_condition_property.get_property_value(dtstart_timestamp, duration);

                value > comparison
            }

            LowerBoundRangeCondition::GreaterEqualThan(range_condition_property) => {
                let (value, comparison) =
                    range_condition_property.get_property_value(dtstart_timestamp, duration);

                value >= comparison
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum UpperBoundRangeCondition {
    LessThan(RangeConditionProperty),
    LessEqualThan(RangeConditionProperty),
}

impl From<XUntilProperty> for UpperBoundRangeCondition {
    fn from(x_until_property: XUntilProperty) -> Self {
        let range_condition_property =
            match x_until_property.params.prop {
                WhereRangeProperty::DTStart => {
                    RangeConditionProperty::DtStart(x_until_property.get_utc_timestamp())
                },

                WhereRangeProperty::DTEnd => {
                    RangeConditionProperty::DtEnd(x_until_property.get_utc_timestamp())
                },
            };

        match x_until_property.params.op {
            WhereUntilRangeOperator::LessThan => {
                UpperBoundRangeCondition::LessThan(range_condition_property)
            },

            WhereUntilRangeOperator::LessEqualThan => {
                UpperBoundRangeCondition::LessEqualThan(range_condition_property)
            },
        }
    }
}

impl From<&XUntilProperty> for UpperBoundRangeCondition {
    fn from(x_until_property: &XUntilProperty) -> Self {
        Self::from(x_until_property.to_owned())
    }
}

impl From<UpperBoundRangeCondition> for UpperBoundFilterCondition {
    fn from(upper_bound_range_condition: UpperBoundRangeCondition) -> Self {
        match upper_bound_range_condition {
            UpperBoundRangeCondition::LessThan(range_condition_property) => {
                UpperBoundFilterCondition::LessThan(range_condition_property.into())
            }

            UpperBoundRangeCondition::LessEqualThan(range_condition_property) => {
                UpperBoundFilterCondition::LessEqualThan(range_condition_property.into())
            }
        }
    }
}

impl UpperBoundRangeCondition {
    pub fn is_filtered(&self, _event_uid: String, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self {
            UpperBoundRangeCondition::LessThan(range_condition_property) => {
                let (value, comparison) =
                    range_condition_property.get_property_value(dtstart_timestamp, duration);

                value < comparison
            }

            UpperBoundRangeCondition::LessEqualThan(range_condition_property) => {
                let (value, comparison) =
                    range_condition_property.get_property_value(dtstart_timestamp, duration);

                value <= comparison
            }
        }
    }
}
