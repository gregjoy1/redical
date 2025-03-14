use geo::{HaversineDistance, Point};
use rstar::{PointDistance, RTree, RTreeObject};
use std::cmp::Ordering;

use rstar::primitives::GeomWithData;

use std::hash::{Hash, Hasher};

use crate::{IndexedConclusion, InvertedCalendarIndexTerm};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum GeoDistance {
    Kilometers((u32, u32)), // (km, fractional (6dp))
    Miles((u32, u32)),      // (ml, fractional (6dp))
}

impl std::fmt::Display for GeoDistance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeoDistance::Kilometers(_) => write!(f, "{}KM", self.to_kilometers_float()),
            GeoDistance::Miles(_) => write!(f, "{}MI", self.to_miles_float()),
        }
    }
}

impl GeoDistance {
    const FRACTIONAL_PREC: f64 = 1000000.0_f64;
    const KM_TO_MILE: f64 = 1.609344_f64;
    const MILE_TO_KM: f64 = 0.621371_f64;

    pub fn to_meters_float(&self) -> f64 {
        match self {
            GeoDistance::Kilometers((km_int, fractional_int)) => {
                Self::int_fractional_tuple_to_float((km_int, fractional_int)) * 1000f64
            }

            GeoDistance::Miles(_) => self.to_kilometers().to_meters_float(),
        }
    }

    pub fn to_kilometers_float(&self) -> f64 {
        match self {
            GeoDistance::Kilometers((km_int, fractional_int)) => {
                Self::int_fractional_tuple_to_float((km_int, fractional_int))
            }

            GeoDistance::Miles(_) => self.to_kilometers().to_kilometers_float(),
        }
    }

    pub fn to_miles_float(&self) -> f64 {
        match self {
            GeoDistance::Kilometers(_) => self.to_miles().to_miles_float(),

            GeoDistance::Miles((mile_int, fractional_int)) => {
                Self::int_fractional_tuple_to_float((mile_int, fractional_int))
            }
        }
    }

    pub fn new_from_meters_float(mt_float: f64) -> Self {
        GeoDistance::Kilometers(Self::float_to_int_fractional_tuple(&(mt_float / 1000_f64)))
    }

    pub fn new_from_kilometers_float(km_float: f64) -> Self {
        GeoDistance::Kilometers(Self::float_to_int_fractional_tuple(&km_float))
    }

    pub fn new_from_miles_float(mile_float: f64) -> Self {
        GeoDistance::Miles(Self::float_to_int_fractional_tuple(&mile_float))
    }

    pub fn to_kilometers(&self) -> Self {
        match self {
            GeoDistance::Kilometers(_) => self.clone(),

            GeoDistance::Miles((mile_int, fractional_int)) => {
                let mile_float = Self::int_fractional_tuple_to_float((mile_int, fractional_int));

                Self::new_from_kilometers_float(mile_float * Self::KM_TO_MILE)
            }
        }
    }

    pub fn to_miles(&self) -> Self {
        match self {
            GeoDistance::Kilometers((km_int, fractional_int)) => {
                let km_float = Self::int_fractional_tuple_to_float((km_int, fractional_int));

                Self::new_from_miles_float(km_float * Self::MILE_TO_KM)
            }

            GeoDistance::Miles((_miles, _ft)) => self.clone(),
        }
    }

    fn int_fractional_tuple_to_float(int_fractional_tuple: (&u32, &u32)) -> f64 {
        let whole_int_float: f64 = num::cast(int_fractional_tuple.0.to_owned()).unwrap();
        let fractional_int_float: f64 = num::cast(int_fractional_tuple.1.to_owned()).unwrap();

        whole_int_float + (fractional_int_float / Self::FRACTIONAL_PREC)
    }

    fn float_to_int_fractional_tuple(value_float: &f64) -> (u32, u32) {
        let mut value_float = value_float.abs();

        if !value_float.is_normal() {
            value_float = 0f64;
        };

        let whole_int: u32 = num::cast(value_float.trunc()).unwrap();
        let fractional_int: u32 = num::cast(value_float.fract() * Self::FRACTIONAL_PREC).unwrap();

        (whole_int, fractional_int)
    }
}

