use geo::{HaversineDistance, Point};
use rstar::{PointDistance, RTree, RTreeObject};
use std::cmp::Ordering;

use rstar::primitives::GeomWithData;

use std::hash::{Hash, Hasher};

use crate::{IndexedConclusion, InvertedCalendarIndexTerm};
use redical_ical::properties::ICalendarGeoProperty;

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

impl<P> From<&P> for GeoPoint
where
    P: ICalendarGeoProperty,
{
    #[inline]
    fn from(property: &P) -> Self {
        GeoPoint::new(
            property.get_latitude(),
            property.get_longitude(),
        )
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

    #[test]
    fn test_geo_spatial_calendar_index() {
        let mut geo_spatial_calendar_index = GeoSpatialCalendarIndex::new();

        let london = GeoPoint::new(51.5074_f64, -0.1278_f64);
        let oxford = GeoPoint::new(51.8773_f64, -1.2475878_f64);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::new(),
            }
        );

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("london_event_uid_one"),
                &london,
                &IndexedConclusion::Include(None)
            )
            .is_ok(),);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    london.clone(),
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
                &london,
                &IndexedConclusion::Exclude(Some(HashSet::from([100])))
            )
            .is_ok(),);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    london.clone(),
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
                &oxford,
                &IndexedConclusion::Include(Some(HashSet::from([100])))
            )
            .is_ok(),);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![
                    GeomWithData::new(
                        london.clone(),
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
                        oxford.clone(),
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
            .remove(String::from("oxford_event_uid"), &oxford)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    london.clone(),
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
            .remove(String::from("london_event_uid_one"), &london)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    london.clone(),
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
            .remove(String::from("london_event_uid_one"), &london)
            .is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![GeomWithData::new(
                    london.clone(),
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
            .remove(String::from("london_event_uid_two"), &london)
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
        let mut geo_spatial_calendar_index = GeoSpatialCalendarIndex::new();

        let random = GeoPoint::new(51.7854972_f64, -1.4701705_f64);
        let random_plus_offset = GeoPoint::new(51.785341_f64, -1.470240_f64);
        let new_york_city = GeoPoint::new(40.7128_f64, -74.006_f64);
        let churchdown = GeoPoint::new(51.8773_f64, -2.1686_f64);
        let london = GeoPoint::new(51.5074_f64, -0.1278_f64);
        let oxford = GeoPoint::new(51.8773_f64, -1.2475878_f64);

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_event_uid"),
                &random,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_and_churchdown_event_uid"),
                &random,
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_plus_offset_event_uid"),
                &random_plus_offset,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_plus_offset_and_london_event_uid"),
                &random_plus_offset,
                &IndexedConclusion::Exclude(Some(HashSet::from([100]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("churchdown_event_uid"),
                &churchdown,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_and_churchdown_event_uid"),
                &churchdown,
                &IndexedConclusion::Exclude(Some(HashSet::from([200, 300]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("oxford_event_one_uid"),
                &oxford,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("oxford_event_two_uid"),
                &oxford,
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("london_event_uid"),
                &london,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("random_plus_offset_and_london_event_uid"),
                &random_plus_offset,
                &IndexedConclusion::Include(Some(HashSet::from([100]))),
            )
            .is_ok());

        assert!(geo_spatial_calendar_index
            .insert(
                String::from("new_york_city_event_uid"),
                &new_york_city,
                &IndexedConclusion::Include(None),
            )
            .is_ok());

        assert_eq_sorted!(
            geo_spatial_calendar_index
                .locate_within_distance(&oxford, &GeoDistance::new_from_kilometers_float(1.0_f64)),
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
                .locate_within_distance(&oxford, &GeoDistance::new_from_kilometers_float(87.0_f64)),
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
                        IndexedConclusion::Include(Some(HashSet::from([100])))
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
                .locate_within_distance(&oxford, &GeoDistance::new_from_kilometers_float(87.5_f64)),
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
                        IndexedConclusion::Include(Some(HashSet::from([100])))
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
    #[allow(clippy::excessive_precision)]
    fn test_geo_distance_rtree() {
        let mut tree = RTree::new();

        let random = GeoPoint::new(51.7854972_f64, -1.4701705_f64);
        let random_plus_offset = GeoPoint::new(51.785341_f64, -1.470240_f64);
        let new_york_city = GeoPoint::new(40.7128_f64, -74.006_f64);
        let churchdown = GeoPoint::new(51.8773_f64, -2.1686_f64);
        let london = GeoPoint::new(51.5074_f64, -0.1278_f64);
        let oxford = GeoPoint::new(51.8773_f64, -1.2475878_f64);

        tree.insert(random.clone());
        tree.insert(random_plus_offset.clone());
        tree.insert(new_york_city.clone());
        tree.insert(churchdown.clone());
        tree.insert(london.clone());

        let mut results = tree.nearest_neighbor_iter_with_distance_2(&oxford.to_point().clone());

        let (point, distance) = results.next().unwrap();

        // Cast all f64 distances to f32 so tests pass under both MacOS and Linux.
        assert_eq!((point, distance as f32), (&random, 18388.597009683246_f32));

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point, distance),
            (&random_plus_offset, 18402.23696221235_f64)
        );

        let (point, distance) = results.next().unwrap();

        // Cast all f64 distances to f32 so tests pass under both MacOS and Linux.
        assert_eq!((point, distance as f32), (&churchdown, 63223.39709694925_f32));

        let (point, distance) = results.next().unwrap();

        assert_eq!((point, distance), (&london, 87458.64969073102_f64));

        let (point, distance) = results.next().unwrap();

        // Cast all f64 distances to f32 so tests pass under both MacOS and Linux.
        assert_eq!((point, distance as f32), (&new_york_city, 5484158.985172745_f32));

        assert_eq!(results.next(), None);

        let results: Vec<&GeoPoint> = tree
            .locate_within_distance(oxford.to_point(), 65000.0_f64)
            .collect();

        assert_eq_sorted!(results, vec![&churchdown, &random_plus_offset, &random]);
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
