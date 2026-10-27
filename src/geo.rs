//! Point-in-polygon over the simplesolat-data GeoJSON maps, to find the zone
//! for a GPS location (what the app does with polygon-lookup).

use serde::Deserialize;

/// A ring of (lng, lat) points; the first ring of a polygon is its outline,
/// any others are holes.
type Ring = Vec<(f64, f64)>;
type Polygon = Vec<Ring>;

#[derive(Debug, Deserialize)]
pub struct FeatureCollection {
    pub features: Vec<Feature>,
}

#[derive(Debug, Deserialize)]
pub struct Feature {
    pub properties: serde_json::Map<String, serde_json::Value>,
    pub geometry: Option<Geometry>,
}

/// Coordinates are [lng, lat] (some files carry a third value, ignored).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
pub enum Geometry {
    Polygon(Vec<Vec<Vec<f64>>>),
    MultiPolygon(Vec<Vec<Vec<Vec<f64>>>>),
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy)]
struct BBox {
    min_lng: f64,
    min_lat: f64,
    max_lng: f64,
    max_lat: f64,
}

impl BBox {
    fn empty() -> Self {
        Self {
            min_lng: f64::INFINITY,
            min_lat: f64::INFINITY,
            max_lng: f64::NEG_INFINITY,
            max_lat: f64::NEG_INFINITY,
        }
    }

    fn extend(&mut self, (lng, lat): (f64, f64)) {
        self.min_lng = self.min_lng.min(lng);
        self.min_lat = self.min_lat.min(lat);
        self.max_lng = self.max_lng.max(lng);
        self.max_lat = self.max_lat.max(lat);
    }

    fn merge(&mut self, other: &BBox) {
        self.extend((other.min_lng, other.min_lat));
        self.extend((other.max_lng, other.max_lat));
    }

    fn contains(&self, lng: f64, lat: f64) -> bool {
        lng >= self.min_lng && lng <= self.max_lng && lat >= self.min_lat && lat <= self.max_lat
    }
}

/// One district (GeoJSON feature): its properties and outline.
pub struct Shape {
    pub properties: serde_json::Map<String, serde_json::Value>,
    bbox: BBox,
    polygons: Vec<Polygon>,
}

impl Shape {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).and_then(|v| v.as_str())
    }

    fn contains(&self, lng: f64, lat: f64) -> bool {
        self.bbox.contains(lng, lat) && self.polygons.iter().any(|p| polygon_contains(p, lng, lat))
    }
}

/// All the districts of one map, with a bounding box to skip the whole map.
pub struct Shapes {
    shapes: Vec<Shape>,
    bbox: BBox,
}

impl Shapes {
    pub fn from_geojson(collection: FeatureCollection) -> Self {
        let mut bbox = BBox::empty();
        let shapes: Vec<Shape> = collection
            .features
            .into_iter()
            .filter_map(|f| {
                let polygons = match f.geometry? {
                    Geometry::Polygon(p) => vec![to_polygon(p)],
                    Geometry::MultiPolygon(ps) => ps.into_iter().map(to_polygon).collect(),
                    Geometry::Other => return None,
                };
                let mut shape_bbox = BBox::empty();
                for point in polygons.iter().flat_map(|p| p.first()).flatten() {
                    shape_bbox.extend(*point);
                }
                bbox.merge(&shape_bbox);
                Some(Shape {
                    properties: f.properties,
                    bbox: shape_bbox,
                    polygons,
                })
            })
            .collect();
        Self { shapes, bbox }
    }

    /// The district containing the point, if any.
    pub fn find(&self, lng: f64, lat: f64) -> Option<&Shape> {
        if !self.bbox.contains(lng, lat) {
            return None;
        }
        self.shapes.iter().find(|s| s.contains(lng, lat))
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
}

fn to_polygon(rings: Vec<Vec<Vec<f64>>>) -> Polygon {
    rings
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .filter(|c| c.len() >= 2)
                .map(|c| (c[0], c[1]))
                .collect()
        })
        .collect()
}

/// Inside the outline and not inside a hole.
fn polygon_contains(polygon: &Polygon, lng: f64, lat: f64) -> bool {
    let Some((outline, holes)) = polygon.split_first() else {
        return false;
    };
    ring_contains(outline, lng, lat) && !holes.iter().any(|h| ring_contains(h, lng, lat))
}

/// Ray casting (even-odd rule).
fn ring_contains(ring: &Ring, lng: f64, lat: f64) -> bool {
    let mut inside = false;
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = ring[i];
        let (xj, yj) = ring[j];
        if (yi > lat) != (yj > lat) && lng < (xj - xi) * (lat - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shapes(json: &str) -> Shapes {
        Shapes::from_geojson(serde_json::from_str(json).unwrap())
    }

    // A 10x10 square with a 2x2 hole in the middle, and a separate
    // MultiPolygon made of two 1x1 islands.
    const MAP: &str = r#"{"type": "FeatureCollection", "features": [
        {"type": "Feature", "properties": {"shapeName": "Square"},
         "geometry": {"type": "Polygon", "coordinates": [
            [[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]],
            [[4, 4], [6, 4], [6, 6], [4, 6], [4, 4]]]}},
        {"type": "Feature", "properties": {"shapeName": "Islands"},
         "geometry": {"type": "MultiPolygon", "coordinates": [
            [[[20, 0, 5], [21, 0, 5], [21, 1, 5], [20, 1, 5], [20, 0, 5]]],
            [[[30, 0], [31, 0], [31, 1], [30, 1], [30, 0]]]]}},
        {"type": "Feature", "properties": {"shapeName": "Nothing"}, "geometry": null}
    ]}"#;

    fn name(s: &Shapes, lng: f64, lat: f64) -> Option<&str> {
        s.find(lng, lat)
            .and_then(|shape| shape.property("shapeName"))
    }

    #[test]
    fn test_point_in_polygon() {
        let s = shapes(MAP);
        assert_eq!(s.len(), 2);
        assert_eq!(name(&s, 1.0, 1.0), Some("Square"));
        assert_eq!(name(&s, 9.5, 5.0), Some("Square"));
    }

    #[test]
    fn test_point_in_hole_is_outside() {
        assert_eq!(name(&shapes(MAP), 5.0, 5.0), None);
    }

    #[test]
    fn test_multipolygon_and_third_coordinate() {
        let s = shapes(MAP);
        assert_eq!(name(&s, 20.5, 0.5), Some("Islands"));
        assert_eq!(name(&s, 30.5, 0.5), Some("Islands"));
        assert_eq!(name(&s, 25.0, 0.5), None);
    }

    #[test]
    fn test_outside_map() {
        let s = shapes(MAP);
        assert_eq!(name(&s, -1.0, 5.0), None);
        assert_eq!(name(&s, 5.0, 50.0), None);
    }
}