impl PartialOrd for GeoDistance {
    fn partial_cmp(&self, other: &GeoDistance) -> Option<Ordering> {
       Some(self.cmp(other))
    }
}

impl Ord for GeoDistance {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.to_kilometers(), other.to_kilometers()) {
            (
                GeoDistance::Kilometers((self_km_int, self_fractional_int)),
                GeoDistance::Kilometers((other_km_int, other_fractional_int)),
            ) => {
                let ordering = self_km_int.cmp(&other_km_int);

                if ordering.is_eq() {
                    self_fractional_int.cmp(&other_fractional_int)
                } else {
                    ordering
                }
            }

            _ => {
                panic!("Expected both self and other GeoDistance to convert to GeoDistance::Kilometers enum.");
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeoPoint {
    pub lat: f64,
    pub long: f64,
}

impl TryFrom<Option<(f64, f64)>> for GeoPoint {
    type Error = String;

    fn try_from(coords: Option<(f64, f64)>) -> Result<Self, Self::Error> {
        if let Some((latitude, longitude)) = coords {
            Ok(
                GeoPoint::new(
                    latitude,
                    longitude,
                )
            )
        } else {
            Err(
                String::from("Cannot build blank GeoPoint")
            )
        }
    }
}

impl From<(f64, f64)> for GeoPoint {
    #[inline]
    fn from(coords: (f64, f64)) -> Self {
        GeoPoint::new(coords.0, coords.1)
    }
}

impl From<GeoPoint> for Point {
    #[inline]
    fn from(geo_point: GeoPoint) -> Self {
        geo_point.to_point()
    }
}

impl GeoPoint {
    pub fn new(lat: f64, long: f64) -> Self {
        GeoPoint { lat, long }
    }

    pub fn to_point(&self) -> Point {
        // Long -> x
        // Lat  -> y
        Point::new(self.long, self.lat)
    }

    pub fn validate(&self) -> Result<&Self, String> {
        if self.lat < -90_f64 || self.lat > 90_f64 {
            return Err(format!(
                "Expected latitude: {} to be greater than -90 and less than 90.",
                self.lat
            ));
        }

        if self.long < -180_f64 || self.long > 180_f64 {
            return Err(format!(
                "Expected latitude: {} to be greater than -180 and less than 180.",
                self.long
            ));
        }

        Ok(self)
    }

    pub fn geohash(&self) -> Result<String, String> {
        geohash::encode(
            geohash::Coord {
                x: self.long,
                y: self.lat,
            },
            12, // Accurrate to 37.2mm × 18.6mm
        )
        .map_err(|geohash_error| geohash_error.to_string())
    }
}

impl std::fmt::Display for GeoPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{}", self.lat, self.long)
    }
}

impl RTreeObject for GeoPoint {
    type Envelope = <geo::Point as RTreeObject>::Envelope;

    fn envelope(&self) -> Self::Envelope {
        self.to_point().envelope()
    }
}

impl PointDistance for GeoPoint {
    fn distance_2(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
    ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
        self.to_point().haversine_distance(point)
    }

    fn contains_point(&self, point: &<Self::Envelope as rstar::Envelope>::Point) -> bool {
        self.to_point().contains_point(point)
    }

    fn distance_2_if_less_or_equal(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
        max_distance_2: <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar,
    ) -> Option<<<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar> {
        let distance = self.distance_2(point);

        if distance <= max_distance_2 {
            Some(distance)
        } else {
            None
        }
    }
}

impl HaversineDistance<f64> for GeoPoint {
    fn haversine_distance(&self, rhs: &GeoPoint) -> f64 {
        self.to_point().haversine_distance(&rhs.to_point())
    }
}

impl Hash for GeoPoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.geohash().hash(state);
    }
}

impl PartialEq for GeoPoint {
    fn eq(&self, other: &Self) -> bool {
        self.geohash() == other.geohash()
    }
}

impl Eq for GeoPoint {}

// Multi layer inverted index (for multiple events) - indexed term - event - include/exclude
//#[derive(Debug, PartialEq, Clone)]
#[derive(Debug, Clone)]
pub struct GeoSpatialCalendarIndex {
    pub coords: RTree<GeomWithData<GeoPoint, InvertedCalendarIndexTerm>>,
}

