use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq)]
pub enum OccurrenceIndexValue {
    Occurrence(i32),
    Occurrences(BTreeSet<i32>)
}

#[derive(Debug, PartialEq)]
pub enum TimeWindowInterval {
    Hourly,
    Daily,
    Weekly
}

impl TimeWindowInterval {

    pub fn window_start_epoch(&self, occurrence: i64) -> (i64, i32) {
        match self {
            TimeWindowInterval::Hourly => Self::window_start_epoch_hourly(occurrence),
            TimeWindowInterval::Daily  => Self::window_start_epoch_daily(occurrence),
            TimeWindowInterval::Weekly => Self::window_start_epoch_weekly(occurrence),
        }
    }

    fn window_start_epoch_hourly(occurrence: i64) -> (i64, i32) {
        let offset = occurrence % (60 * 60);

        (
            occurrence - offset,
            offset as i32
        )
    }

    fn window_start_epoch_daily(occurrence: i64) -> (i64, i32) {
        let offset = occurrence % (24 * (60 * 60));

        (
            occurrence - offset,
            offset as i32
        )
    }

    fn window_start_epoch_weekly(occurrence: i64) -> (i64, i32) {
        // As epoch 0 is Thursday 1st January 1970 and the week start is Monday, we
        // need to offset the calculation by 4 days.
        let week_start_offset = (24 * (60 * 60)) * 4;

        let offset = (occurrence - week_start_offset) % (7 * (24 * (60 * 60)));

        (
            occurrence - offset,
            offset as i32
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct OccurrenceIndex<'a> {
    pub time_window_interval: &'a TimeWindowInterval,
    pub time_windows: BTreeMap<i64, OccurrenceIndexValue>
}

impl<'a> OccurrenceIndex<'a> {
    pub fn new(time_window_interval: &TimeWindowInterval) -> OccurrenceIndex {
        OccurrenceIndex {
            time_window_interval,
            time_windows: BTreeMap::new()
        }
    }

    pub fn insert(&mut self, occurrence: i64) {
        let (window_start_epoch, window_start_epoch_mod) = self.time_window_interval.window_start_epoch(occurrence);

        match self.time_windows.get_mut(&window_start_epoch) {
            Some(OccurrenceIndexValue::Occurrence(occurrence)) => {
                let mut inserted_occurrences: BTreeSet<i32> = BTreeSet::new();

                inserted_occurrences.insert(*occurrence);
                inserted_occurrences.insert(window_start_epoch_mod);

                self.time_windows.insert(window_start_epoch, OccurrenceIndexValue::Occurrences(inserted_occurrences));
            },
            Some(OccurrenceIndexValue::Occurrences(occurrences)) => {
                occurrences.insert(window_start_epoch_mod);
            }
            None => {
                self.time_windows.insert(window_start_epoch, OccurrenceIndexValue::Occurrence(window_start_epoch_mod));
            }
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn test_time_window_interval() {
        let current_epoch = 1686938560; // Fri Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            TimeWindowInterval::Hourly.window_start_epoch(current_epoch),
            (
                1686938400, // Fri Jun 16 2023 18:00:00 GMT+0000
                160         // 2 Minutes 40 Seconds
            )
        );

        assert_eq!(
            TimeWindowInterval::Daily.window_start_epoch(current_epoch),
            (
                1686873600, // Fri Jun 16 2023 00:00:00 GMT+0000
                64960       // 18 Hours 2 Minutes 40 Seconds
            )
        );

        assert_eq!(
            TimeWindowInterval::Weekly.window_start_epoch(current_epoch),
            (
                1686528000, // Mon Jun 12 2023 00:00:00 GMT+0000
                410560      // 4 Days 18 Hours 2 Minutes 40 Seconds
            )
        );
    }

    #[test]
    fn test_occurrence_index_new() {
        vec![
            TimeWindowInterval::Hourly,
            TimeWindowInterval::Daily,
            TimeWindowInterval::Weekly
        ].iter()
         .for_each(|time_window_interval| {
             assert_eq!(
                 OccurrenceIndex::new(time_window_interval),
                 OccurrenceIndex {
                     time_window_interval: time_window_interval,
                     time_windows: BTreeMap::new()
                 }
             );
         });
    }

    #[test]
    fn test_occurrence_index_insert() {
        let mut occurrence_index = OccurrenceIndex::new(&TimeWindowInterval::Daily);

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                time_window_interval: &TimeWindowInterval::Daily,
                time_windows: BTreeMap::from([])
            }
        );

        occurrence_index.insert(1686938560); // Fri Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                time_window_interval: &TimeWindowInterval::Daily,
                time_windows: BTreeMap::from(
                    [
                        (
                            1686873600, // Fri Jun 16 2023 00:00:00 GMT+0000
                            OccurrenceIndexValue::Occurrence(
                                64960 // 18 Hours 2 Minutes 40 Seconds
                            )
                        )
                    ]
                )
            }
        );

        occurrence_index.insert(1686949960); // Fri Jun 16 2023 21:12:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                time_window_interval: &TimeWindowInterval::Daily,
                time_windows: BTreeMap::from(
                    [
                        (
                            1686873600, // Fri Jun 16 2023 00:00:00 GMT+0000
                            OccurrenceIndexValue::Occurrences(
                                BTreeSet::from(
                                    [
                                        64960, // 18 Hours 2 Minutes 40 Seconds
                                        76360 // 21 Hours 12 Minutes 40 Seconds
                                    ]
                                )
                            )
                        )
                    ]
                )
            }
        );

        occurrence_index.insert(1687068620); // Sun Jun 18 2023 06:10:20 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                time_window_interval: &TimeWindowInterval::Daily,
                time_windows: BTreeMap::from(
                    [
                        (
                            1686873600, // Fri Jun 16 2023 00:00:00 GMT+0000
                            OccurrenceIndexValue::Occurrences(
                                BTreeSet::from(
                                    [
                                        64960, // 18 Hours 2 Minutes 40 Seconds
                                        76360 // 21 Hours 12 Minutes 40 Seconds
                                    ]
                                )
                            )
                        ),
                        (
                            1687046400, // Sun Jun 18 2023 00:00:00 GMT+0000
                            OccurrenceIndexValue::Occurrence(
                                22220 // 6 Hours 10 Minutes 20 Seconds
                            )
                        )
                    ]
                )
            }
        );

    }
}
