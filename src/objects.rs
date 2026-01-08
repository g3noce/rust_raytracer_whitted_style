use crate::aabb::Aabb;
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::Vec3;
use std::path::Path;

#[derive(Clone)]
pub struct Intersection {
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Material,
}

pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub mat: Material,
}

impl Sphere {
    pub fn aabb(&self) -> Aabb {
        let r_vec = Vec3::new(self.radius, self.radius, self.radius);
        Aabb {
            min: self.center - r_vec,
            max: self.center + r_vec,
        }
    }
    pub fn intersect(&self, ray: &Ray) -> Option<(f32, Vec3, Material)> {
        let oc = ray.origin - self.center;
        let a = ray.direction.dot(ray.direction);
        let b = 2.0 * oc.dot(ray.direction);
        let c = oc.dot(oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }
        let t = (-b - discriminant.sqrt()) / (2.0 * a);
        if t < 0.001 {
            return None;
        }
        let hit_point = ray.origin + t * ray.direction;
        let normal = (hit_point - self.center).normalize();
        Some((t, normal, self.mat))
    }
}

pub struct Triangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    pub mat: Material,
}

impl Triangle {
    pub fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::empty();
        aabb.grow(self.v0);
        aabb.grow(self.v1);
        aabb.grow(self.v2);
        aabb
    }
    pub fn intersect(&self, ray: &Ray) -> Option<(f32, Vec3, Material)> {
        let epsilon = 1e-6;
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        let h = ray.direction.cross(edge2);
        let a = edge1.dot(h);
        if a.abs() < epsilon {
            return None;
        }
        let f = 1.0 / a;
        let s = ray.origin - self.v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(edge1);
        let v = f * ray.direction.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * edge2.dot(q);
        if t > epsilon {
            let mut normal = edge1.cross(edge2).normalize();
            if normal.dot(ray.direction) > 0.0 {
                normal = -normal;
            }
            return Some((t, normal, self.mat));
        }
        None
    }
}

pub enum Object {
    Sphere(Sphere),
    Triangle(Triangle),
}

impl Object {
    pub fn aabb(&self) -> Aabb {
        match self {
            Object::Sphere(s) => s.aabb(),
            Object::Triangle(t) => t.aabb(),
        }
    }
    pub fn intersect(&self, ray: &Ray) -> Option<(f32, Vec3, Material)> {
        match self {
            Object::Sphere(s) => s.intersect(ray),
            Object::Triangle(t) => t.intersect(ray),
        }
    }
}

pub fn load_obj(path: &str, translation: Vec3, scale: f32, mat: Material) -> Vec<Object> {
    let path_obj = Path::new(path);
    let (models, _materials) = tobj::load_obj(
        path_obj,
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
    )
    .expect("Failed to load OBJ file");

    let mut objects = Vec::new();

    for model in models {
        let mesh = model.mesh;

        for i in (0..mesh.indices.len()).step_by(3) {
            let idx0 = mesh.indices[i] as usize;
            let idx1 = mesh.indices[i + 1] as usize;
            let idx2 = mesh.indices[i + 2] as usize;

            let v0_raw = Vec3::new(
                mesh.positions[3 * idx0],
                mesh.positions[3 * idx0 + 1],
                mesh.positions[3 * idx0 + 2],
            );
            let v1_raw = Vec3::new(
                mesh.positions[3 * idx1],
                mesh.positions[3 * idx1 + 1],
                mesh.positions[3 * idx1 + 2],
            );
            let v2_raw = Vec3::new(
                mesh.positions[3 * idx2],
                mesh.positions[3 * idx2 + 1],
                mesh.positions[3 * idx2 + 2],
            );

            let v0 = v0_raw * scale + translation;
            let v1 = v1_raw * scale + translation;
            let v2 = v2_raw * scale + translation;

            objects.push(Object::Triangle(Triangle { v0, v1, v2, mat }));
        }
    }

    println!("Loaded {} triangles from {:?}", objects.len(), path);
    objects
}
