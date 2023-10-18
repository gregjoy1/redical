use geo::prelude::*;
use geo::{Point, Coord};
use rstar::RTree;

use rstar::primitives::GeomWithData;

use serde::{Serialize, Serializer, Deserialize};

use std::collections::{HashMap, HashSet};

use std::hash::{Hash, Hasher};

use crate::data_types::{InvertedCalendarIndexTerm, IndexedConclusion};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LongLatCoord((f64, f64));

impl From<Coord<f64>> for LongLatCoord {
    #[inline]
    fn from(coord: Coord<f64>) -> Self {
        LongLatCoord(coord.into())
    }
}

impl From<(f64, f64)> for LongLatCoord {
    #[inline]
    fn from(coords: (f64, f64)) -> Self {
        LongLatCoord(coords.into())
    }
}

impl LongLatCoord {
    pub fn to_string(&self) -> String {
        format!("{};{}", self.0.0, self.0.1)
    }

    pub fn to_coord(&self) -> Coord<f64> {
        Coord::from(self.0)
    }
}

impl Hash for LongLatCoord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.to_string().hash(state);
    }
}

impl PartialEq for LongLatCoord {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for LongLatCoord {}


// Multi layer inverted index (for multiple events) - indexed term - event - include/exclude
//#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[derive(Debug, Clone)]
pub struct GeoSpatialCalendarIndex {
    pub coords: RTree<GeomWithData<Point, InvertedCalendarIndexTerm>>,
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

    pub fn insert(&mut self, event_uuid: String, long_lat: &Point, indexed_conclusion: &IndexedConclusion) -> Result<&mut Self, String> {
        match self.coords.locate_at_point_mut(&long_lat) {
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

    pub fn remove(&mut self, event_uuid: String, long_lat: &Point) -> Result<&mut Self, String> {
        if let Some(existing_result) = self.coords.locate_at_point_mut(&long_lat) {
            if existing_result.data.remove_event(event_uuid).is_ok_and(|inverted_calendar_index_term| inverted_calendar_index_term.is_empty()) {
                self.coords.remove_at_point(&long_lat);
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_geo_spatial_calendar_index() {
        let mut geo_spatial_calendar_index = GeoSpatialCalendarIndex::new();

        let london = Point::new(-0.1278f64,    51.5074f64);
        let oxford = Point::new(-1.2475878f64, 51.8773f64);

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
                                    london,
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
                                    london,
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
                                    london,
                                    InvertedCalendarIndexTerm {
                                        events: HashMap::from([
                                                    (String::from("london_event_uuid_one"), IndexedConclusion::Include(None)),
                                                    (String::from("london_event_uuid_two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                                        ])
                                    }
                                ),
                                GeomWithData::new(
                                    oxford,
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
                                    london,
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
                                    london,
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
                                    london,
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

        let random             = Point::new(-1.4701705f64, 51.7854972f64);
        let random_plus_offset = Point::new(-1.470240f64,  51.785341f64);
        let new_york_city      = Point::new(-74.006f64,    40.7128f64);
        let churchdown         = Point::new(-2.1686f64,    51.8773f64);
        let london             = Point::new(-0.1278f64,    51.5074f64);
        let oxford             = Point::new(-1.2475878f64, 51.8773f64);

        tree.insert(random);
        tree.insert(random_plus_offset);
        tree.insert(new_york_city);
        tree.insert(churchdown);
        tree.insert(london);

        let mut results = tree.nearest_neighbor_iter_with_distance_2(&oxford);

        let (point, distance) = results.next().unwrap();
        assert_eq!(
            (point,   distance,            oxford.haversine_distance(&point)),
            (&random, 0.05797081242712935, 18388.59700968325)
        );


        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,               distance,           oxford.haversine_distance(&point)),
            (&random_plus_offset, 0.0580304598458392, 18402.23696221235)
        );

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,       distance,           oxford.haversine_distance(&point)),
            (&churchdown, 0.8482634725488402, 63223.39709694926)
        );

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,   distance,           oxford.haversine_distance(&point)),
            (&london, 1.3907507270288413, 87458.64969073102)
        );

        let (point, distance) = results.next().unwrap();

        assert_eq!(
            (point,          distance,          oxford.haversine_distance(&point)),
            (&new_york_city, 5418.432606115109, 5484158.985172745)
        );

        assert_eq!(results.next(), None);
    }
}
