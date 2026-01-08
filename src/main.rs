use minifb::{Key, MouseMode, Window, WindowOptions};
use rayon::prelude::*;

mod aabb;
mod bvh;
mod camera;
mod constants;
mod material;
mod objects;
mod ray;
mod vec3;

use bvh::Bvh;
use camera::Camera;
use constants::*;
use material::Material;
use objects::{load_obj, Object, PointLight, Sphere, Triangle};
use ray::Ray;
use vec3::Vec3;

fn compute_pixel_color(mut ray: Ray, objects: &[Object], bvh: &Bvh, light: &PointLight) -> u32 {
    let mut final_color = Vec3::new(0.0, 0.0, 0.0);
    let mut throughput = Vec3::new(1.0, 1.0, 1.0);

    for bounce in 0..MAX_BOUNCES {
        if let Some(hit) = bvh.intersect(&ray, objects) {
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

            let view_dir = -ray.direction;
            let n_dot_v = hit.normal.dot(view_dir).max(0.0);
            let fresnel =
                hit.material.specular + (1.0 - hit.material.specular) * (1.0 - n_dot_v).powi(5);

            let light_vec = light.position - hit.point;
            let dist_sq = light_vec.len_sq();
            let dist = dist_sq.sqrt();
            let light_dir = light_vec * (1.0 / dist);

            let shadow_ray = Ray::new(hit.point + hit.normal * 0.001, light_dir);
            let mut in_shadow = false;

            if let Some(shadow_hit) = bvh.intersect(&shadow_ray, objects) {
                let dist_to_blocker_sq = (shadow_hit.point - shadow_ray.origin).len_sq();
                if dist_to_blocker_sq < dist_sq && shadow_hit.material.emission.len() == 0.0 {
                    in_shadow = true;
                }
            }

            if !in_shadow {
                let n_dot_l = hit.normal.dot(light_dir).max(0.0);
                let attenuation = 1.0 / dist_sq;
                let incoming_light = light.color * light.intensity * attenuation;

                let halfway_dir = (light_dir + view_dir).normalize();
                let n_dot_h = hit.normal.dot(halfway_dir).max(0.0);
                let specular_intensity = n_dot_h.powf(hit.material.shininess);
                let specular_color = Vec3::new(1.0, 1.0, 1.0) * specular_intensity * fresnel;

                let k_diffuse = 1.0 - fresnel;
                let diffuse_color = albedo.mul_vec(incoming_light) * n_dot_l * k_diffuse;

                final_color = final_color
                    + throughput.mul_vec(diffuse_color + specular_color.mul_vec(incoming_light));
            }

            if hit.material.specular > 0.0 || fresnel > 0.05 {
                let reflect = ray.direction - 2.0 * ray.direction.dot(hit.normal) * hit.normal;
                throughput = throughput * fresnel;
                ray = Ray::new(hit.point + hit.normal * 0.001, reflect.normalize());
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

fn render_frame(
    buffer: &mut [u32],
    camera: &Camera,
    objects: &[Object],
    bvh: &Bvh,
    light: &PointLight,
) {
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

                let ray = Ray::new(camera.pos, direction);
                *pixel = compute_pixel_color(ray, objects, bvh, light);
            }
        });
}

fn init_scene() -> (Vec<Object>, Bvh, PointLight) {
    let light = PointLight {
        position: Vec3::new(2.0, 5.0, 3.0),
        color: Vec3::new(1.0, 1.0, 0.9),
        intensity: 50.0,
    };

    let mat_mirror = Material {
        albedo: Vec3::new(0.9, 0.9, 0.9),
        emission: Vec3::new(0.0, 0.0, 0.0),
        specular: 0.95,
        shininess: 1000.0,
        checkered: false,
    };
    let mat_gold = Material {
        albedo: Vec3::new(0.8, 0.6, 0.2),
        emission: Vec3::new(0.0, 0.0, 0.0),
        specular: 0.6,
        shininess: 64.0,
        checkered: false,
    };
    let mat_checker = Material {
        albedo: Vec3::new(0.9, 0.9, 0.9),
        emission: Vec3::new(0.0, 0.0, 0.0),
        specular: 0.1,
        shininess: 32.0,
        checkered: true,
    };
    let mat_bulb = Material {
        albedo: Vec3::new(0.0, 0.0, 0.0),
        emission: light.color * light.intensity,
        specular: 0.0,
        shininess: 0.0,
        checkered: false,
    };

    let mut objects = vec![
        Object::Sphere(Sphere {
            center: light.position,
            radius: 0.2,
            mat: mat_bulb,
        }),
        Object::Sphere(Sphere {
            center: Vec3::new(2.0, 1.0, -1.0),
            radius: 1.0,
            mat: mat_mirror,
        }),
        Object::Triangle(Triangle {
            v0: Vec3::new(-20.0, 0.0, -20.0),
            v1: Vec3::new(-20.0, 0.0, 20.0),
            v2: Vec3::new(20.0, 0.0, 20.0),
            mat: mat_checker,
        }),
        Object::Triangle(Triangle {
            v0: Vec3::new(-20.0, 0.0, -20.0),
            v1: Vec3::new(20.0, 0.0, 20.0),
            v2: Vec3::new(20.0, 0.0, -20.0),
            mat: mat_checker,
        }),
    ];

    println!("Loading OBJ model...");
    let mesh_objects = load_obj(
        "C:/Users/user/Desktop/rust/raytracer/assets/suzane.obj",
        Vec3::new(-1.5, 2.0, 0.0),
        1.0,
        mat_gold,
    );
    objects.extend(mesh_objects);

    println!("Building BVH for {} objects...", objects.len());
    let bvh = Bvh::build(&objects);
    println!("Scene initialized.");

    (objects, bvh, light)
}

fn main() {
    let mut window = Window::new(
        "Raytracer Rust",
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

    let (objects, bvh, light) = init_scene();

    let start_mouse = window.get_mouse_pos(MouseMode::Pass).unwrap_or((0.0, 0.0));
    let mut camera = Camera::new(Vec3::new(0.0, 2.0, 5.0), start_mouse);
    camera.pitch = -20.0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        camera.update(&window);
        render_frame(&mut buffer, &camera, &objects, &bvh, &light);
        window
            .update_with_buffer(&buffer, BUFFER_WIDTH, BUFFER_HEIGHT)
            .unwrap();
    }
}