impl PartialEq for GeoSpatialCalendarIndex {
    fn eq(&self, other: &GeoSpatialCalendarIndex) -> bool {
        let self_iter = self.coords.into_iter();
        let other_iter = other.coords.into_iter();

        for (self_element, other_element) in self_iter.zip(other_iter) {
            if self_element != other_element {
                return false;
            }
        }

        true
    }
}

impl Default for GeoSpatialCalendarIndex {
    fn default() -> Self {
        GeoSpatialCalendarIndex {
            coords: RTree::new(),
        }
    }
}

impl GeoSpatialCalendarIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn locate_within_distance(
        &self,
        long_lat: &GeoPoint,
        distance: &GeoDistance,
    ) -> InvertedCalendarIndexTerm {
        let mut result_inverted_index_term = InvertedCalendarIndexTerm::new();

        for indexed_coord in self
            .coords
            .locate_within_distance(long_lat.to_point(), distance.to_meters_float())
        {
            result_inverted_index_term = InvertedCalendarIndexTerm::merge_or(
                &result_inverted_index_term,
                &indexed_coord.data,
            );
        }

        result_inverted_index_term
    }

    /// As there may be other events in the calendar outside those indexed here, a vector of
    /// all the event uids contained in the calendar must be passed so that they can be referenced
    /// in the negated event set, as by design they will not match the given term.
    ///
    /// The negated virtual index is formed by building an index of full inclusions of all events
    /// in the calendar and then merging in the inverse of the event set of the given term.
    pub fn locate_not_within_distance(
        &self,
        long_lat: &GeoPoint,
        distance: &GeoDistance,
        calendar_event_uids: &[String],
    ) -> InvertedCalendarIndexTerm {
        let mut negated_event_set = InvertedCalendarIndexTerm::new();

        for event_uid in calendar_event_uids.iter().cloned() {
            negated_event_set.insert_included_event(event_uid, None);
        }

        let outside_distance_event_set = self
            .locate_within_distance(long_lat, distance)
            .inverse();

        for (event_uid, indexed_conclusion) in outside_distance_event_set.events {
            if indexed_conclusion.is_empty_exclude() {
                negated_event_set.events.remove(&event_uid);
            } else {
                negated_event_set.events.insert(event_uid, indexed_conclusion);
            }
        }

        negated_event_set
    }

    pub fn insert(
        &mut self,
        event_uid: String,
        long_lat: &GeoPoint,
        indexed_conclusion: &IndexedConclusion,
    ) -> Result<&mut Self, String> {
        match self.coords.locate_at_point_mut(&long_lat.to_point()) {
            Some(existing_result) => {
                match indexed_conclusion {
                    IndexedConclusion::Include(exceptions) => existing_result
                        .data
                        .insert_included_event(event_uid.clone(), exceptions.clone()),
                    IndexedConclusion::Exclude(exceptions) => existing_result
                        .data
                        .insert_excluded_event(event_uid.clone(), exceptions.clone()),
                };
            }

            None => {
                self.coords.insert(GeomWithData::new(
                    long_lat.clone(),
                    InvertedCalendarIndexTerm::new_with_event(
                        event_uid.clone(),
                        indexed_conclusion.clone(),
                    ),
                ));
            }
        }

        Ok(self)
    }

    pub fn remove(&mut self, event_uid: String, long_lat: &GeoPoint) -> Result<&mut Self, String> {
        if let Some(existing_result) = self.coords.locate_at_point_mut(&long_lat.to_point()) {
            if existing_result
                .data
                .remove_event(event_uid)
                .is_ok_and(|inverted_calendar_index_term| inverted_calendar_index_term.is_empty())
            {
                self.coords.remove_at_point(&long_lat.to_point());
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{HashMap, HashSet};

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    const RANDOM: GeoPoint = GeoPoint { lat: 51.7854972_f64, long: -1.4701705_f64 };
    const RANDOM_PLUS_OFFSET: GeoPoint = GeoPoint { lat: 51.785341_f64, long: -1.470240_f64 };
    const NEW_YORK_CITY: GeoPoint = GeoPoint { lat: 40.7128_f64, long: -74.006_f64 };
    const CHURCHDOWN: GeoPoint = GeoPoint { lat: 51.8773_f64, long: -2.1686_f64 };
    const LONDON: GeoPoint = GeoPoint { lat: 51.5074_f64, long: -0.1278_f64 };
    const OXFORD: GeoPoint = GeoPoint { lat: 51.8773_f64, long: -1.2475878_f64 };

    fn example_geo_index() -> GeoSpatialCalendarIndex {
        let mut geo_spatial_calendar_index = GeoSpatialCalendarIndex::new();

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_event_uid"),
                &RANDOM,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_and_churchdown_event_uid"),
                &RANDOM,
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_plus_offset_event_uid"),
                &RANDOM_PLUS_OFFSET,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_plus_offset_and_london_event_uid"),
                &RANDOM_PLUS_OFFSET,
                &IndexedConclusion::Exclude(Some(HashSet::from([100]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("churchdown_event_uid"),
                &CHURCHDOWN,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_and_churchdown_event_uid"),
                &CHURCHDOWN,
                &IndexedConclusion::Exclude(Some(HashSet::from([200, 300]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("oxford_event_one_uid"),
                &OXFORD,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("oxford_event_two_uid"),
                &OXFORD,
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("london_event_uid"),
                &LONDON,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_plus_offset_and_london_event_uid"),
                &LONDON,
                &IndexedConclusion::Include(Some(HashSet::from([100]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("new_york_city_event_uid"),
                &NEW_YORK_CITY,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        geo_spatial_calendar_index
    }

    #[test]
    fn test_geo_spatial_calendar_index() {
        let mut geo_spatial_calendar_index = GeoSpatialCalendarIndex::new();

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::new(),
            }
        );

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("london_event_uid_one"),
                &LONDON,
                &IndexedConclusion::Include(None)
            )
            .is_ok(),);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    LONDON.clone(),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([(
                            String::from("london_event_uid_one"),
                            IndexedConclusion::Include(None)
                        )])
                    }
                )])
            }
        );

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("london_event_uid_two"),
                &LONDON,
                &IndexedConclusion::Exclude(Some(HashSet::from([100])))
            )
            .is_ok(),);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    LONDON.clone(),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([
                            (
                                String::from("london_event_uid_one"),
                                IndexedConclusion::Include(None)
                            ),
                            (
                                String::from("london_event_uid_two"),
                                IndexedConclusion::Exclude(Some(HashSet::from([100])))
                            ),
                        ])
                    }
                )])
            }
        );

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("oxford_event_uid"),
                &OXFORD,
                &IndexedConclusion::Include(Some(HashSet::from([100])))
            )
            .is_ok(),);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![
                    GeomWithData::new(
                        LONDON.clone(),
                        InvertedCalendarIndexTerm {
                            events: HashMap::from([
                                (
                                    String::from("london_event_uid_one"),
                                    IndexedConclusion::Include(None)
                                ),
                                (
                                    String::from("london_event_uid_two"),
                                    IndexedConclusion::Exclude(Some(HashSet::from([100])))
                                ),
                            ])
                        }
                    ),
                    GeomWithData::new(
                        OXFORD.clone(),
                        InvertedCalendarIndexTerm {
                            events: HashMap::from([(
                                String::from("oxford_event_uid"),
                                IndexedConclusion::Include(Some(HashSet::from([100])))
                            ),])
                        }
                    )
                ])
            }
        );

        assert!(geo_spatial_calendar_index
            .remove(String::from("oxford_event_uid"), &OXFORD)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    LONDON.clone(),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([
                            (
                                String::from("london_event_uid_one"),
                                IndexedConclusion::Include(None)
                            ),
                            (
                                String::from("london_event_uid_two"),
                                IndexedConclusion::Exclude(Some(HashSet::from([100])))
                            ),
                        ])
                    }
                )])
            }
        );

        assert!(geo_spatial_calendar_index
            .remove(String::from("london_event_uid_one"), &LONDON)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    LONDON.clone(),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([(
                            String::from("london_event_uid_two"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100])))
                        ),])
                    }
                )])
            }
        );

        assert!(geo_spatial_calendar_index
            .remove(String::from("london_event_uid_one"), &LONDON)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    LONDON.clone(),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([(
                            String::from("london_event_uid_two"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100])))
                        ),])
                    }
                )])
            }
        );

        assert!(geo_spatial_calendar_index
            .remove(String::from("london_event_uid_two"), &LONDON)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![])
            }
        );
    }

    #[test]
    fn test_geo_spatial_calendar_index_locate_within_distance() {
        let geo_spatial_calendar_index = example_geo_index();

        assert_eq_sorted!(
            geo_spatial_calendar_index
                .locate_within_distance(&OXFORD, &GeoDistance::new_from_kilometers_float(1.0_f64)),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("oxford_event_one_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("oxford_event_two_uid"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                ])
            }
        );

        assert_eq_sorted!(
            geo_spatial_calendar_index
                .locate_within_distance(&OXFORD, &GeoDistance::new_from_kilometers_float(87.0_f64)),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("churchdown_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("oxford_event_one_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("oxford_event_two_uid"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("random_and_churchdown_event_uid"),
                        IndexedConclusion::Include(Some(HashSet::from([100])))
                    ),
                    (
                        String::from("random_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("random_plus_offset_and_london_event_uid"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100])))
                    ),
                    (
                        String::from("random_plus_offset_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );

        assert_eq_sorted!(
            geo_spatial_calendar_index
                .locate_within_distance(&OXFORD, &GeoDistance::new_from_kilometers_float(87.5_f64)),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("churchdown_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("oxford_event_one_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("oxford_event_two_uid"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("random_and_churchdown_event_uid"),
                        IndexedConclusion::Include(Some(HashSet::from([100])))
                    ),
                    (
                        String::from("random_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("random_plus_offset_and_london_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("random_plus_offset_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("london_event_uid"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_geo_spatial_calendar_index_locate_not_within_distance() {
        let geo_spatial_calendar_index = example_geo_index();

        // Contains some event uids not included in the target index to mimic events indexed
        // elsewhere in the calendar (e.g. another index).
        let calendar_event_uids = vec![
            String::from("random_event_uid"),
            String::from("random_and_churchdown_event_uid"),
            String::from("random_plus_offset_event_uid"),
            String::from("random_plus_offset_and_london_event_uid"),
            String::from("churchdown_event_uid"),
            String::from("random_and_churchdown_event_uid"),
            String::from("oxford_event_one_uid"),
            String::from("oxford_event_two_uid"),
            String::from("london_event_uid"),
            String::from("random_plus_offset_and_london_event_uid"),
            String::from("new_york_city_event_uid"),
            String::from("unknown_location_event_uid_1"),
            String::from("unknown_location_event_uid_2"),
        ];

        assert_eq_sorted!(
            geo_spatial_calendar_index.locate_not_within_distance(
                &OXFORD,
                &GeoDistance::new_from_kilometers_float(1.0_f64),
                &calendar_event_uids,
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("churchdown_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("london_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("new_york_city_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("random_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("random_and_churchdown_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("random_plus_offset_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("random_plus_offset_and_london_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("oxford_event_two_uid"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("unknown_location_event_uid_1"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("unknown_location_event_uid_2"),
                        IndexedConclusion::Include(None),
                    ),
                ])
            }
        );

        assert_eq_sorted!(
            geo_spatial_calendar_index.locate_not_within_distance(
                &OXFORD,
                &GeoDistance::new_from_kilometers_float(87.0_f64),
                &calendar_event_uids,
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("london_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("new_york_city_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("oxford_event_two_uid"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("random_and_churchdown_event_uid"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100])))
                    ),
                    (
                        String::from("random_plus_offset_and_london_event_uid"),
                        IndexedConclusion::Include(Some(HashSet::from([100])))
                    ),
                    (
                        String::from("unknown_location_event_uid_1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("unknown_location_event_uid_2"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );

        assert_eq_sorted!(
            geo_spatial_calendar_index.locate_not_within_distance(
                &OXFORD,
                &GeoDistance::new_from_kilometers_float(87.5_f64),
                &calendar_event_uids,
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("new_york_city_event_uid"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("oxford_event_two_uid"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("random_and_churchdown_event_uid"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100])))
                    ),
                    (
                        String::from("unknown_location_event_uid_1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("unknown_location_event_uid_2"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    #[allow(clippy::excessive_precision)]
    fn test_geo_distance_rtree() {
        let mut tree = RTree::new();

        tree.insert(RANDOM.clone());
        tree.insert(RANDOM_PLUS_OFFSET.clone());
        tree.insert(NEW_YORK_CITY.clone());
        tree.insert(CHURCHDOWN.clone());
        tree.insert(LONDON.clone());

        let mut results = tree.nearest_neighbor_iter_with_distance_2(&OXFORD.to_point().clone());

        let (point, distance) = results.next().unwrap();

        // Cast all f64 distances to f32 so tests pass under both MacOS and Linux.
        assert_eq!((point, distance as f32), (&RANDOM, 18388.597009683246_f32));

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point, distance),
            (&RANDOM_PLUS_OFFSET, 18402.23696221235_f64)
        );

        let (point, distance) = results.next().unwrap();

        // Cast all f64 distances to f32 so tests pass under both MacOS and Linux.
        assert_eq!((point, distance as f32), (&CHURCHDOWN, 63223.39709694925_f32));

        let (point, distance) = results.next().unwrap();

        assert_eq!((point, distance), (&LONDON, 87458.64969073102_f64));

        let (point, distance) = results.next().unwrap();

        // Cast all f64 distances to f32 so tests pass under both MacOS and Linux.
        assert_eq!((point, distance as f32), (&NEW_YORK_CITY, 5484158.985172745_f32));

        assert_eq!(results.next(), None);

        let results: Vec<&GeoPoint> = tree
            .locate_within_distance(OXFORD.to_point(), 65000.0_f64)
            .collect();

        assert_eq_sorted!(results, vec![&CHURCHDOWN, &RANDOM_PLUS_OFFSET, &RANDOM]);
    }

    #[test]
    fn test_geo_distance_enum() {
        assert_eq!(
            GeoDistance::new_from_kilometers_float(4321.123456789_f64),
            GeoDistance::Kilometers((4321_u32, 123456_u32)),
        );

        assert_eq!(
            GeoDistance::new_from_meters_float(4321.123456789_f64),
            GeoDistance::Kilometers((4_u32, 321123_u32)),
        );

        assert_eq!(
            GeoDistance::new_from_miles_float(4321.123456789_f64),
            GeoDistance::Miles((4321_u32, 123456_u32)),
        );

        assert_eq!(
            GeoDistance::new_from_kilometers_float(-4321.123456789_f64),
            GeoDistance::Kilometers((4321_u32, 123456_u32)),
        );

        assert_eq!(
            GeoDistance::new_from_miles_float(f64::NAN),
            GeoDistance::Miles((0_u32, 0_u32)),
        );

        assert_eq!(
            GeoDistance::new_from_kilometers_float(1.5),
            GeoDistance::Kilometers((1_u32, 500000_u32)),
        );

        let one_and_a_half_km = GeoDistance::new_from_kilometers_float(1.5);

        assert_eq!(one_and_a_half_km.to_kilometers_float(), 1.5);
        assert_eq!(one_and_a_half_km.to_miles_float(), 0.932056);
        assert_eq!(
            one_and_a_half_km.to_miles(),
            GeoDistance::Miles((0_u32, 932056_u32))
        );
        assert_eq!(one_and_a_half_km.to_string(), String::from("1.5KM"));

        let one_and_a_half_miles = GeoDistance::new_from_miles_float(1.5);

        assert_eq!(one_and_a_half_miles.to_miles_float(), 1.5);
        assert_eq!(one_and_a_half_miles.to_kilometers_float(), 2.414016);
        assert_eq!(
            one_and_a_half_miles.to_kilometers(),
            GeoDistance::Kilometers((2_u32, 414016_u32))
        );
        assert_eq!(one_and_a_half_miles.to_string(), String::from("1.5MI"));
    }
}
