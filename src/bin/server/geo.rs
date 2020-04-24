use rstar::{RTreeObject, AABB, PointDistance};
use std::hash::{Hash, Hasher};
use crate::util;

use serde::{Serialize, Deserialize};
use crate::printer::{JsonPrint, GeoJsonFeature};
use serde_json::Value;
use geohash::Coordinate;

pub type Scalar = f64;


#[derive(Clone, Debug)]
pub struct Circle
{
    pub origin: [Scalar; 2],
    pub radius: Scalar,
}

impl RTreeObject for Circle {
    type Envelope = AABB<[Scalar; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let corner_1 = [self.origin[0] - self.radius, self.origin[1] - self.radius];
        let corner_2 = [self.origin[0] + self.radius, self.origin[1] + self.radius];
        AABB::from_corners(corner_1, corner_2)
    }
}

impl PointDistance for Circle
{
    fn distance_2(&self, point: &[Scalar; 2]) -> f64
    {
        let distance_to_origin = util::get_distance((self.origin[0], self.origin[1]), (point[0], point[1]));
        let distance_to_ring = distance_to_origin - self.radius;
        let distance_to_circle = Scalar::max(0.0, distance_to_ring);
        // We must return the squared distance!
        distance_to_circle * distance_to_circle
    }

    // This implementation is not required but more efficient since it
    // omits the calculation of a square root
    fn contains_point(&self, point: &[Scalar; 2]) -> bool
    {
        let distance_to_origin_2 = util::get_distance((self.origin[0], self.origin[1]), (point[0], point[1]));
        let radius_2 = self.radius * self.radius;
        distance_to_origin_2 <= radius_2
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoPoint2D {
    pub tag: String,
    pub data: Option<Value>,
    x_cord: f64,
    y_cord: f64,
    hash: String,
}


impl GeoPoint2D {
    pub fn new(tag: String) -> Self {
        GeoPoint2D {
            tag,
            data: None,
            x_cord: 0.0,
            y_cord: 0.0,
            hash: String::new(),
        }
    }
    pub fn with_cord(tag: String, x_cord: f64, y_cord: f64) -> Self {
        let mut geo_point = GeoPoint2D {
            tag,
            data: None,
            x_cord,
            y_cord,
            hash: String::new(),
        };
        match geohash::encode(Coordinate { x: geo_point.x_cord, y: geo_point.y_cord }, 10) {
            Ok(t) => geo_point.hash = t,
            Err(e) => {}
        };

        geo_point
    }
    pub fn set_cord(&mut self, x_cord: f64, y_cord: f64) {
        self.x_cord = x_cord;
        self.y_cord = y_cord;

        match geohash::encode(Coordinate { x: self.x_cord, y: self.y_cord }, 10) {
            Ok(t) => self.hash = t,
            Err(e) => {}
        };
    }

    pub fn set_x_cord(&mut self, x_cord: f64) {
        self.x_cord = x_cord;
        match geohash::encode(Coordinate { x: self.x_cord, y: self.y_cord }, 10) {
            Ok(t) => self.hash = t,
            Err(e) => {}
        };
    }
    pub fn set_y_cord(&mut self, y_cord: f64) {
        self.y_cord = y_cord;
        match geohash::encode(Coordinate { x: self.x_cord, y: self.y_cord }, 10) {
            Ok(t) => self.hash = t,
            Err(e) => {}
        };
    }

    pub fn x_cord(&self) -> f64 {
        self.x_cord
    }

    pub fn y_cord(&self) -> f64 {
        self.y_cord
    }

    pub fn hash(&self) -> &String {
        &self.hash
    }

    pub fn get_cord(&self) -> (f64, f64) {
        (self.x_cord, self.y_cord)
    }
}

impl RTreeObject for GeoPoint2D
{
    type Envelope = AABB<[Scalar; 2]>;

    fn envelope(&self) -> Self::Envelope
    {
        AABB::from_point([self.x_cord, self.y_cord])
    }
}

impl PointDistance for GeoPoint2D {
    fn distance_2(&self, point: &[Scalar; 2]) -> Scalar
    {
        let distance_to_origin = util::get_distance((self.x_cord, self.y_cord), (point[0], point[1]));
        distance_to_origin
    }
}

impl PartialEq for GeoPoint2D {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}

impl Eq for GeoPoint2D {}

impl Hash for GeoPoint2D {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.tag.hash(hasher)
    }
}

impl JsonPrint for GeoPoint2D {
    fn print_json(&self) -> Value {
        json!(
        {
              "type": "Feature",
              "properties": {
                "name" : self.tag,
                "data" : self.data
              },
              "geometry": {
                "type": "Point",
                "coordinates": [
                  self.x_cord,
                  self.y_cord
                ]
              }
            }
        )
    }
}

impl GeoJsonFeature for GeoPoint2D {
    fn geo_json_feature(&self) -> Value {
        self.print_json()
    }
}