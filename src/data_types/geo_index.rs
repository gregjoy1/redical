use geo::prelude::*;
use geo::{Point, HaversineDistance};
use rstar::{RTree, RTreeObject, PointDistance};

use rstar::primitives::GeomWithData;

use serde::{Serialize, Serializer, Deserialize};

use std::collections::{HashMap, HashSet};

use std::hash::{Hash, Hasher};

use crate::data_types::{InvertedCalendarIndexTerm, IndexedConclusion};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoPoint {
    pub long: f64,
    pub lat:  f64,
}

impl From<(f64, f64)> for GeoPoint {
    #[inline]
    fn from(coords: (f64, f64)) -> Self {
        GeoPoint::new(
            coords.0,
            coords.1,
        )
    }
}

impl From<GeoPoint> for Point {
    #[inline]
    fn from(geo_point: GeoPoint) -> Self {
        geo_point.to_point()
    }
}

impl GeoPoint {
    pub fn new(long: f64, lat: f64) -> Self {
        GeoPoint {
            long,
            lat,
        }
    }

    pub fn to_point(&self) -> Point {
        Point::new(self.long, self.lat)
    }

    pub fn validate(&self) -> Result<&Self, String> {
        if self.lat < -90f64 || self.lat > 90f64 {
            return Err(format!("Expected latitude: {} to be greater than -90 and less than 90.", self.lat));
        }

        if self.long < -180f64 || self.long > 180f64 {
            return Err(format!("Expected latitude: {} to be greater than -180 and less than 180.", self.long));
        }

        Ok(self)
    }

    pub fn to_string(&self) -> String {
        format!("{};{}", self.long, self.lat)
    }

    pub fn geohash(&self) -> Result<String, String> {
        geohash::encode(
            geohash::Coord {
                x: self.long,
                y: self.lat,
            },
            12, // Accurrate to 37.2mm × 18.6mm
        ).map_err(|geohash_error| {
            geohash_error.to_string()
        })
    }
}


impl RTreeObject for GeoPoint
{
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
//#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoSpatialCalendarIndex {
    pub coords: RTree<GeomWithData<GeoPoint, InvertedCalendarIndexTerm>>,
}

impl PartialEq for GeoSpatialCalendarIndex {

    fn eq(&self, other: &GeoSpatialCalendarIndex) -> bool {
        let self_iter  = self.coords.into_iter();
        let other_iter = other.coords.into_iter();

        for (self_element, other_element) in self_iter.zip(other_iter) {
            if self_element != other_element {
                return false;
            }
        }

        true
    }

}

impl GeoSpatialCalendarIndex {

    pub fn new() -> Self {
        GeoSpatialCalendarIndex {
            coords: RTree::new()
        }
    }

    pub fn insert(&mut self, event_uuid: String, long_lat: &GeoPoint, indexed_conclusion: &IndexedConclusion) -> Result<&mut Self, String> {
        match self.coords.locate_at_point_mut(&long_lat.to_point()) {
            Some(existing_result) => {
                match indexed_conclusion {
                    IndexedConclusion::Include(exceptions) => existing_result.data.insert_included_event(event_uuid.clone(), exceptions.clone()),
                    IndexedConclusion::Exclude(exceptions) => existing_result.data.insert_excluded_event(event_uuid.clone(), exceptions.clone()),
                };
            },

            None => {
                self.coords
                    .insert(
                        GeomWithData::new(
                            long_lat.clone(),
                            InvertedCalendarIndexTerm::new_with_event(event_uuid.clone(), indexed_conclusion.clone())
                        )
                    );
            },
        }

        Ok(self)
    }

