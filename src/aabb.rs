use crate::ray::Ray;
use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
        }
    }
    pub fn union(&self, other: &Aabb) -> Aabb {
        Aabb {
            min: self.min.min_vec(other.min),
            max: self.max.max_vec(other.max),
        }
    }
    pub fn grow(&mut self, p: Vec3) {
        self.min = self.min.min_vec(p);
        self.max = self.max.max_vec(p);
    }
    pub fn intersect(&self, ray: &Ray) -> f32 {
        let t1 = (self.min.x - ray.origin.x) * ray.inv_direction.x;
        let t2 = (self.max.x - ray.origin.x) * ray.inv_direction.x;
        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let t1 = (self.min.y - ray.origin.y) * ray.inv_direction.y;
        let t2 = (self.max.y - ray.origin.y) * ray.inv_direction.y;
        let tmin = tmin.max(t1.min(t2));
        let tmax = tmax.min(t1.max(t2));

        let t1 = (self.min.z - ray.origin.z) * ray.inv_direction.z;
        let t2 = (self.max.z - ray.origin.z) * ray.inv_direction.z;
        let tmin = tmin.max(t1.min(t2));
        let tmax = tmax.min(t1.max(t2));

        if tmax >= tmin.max(0.0) {
            tmin
        } else {
            f32::MAX
        }
    }
}
