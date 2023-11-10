use crate::data_types::{FilterProperty, LowerBoundFilterCondition, UpperBoundFilterCondition};

#[derive(Debug, PartialEq, Clone)]
pub enum RangeConditionProperty {
    DtStart(i64),
    DtEnd(i64),
}

impl Into<FilterProperty> for RangeConditionProperty {
    fn into(self) -> FilterProperty {
        match self {
            RangeConditionProperty::DtStart(dtstart_timestamp) => FilterProperty::DtStart(dtstart_timestamp),
            RangeConditionProperty::DtEnd(dtend_timestamp)     => FilterProperty::DtEnd(dtend_timestamp),
        }
    }
}

impl RangeConditionProperty {

    pub fn get_property_value(&self, dtstart_timestamp: &i64, duration: &i64) -> (i64, i64) {
        match self {
            RangeConditionProperty::DtStart(comparison) => (dtstart_timestamp.to_owned(),   comparison.to_owned()),
            RangeConditionProperty::DtEnd(comparison)   => ((dtstart_timestamp + duration), comparison.to_owned()),
        }
    }

}

#[derive(Debug, PartialEq, Clone)]
pub enum LowerBoundRangeCondition {
    GreaterThan(RangeConditionProperty, Option<String>),
    GreaterEqualThan(RangeConditionProperty, Option<String>),
}

impl Into<LowerBoundFilterCondition> for LowerBoundRangeCondition {
    fn into(self) -> LowerBoundFilterCondition {
        match self {
            LowerBoundRangeCondition::GreaterThan(range_condition_property, _event_uuid) => {
                LowerBoundFilterCondition::GreaterThan(range_condition_property.into())
            },

            LowerBoundRangeCondition::GreaterEqualThan(range_condition_property, _event_uuid) => {
                LowerBoundFilterCondition::GreaterEqualThan(range_condition_property.into())
            },
        }
    }
}

impl LowerBoundRangeCondition {

    pub fn is_filtered(&self, _event_uuid: String, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self {
            LowerBoundRangeCondition::GreaterThan(range_condition_property, _range_condition_event_uuid) => {
                let (value, comparison) = range_condition_property.get_property_value(dtstart_timestamp, duration);

                value > comparison
            },

            LowerBoundRangeCondition::GreaterEqualThan(range_condition_property, _range_condition_event_uuid) => {
                let (value, comparison) = range_condition_property.get_property_value(dtstart_timestamp, duration);

                value >= comparison
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum UpperBoundRangeCondition {
    LessThan(RangeConditionProperty),
    LessEqualThan(RangeConditionProperty),
}

impl Into<UpperBoundFilterCondition> for UpperBoundRangeCondition {
    fn into(self) -> UpperBoundFilterCondition {
        match self {
            UpperBoundRangeCondition::LessThan(range_condition_property) => {
                UpperBoundFilterCondition::LessThan(range_condition_property.into())
            },

            UpperBoundRangeCondition::LessEqualThan(range_condition_property) => {
                UpperBoundFilterCondition::LessEqualThan(range_condition_property.into())
            },
        }
    }
}

impl UpperBoundRangeCondition {

    pub fn is_filtered(&self, _event_uuid: String, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self {
            UpperBoundRangeCondition::LessThan(range_condition_property) => {
                let (value, comparison) = range_condition_property.get_property_value(dtstart_timestamp, duration);

                value < comparison
            },

            UpperBoundRangeCondition::LessEqualThan(range_condition_property) => {
                let (value, comparison) = range_condition_property.get_property_value(dtstart_timestamp, duration);

                value <= comparison
            },
        }
    }
}
