use rstar::{RTreeObject, AABB, PointDistance};
use std::hash::{Hash, Hasher};
use crate::util;

use serde::{Serialize, Deserialize};
use crate::printer::{JsonPrint, GeoJsonFeature};
use serde_json::Value;

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
    pub x_cord: f64,
    pub y_cord: f64,
    pub hash: String,
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
                "name" : self.tag
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