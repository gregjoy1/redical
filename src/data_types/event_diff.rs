use crate::data_types::{UpdatedSetMembers, Event, ScheduleProperties, IndexedProperties, PassiveProperties, EventOccurrenceOverrides, InvertedEventIndex, hashmap_to_hashset};

use std::collections::HashSet;

use std::hash::Hash;

pub struct EventDiff {
    indexed_calendars:   Option<UpdatedSetMembers<String>>,
    indexed_categories:  Option<UpdatedSetMembers<String>>,
    indexed_related_to:  Option<UpdatedSetMembers<(String, String)>>,

    passive_properties:  Option<UpdatedSetMembers<(String, String)>>,
    schedule_properties: Option<SchedulePropertiesDiff>,
}

impl EventDiff {

    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        let mut event_diff = EventDiff {
            indexed_calendars:   None,
            indexed_categories:  None,
            indexed_related_to:  None,

            passive_properties:  None,
            schedule_properties: None,
        };

        event_diff.diff_indexed_calendars(original_event, updated_event);
        event_diff.diff_indexed_categories(original_event, updated_event);
        event_diff.diff_indexed_related_to(original_event, updated_event);
        event_diff.diff_passive_properties(original_event, updated_event);
        event_diff.diff_schedule_properties(original_event, updated_event);

        event_diff
    }

    fn diff_indexed_calendars(&mut self, original_event: &Event, updated_event: &Event) {
        self.indexed_calendars = Some(
            UpdatedSetMembers::new(
                original_event.indexed_properties.get_indexed_calendars().as_ref(),
                updated_event.indexed_properties.get_indexed_calendars().as_ref()
            )
        );
    }

    fn diff_indexed_categories(&mut self, original_event: &Event, updated_event: &Event) {
        self.indexed_categories = Some(
            UpdatedSetMembers::new(
                original_event.indexed_properties.categories.as_ref(),
                updated_event.indexed_properties.categories.as_ref()
            )
        );
    }

    fn diff_indexed_related_to(&mut self, original_event: &Event, updated_event: &Event) {
        let original_related_to = hashmap_to_hashset(original_event.indexed_properties.related_to.as_ref());
        let updated_related_to = hashmap_to_hashset(updated_event.indexed_properties.related_to.as_ref());

        self.indexed_related_to = Some(
            UpdatedSetMembers::new(
                original_related_to.as_ref(),
                updated_related_to.as_ref()
            )
        );
    }


    fn diff_passive_properties(&mut self, original_event: &Event, updated_event: &Event) {
        // TODO: Improve this to be 0 copy
        let original_passive_properties = hashmap_to_hashset(Some(&original_event.passive_properties.properties));
        let updated_passive_properties = hashmap_to_hashset(Some(&updated_event.passive_properties.properties));

        self.passive_properties = Some(
            UpdatedSetMembers::new(
                original_passive_properties.as_ref(),
                updated_passive_properties.as_ref()
            )
        );
    }

    fn diff_schedule_properties(&mut self, original_event: &Event, updated_event: &Event) {
        self.schedule_properties = Some(
            SchedulePropertiesDiff::new(original_event, updated_event)
        );
    }

}

pub struct SchedulePropertiesDiff {
    rrule:    Option<UpdatedSetMembers<String>>,
    exrule:   Option<UpdatedSetMembers<String>>,
    rdate:    Option<UpdatedSetMembers<String>>,
    exdate:   Option<UpdatedSetMembers<String>>,
    duration: Option<UpdatedSetMembers<String>>,
    dtstart:  Option<UpdatedSetMembers<String>>,
    dtend:    Option<UpdatedSetMembers<String>>,
}

impl SchedulePropertiesDiff {
    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        let original_event_schedule_properties = &original_event.schedule_properties;
        let updated_event_schedule_properties  = &updated_event.schedule_properties;

        SchedulePropertiesDiff {
            rrule:    Self::build_updated_set_members(original_event_schedule_properties.rrule.as_ref(),    updated_event_schedule_properties.rrule.as_ref()),
            exrule:   Self::build_updated_set_members(original_event_schedule_properties.exrule.as_ref(),   updated_event_schedule_properties.exrule.as_ref()),
            rdate:    Self::build_updated_set_members(original_event_schedule_properties.rdate.as_ref(),    updated_event_schedule_properties.rdate.as_ref()),
            exdate:   Self::build_updated_set_members(original_event_schedule_properties.exdate.as_ref(),   updated_event_schedule_properties.exdate.as_ref()),
            duration: Self::build_updated_set_members(original_event_schedule_properties.duration.as_ref(), updated_event_schedule_properties.duration.as_ref()),
            dtstart:  Self::build_updated_set_members(original_event_schedule_properties.dtstart.as_ref(),  updated_event_schedule_properties.dtstart.as_ref()),
            dtend:    Self::build_updated_set_members(original_event_schedule_properties.dtend.as_ref(),    updated_event_schedule_properties.dtend.as_ref()),
        }
    }

    pub fn get_rebuild_consensus(&self) -> RebuildConsensus {
        fn property_has_changed(property: Option<&UpdatedSetMembers<String>>) -> bool {
            property.is_some_and(|property| property.is_changed())
        }

        #[allow(unused_parens)]
        if (
            property_has_changed(self.rrule.as_ref())  ||
            property_has_changed(self.exrule.as_ref()) ||
            property_has_changed(self.rdate.as_ref())  ||
            property_has_changed(self.exdate.as_ref()) ||
            property_has_changed(self.dtstart.as_ref())
        ) {
            // TODO: handle more granular changes yielding RebuildConsensus::Partial for partial
            // updated occurrence extrapolation.
            RebuildConsensus::Full
        } else {
            RebuildConsensus::None
        }
    }

    fn build_updated_set_members<T>(original_set: Option<&HashSet<T>>, updated_set: Option<&HashSet<T>>) -> Option<UpdatedSetMembers<T>>
    where
        T: Eq + PartialEq + Hash + Clone
    {
        match (original_set, updated_set) {
            (None, None) => None,

            _ => Some(UpdatedSetMembers::new(original_set, updated_set))
        }
    }
}

pub enum RebuildConsensus {
    None,
    Full,
    Partial,
}
