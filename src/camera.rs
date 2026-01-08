use crate::constants::{MOUSE_SENSITIVITY, MOVE_SPEED};
use crate::vec3::Vec3;
use minifb::{Key, MouseButton, MouseMode, Window};

pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub last_mouse_pos: (f32, f32),
}

impl Camera {
    pub fn new(pos: Vec3, start_mouse: (f32, f32)) -> Self {
        Self {
            pos,
            yaw: -90.0,
            pitch: 0.0,
            last_mouse_pos: start_mouse,
        }
    }
    pub fn update(&mut self, window: &Window) {
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
    pub fn get_vectors(&self) -> (Vec3, Vec3, Vec3) {
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