    pub fn remove(&mut self, event_uuid: String, long_lat: &GeoPoint) -> Result<&mut Self, String> {
        if let Some(existing_result) = self.coords.locate_at_point_mut(&long_lat.to_point()) {
            if existing_result.data.remove_event(event_uuid).is_ok_and(|inverted_calendar_index_term| inverted_calendar_index_term.is_empty()) {
                self.coords.remove_at_point(&long_lat.to_point());
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_geo_spatial_calendar_index() {
        let mut geo_spatial_calendar_index = GeoSpatialCalendarIndex::new();

        let london = GeoPoint::new(-0.1278f64,    51.5074f64);
        let oxford = GeoPoint::new(-1.2475878f64, 51.8773f64);

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::new(),
            }
        );

        assert!(
            geo_spatial_calendar_index.insert(
                String::from("london_event_uuid_one"),
                &london,
                &IndexedConclusion::Include(None)
            ).is_ok(),
        );

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(
                            vec![
                                GeomWithData::new(
                                    london.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([(String::from("london_event_uuid_one"), IndexedConclusion::Include(None))])
                                    }
                                )
                            ]
                        )
            }
        );

        assert!(
            geo_spatial_calendar_index.insert(
                String::from("london_event_uuid_two"),
                &london,
                &IndexedConclusion::Exclude(
                    Some(
                        HashSet::from([100])
                    )
                )
            ).is_ok(),
        );

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(
                            vec![
                                GeomWithData::new(
                                    london.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("london_event_uuid_one"), IndexedConclusion::Include(None)),
                                                    (String::from("london_event_uuid_two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                                        ])
                                    }
                                )
                            ]
                        )
            }
        );

        assert!(
            geo_spatial_calendar_index.insert(
                String::from("oxford_event_uuid"),
                &oxford,
                &IndexedConclusion::Include(
                    Some(
                        HashSet::from([100])
                    )
                )
            ).is_ok(),
        );

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(
                            vec![
                                GeomWithData::new(
                                    london.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("london_event_uuid_one"), IndexedConclusion::Include(None)),
                                                    (String::from("london_event_uuid_two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                                        ])
                                    }
                                ),
                                GeomWithData::new(
                                    oxford.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("oxford_event_uuid"),     IndexedConclusion::Include(Some(HashSet::from([100])))),
                                        ])
                                    }
                                )
                            ]
                        )
            }
        );

        assert!(geo_spatial_calendar_index.remove(String::from("oxford_event_uuid"), &oxford).is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(
                            vec![
                                GeomWithData::new(
                                    london.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("london_event_uuid_one"), IndexedConclusion::Include(None)),
                                                    (String::from("london_event_uuid_two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                                        ])
                                    }
                                )
                            ]
                        )
            }
        );

        assert!(geo_spatial_calendar_index.remove(String::from("london_event_uuid_one"), &london).is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(
                            vec![
                                GeomWithData::new(
                                    london.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("london_event_uuid_two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                                        ])
                                    }
                                )
                            ]
                        )
            }
        );

        assert!(geo_spatial_calendar_index.remove(String::from("london_event_uuid_one"), &london).is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(
                            vec![
                                GeomWithData::new(
                                    london.clone(),
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("london_event_uuid_two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                                        ])
                                    }
                                )
                            ]
                        )
            }
        );

        assert!(geo_spatial_calendar_index.remove(String::from("london_event_uuid_two"), &london).is_ok());

        assert_eq!(
            geo_spatial_calendar_index,
            GeoSpatialCalendarIndex {
                coords: RTree::bulk_load(vec![])
            }
        );
    }

    #[test]
    fn test_geo_distance_rtree() {
        let mut tree = RTree::new();

        let random             = GeoPoint::new(-1.4701705f64, 51.7854972f64);
        let random_plus_offset = GeoPoint::new(-1.470240f64,  51.785341f64);
        let new_york_city      = GeoPoint::new(-74.006f64,    40.7128f64);
        let churchdown         = GeoPoint::new(-2.1686f64,    51.8773f64);
        let london             = GeoPoint::new(-0.1278f64,    51.5074f64);
        let oxford             = GeoPoint::new(-1.2475878f64, 51.8773f64);

        tree.insert(random.clone());
        tree.insert(random_plus_offset.clone());
        tree.insert(new_york_city.clone());
        tree.insert(churchdown.clone());
        tree.insert(london.clone());

        let mut results = tree.nearest_neighbor_iter_with_distance_2(&oxford.to_point().clone());

        let (point, distance) = results.next().unwrap();
        assert_eq!(
            (point,   distance),
            (&random, 18388.59700968325f64)
        );


        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,               distance),
            (&random_plus_offset, 18402.23696221235f64)
        );

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,       distance),
            (&churchdown, 63223.39709694926f64)
        );

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,   distance),
            (&london, 87458.64969073102f64)
        );

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,          distance),
            (&new_york_city, 5484158.985172745f64)
        );

        assert_eq!(results.next(), None);

        let results: Vec<&GeoPoint> = tree.locate_within_distance(oxford.to_point().clone(), 65000.0f64).collect();

        assert_eq_sorted!(results, vec![&churchdown, &random_plus_offset, &random]);
    }
}
