use crate::vec3::Vec3;

#[derive(Clone, Copy)]
pub struct Material {
    pub albedo: Vec3,
    pub emission: Vec3,
    pub specular: f32,
    pub shininess: f32,
    pub checkered: bool,
}
