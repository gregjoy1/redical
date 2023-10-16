use geo::prelude::*;
use geo::point;
use geo::Point;
use rstar::RTree;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_geo_distance_rtree() {
        let mut tree = RTree::new();

        let random             = point!(x: -1.4701705f64, y: 51.7854972f64);
        let random_plus_offset = point!(x: -1.470240f64, y: 51.785341f64);
        let new_york_city      = point!(x: -74.006f64, y: 40.7128f64);
        let churchdown         = point!(x: -2.1686f64, y: 51.8773f64);
        let london             = point!(x: -0.1278f64, y: 51.5074f64);
        let oxford             = point!(x: -1.2475878f64, y: 51.8773f64);

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
