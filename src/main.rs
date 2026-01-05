use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use rayon::prelude::*;
use std::ops::{Add, Mul, Neg, Sub};

const BUFFER_WIDTH: usize = 192 * 8;
const BUFFER_HEIGHT: usize = 108 * 8;
const WINDOW_WIDTH: usize = 1920;
const WINDOW_HEIGHT: usize = 1080;
const MAX_BOUNCES: u8 = 4;
const MOVE_SPEED: f32 = 0.1;
const MOUSE_SENSITIVITY: f32 = 0.2;
const GAMMA: f32 = 2.2;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }
    fn dot(&self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
    fn cross(&self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
    fn normalize(&self) -> Vec3 {
        let len = self.dot(*self).sqrt();
        if len == 0.0 {
            Vec3::new(0.0, 0.0, 0.0)
        } else {
            *self * (1.0 / len)
        }
    }

    fn mul_vec(&self, other: Vec3) -> Vec3 {
        Vec3::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    fn len_sq(&self) -> f32 {
        self.dot(*self)
    }
    fn len(&self) -> f32 {
        self.dot(*self).sqrt()
    }
    fn to_u32_gamma(self) -> u32 {
        let r = (self.x.powf(1.0 / GAMMA).clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.y.powf(1.0 / GAMMA).clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.z.powf(1.0 / GAMMA).clamp(0.0, 1.0) * 255.0) as u32;
        (255 << 24) | (r << 16) | (g << 8) | b
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        Vec3::new(self * v.x, self * v.y, self * v.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
}

#[derive(Clone, Copy)]
struct Material {
    albedo: Vec3,
    emission: Vec3,
    reflectivity: f32,
    checkered: bool,
}

#[derive(Clone)]
struct Intersection {
    point: Vec3,
    normal: Vec3,
    material: Material,
}

struct Sphere {
    center: Vec3,
    radius: f32,
    mat: Material,
}

impl Sphere {
    fn intersect(&self, ray: &Ray) -> Option<(f32, Vec3, Material)> {
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

struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    mat: Material,
}

impl Triangle {
    fn intersect(&self, ray: &Ray) -> Option<(f32, Vec3, Material)> {
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

enum Object {
    Sphere(Sphere),
    Triangle(Triangle),
}

impl Object {
    fn intersect(&self, ray: &Ray) -> Option<(f32, Vec3, Material)> {
        match self {
            Object::Sphere(s) => s.intersect(ray),
            Object::Triangle(t) => t.intersect(ray),
        }
    }
}

struct PointLight {
    position: Vec3,
    color: Vec3,
    intensity: f32,
}

struct Camera {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    last_mouse_pos: (f32, f32),
}

impl Camera {
    fn new(pos: Vec3, start_mouse: (f32, f32)) -> Self {
        Self {
            pos,
            yaw: -90.0,
            pitch: 0.0,
            last_mouse_pos: start_mouse,
        }
    }

    fn update(&mut self, window: &Window) {
        let current_mouse_pos = window
            .get_mouse_pos(MouseMode::Pass)
            .unwrap_or(self.last_mouse_pos);
        let dx = current_mouse_pos.0 - self.last_mouse_pos.0;
        let dy = current_mouse_pos.1 - self.last_mouse_pos.1;

        if window.get_mouse_down(MouseButton::Left) {
            self.yaw += dx * MOUSE_SENSITIVITY;
            self.pitch = (self.pitch - dy * MOUSE_SENSITIVITY).clamp(-89.0, 89.0);
        }
        self.last_mouse_pos = current_mouse_pos;

        let (forward, right, _) = self.get_vectors();
        let global_up = Vec3::new(0.0, 1.0, 0.0);

        if window.is_key_down(Key::W) {
            self.pos = self.pos + forward * MOVE_SPEED;
        }
        if window.is_key_down(Key::S) {
            self.pos = self.pos - forward * MOVE_SPEED;
        }
        if window.is_key_down(Key::A) {
            self.pos = self.pos - right * MOVE_SPEED;
        }
        if window.is_key_down(Key::D) {
            self.pos = self.pos + right * MOVE_SPEED;
        }
        if window.is_key_down(Key::Space) {
            self.pos = self.pos + global_up * MOVE_SPEED;
        }
        if window.is_key_down(Key::LeftShift) {
            self.pos = self.pos - global_up * MOVE_SPEED;
        }
    }

    fn get_vectors(&self) -> (Vec3, Vec3, Vec3) {
        let (rad_yaw, rad_pitch) = (self.yaw.to_radians(), self.pitch.to_radians());

        let forward = Vec3::new(
            rad_yaw.cos() * rad_pitch.cos(),
            rad_pitch.sin(),
            rad_yaw.sin() * rad_pitch.cos(),
        )
        .normalize();

        let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(forward).normalize();
        (forward, right, up)
    }
}

fn get_closest_intersection(ray: &Ray, objects: &[Object]) -> Option<Intersection> {
    let mut closest_t = f32::MAX;
    let mut closest_hit: Option<Intersection> = None;

    for obj in objects {
        if let Some((t, normal, mat)) = obj.intersect(ray) {
            if t < closest_t {
                closest_t = t;
                closest_hit = Some(Intersection {
                    point: ray.origin + t * ray.direction,
                    normal,
                    material: mat,
                });
            }
        }
    }
    closest_hit
}

fn compute_pixel_color(mut ray: Ray, objects: &[Object], light: &PointLight) -> u32 {
    let mut final_color = Vec3::new(0.0, 0.0, 0.0);
    let mut throughput = Vec3::new(1.0, 1.0, 1.0);

    for bounce in 0..MAX_BOUNCES {
        if let Some(hit) = get_closest_intersection(&ray, objects) {
            final_color = final_color + throughput.mul_vec(hit.material.emission);

            let mut albedo = hit.material.albedo;
            if hit.material.checkered {
                let size = 1.0;
                let x = (hit.point.x * size).floor() as i32;
                let z = (hit.point.z * size).floor() as i32;
                if (x + z) % 2 != 0 {
                    albedo = Vec3::new(0.1, 0.1, 0.1);
                }
            }

            let diffuse_factor = 1.0 - hit.material.reflectivity;

            if diffuse_factor > 0.0 {
                let light_vec = light.position - hit.point;
                let dist_sq = light_vec.len_sq();
                let dist = dist_sq.sqrt();
                let light_dir = light_vec * (1.0 / dist);

                let shadow_ray = Ray {
                    origin: hit.point + hit.normal * 0.001,
                    direction: light_dir,
                };

                let mut in_shadow = false;
                if let Some(shadow_hit) = get_closest_intersection(&shadow_ray, objects) {
                    let dist_to_blocker_sq = (shadow_hit.point - shadow_ray.origin).len_sq();
                    if dist_to_blocker_sq < dist_sq && shadow_hit.material.emission.len() == 0.0 {
                        in_shadow = true;
                    }
                }

                if !in_shadow {
                    let n_dot_l = hit.normal.dot(light_dir).max(0.0);

                    let attenuation = 1.0 / dist_sq;

                    let incoming_light = light.color * light.intensity * attenuation;

                    let reflected_light = albedo.mul_vec(incoming_light) * n_dot_l;

                    final_color =
                        final_color + throughput.mul_vec(reflected_light) * diffuse_factor;
                }
            }

            if hit.material.reflectivity > 0.0 {
                throughput = throughput.mul_vec(albedo) * hit.material.reflectivity;

                let reflect = ray.direction - 2.0 * ray.direction.dot(hit.normal) * hit.normal;

                ray = Ray {
                    origin: hit.point + hit.normal * 0.001,
                    direction: reflect.normalize(),
                };
            } else {
                break;
            }
        } else {
            let background = Vec3::new(0.05, 0.05, 0.1) * (0.5_f32).powi(bounce as i32);
            final_color = final_color + throughput.mul_vec(background);
            break;
        }
    }
    final_color.to_u32_gamma()
}

fn render_frame(buffer: &mut [u32], camera: &Camera, objects: &[Object], light: &PointLight) {
    let aspect_ratio = BUFFER_WIDTH as f32 / BUFFER_HEIGHT as f32;

    let fov_scale = (90.0f32.to_radians() / 2.0).tan();
    let (cam_forward, cam_right, cam_up) = camera.get_vectors();

    buffer
        .par_chunks_mut(BUFFER_WIDTH)
        .enumerate()
        .for_each(|(j, row)| {
            for (i, pixel) in row.iter_mut().enumerate() {
                let ndc_x =
                    (2.0 * (i as f32 + 0.5) / BUFFER_WIDTH as f32 - 1.0) * aspect_ratio * fov_scale;
                let ndc_y = (1.0 - 2.0 * (j as f32 + 0.5) / BUFFER_HEIGHT as f32) * fov_scale;

                let direction = (cam_forward + cam_right * ndc_x + cam_up * ndc_y).normalize();

                let ray = Ray {
                    origin: camera.pos,
                    direction,
                };

                *pixel = compute_pixel_color(ray, objects, light);
            }
        });
}

fn init_scene() -> (Vec<Object>, PointLight) {
    let light = PointLight {
        position: Vec3::new(2.0, 5.0, 3.0),
        color: Vec3::new(0.4823, 0.1686, 0.552),
        intensity: 80.0,
    };

    let mat_mirror = Material {
        albedo: Vec3::new(1.0, 1.0, 1.0),
        emission: Vec3::new(0.0, 0.0, 0.0),
        reflectivity: 0.9,
        checkered: false,
    };
    let mat_red_matte = Material {
        albedo: Vec3::new(0.9, 0.1, 0.1),
        emission: Vec3::new(0.0, 0.0, 0.0),
        reflectivity: 0.1,
        checkered: false,
    };
    let mat_checker = Material {
        albedo: Vec3::new(0.9, 0.9, 0.9),
        emission: Vec3::new(0.0, 0.0, 0.0),
        reflectivity: 0.5,
        checkered: true,
    };

    let mat_bulb = Material {
        albedo: Vec3::new(0.0, 0.0, 0.0),
        emission: light.color * light.intensity,
        reflectivity: 0.0,
        checkered: false,
    };

    let objects = vec![
        // Sphère représentant la lumière
        Object::Sphere(Sphere {
            center: light.position,
            radius: 0.2,
            mat: mat_bulb,
        }),
        // Sphère Miroir
        Object::Sphere(Sphere {
            center: Vec3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            mat: mat_mirror,
        }),
        // Sphère Rouge Mate
        Object::Sphere(Sphere {
            center: Vec3::new(-2.0, 0.5, -1.0),
            radius: 0.5,
            mat: mat_red_matte,
        }),
        // Sol 1
        Object::Triangle(Triangle {
            v0: Vec3::new(-20.0, 0.0, -20.0),
            v1: Vec3::new(-20.0, 0.0, 20.0),
            v2: Vec3::new(20.0, 0.0, 20.0),
            mat: mat_checker,
        }),
        // Sol 2
        Object::Triangle(Triangle {
            v0: Vec3::new(-20.0, 0.0, -20.0),
            v1: Vec3::new(20.0, 0.0, 20.0),
            v2: Vec3::new(20.0, 0.0, -20.0),
            mat: mat_checker,
        }),
    ];
    (objects, light)
}

fn main() {
    let mut window = Window::new(
        "Raytracer - Whitted Style (Mirror & Checkerboard)",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            borderless: true,
            title: false,
            topmost: true,
            resize: false,
            scale: minifb::Scale::FitScreen,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    window.set_target_fps(60);
    let mut buffer: Vec<u32> = vec![0; BUFFER_WIDTH * BUFFER_HEIGHT];
    let (objects, light) = init_scene();

    let start_mouse = window.get_mouse_pos(MouseMode::Pass).unwrap_or((0.0, 0.0));
    let mut camera = Camera::new(Vec3::new(0.0, 2.0, 5.0), start_mouse);
    camera.pitch = -20.0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        camera.update(&window);
        render_frame(&mut buffer, &camera, &objects, &light);
        window
            .update_with_buffer(&buffer, BUFFER_WIDTH, BUFFER_HEIGHT)
            .unwrap();
    }
}
