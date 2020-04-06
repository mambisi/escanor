use rstar::{RTreeObject, AABB};
use std::hash::{Hash, Hasher};

 pub struct Circle
 {
     pub origin: [f32; 2],
     pub radius: f32,
 }

 impl RTreeObject for Circle {
     type Envelope = AABB<[f32; 2]>;

     fn envelope(&self) -> Self::Envelope {
         let corner_1 = [self.origin[0] - self.radius, self.origin[1] - self.radius];
         let corner_2 = [self.origin[0] + self.radius, self.origin[1] + self.radius];
         AABB::from_corners(corner_1, corner_2)
     }
}

pub struct GeoPoint2D {
    pub tag: String,
    pub lat: f64,
    pub lng: f64,
}

impl RTreeObject for GeoPoint2D
{
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope
    {
        AABB::from_point([self.lat, self.lng])
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