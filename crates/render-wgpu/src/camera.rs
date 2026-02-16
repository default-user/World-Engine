use glam::{Mat4, Vec3};

/// Fly camera with position, yaw, pitch, and projection parameters.
/// Camera motion is NOT deterministic ... it exists outside the kernel boundary.
pub struct FlyCamera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 10.0, 15.0),
            yaw: -90.0_f32.to_radians(),
            pitch: -30.0_f32.to_radians(),
            fov: 60.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
            speed: 10.0,
            sensitivity: 0.003,
        }
    }
}

impl FlyCamera {
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn move_forward(&mut self, dt: f32) {
        let fwd = self.forward();
        self.position += fwd * self.speed * dt;
    }

    pub fn move_backward(&mut self, dt: f32) {
        let fwd = self.forward();
        self.position -= fwd * self.speed * dt;
    }

    pub fn move_left(&mut self, dt: f32) {
        let right = self.right();
        self.position -= right * self.speed * dt;
    }

    pub fn move_right(&mut self, dt: f32) {
        let right = self.right();
        self.position += right * self.speed * dt;
    }

    pub fn move_up(&mut self, dt: f32) {
        self.position.y += self.speed * dt;
    }

    pub fn move_down(&mut self, dt: f32) {
        self.position.y -= self.speed * dt;
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * self.sensitivity;
        self.pitch -= dy * self.sensitivity;
        self.pitch = self.pitch.clamp(
            -89.0_f32.to_radians(),
            89.0_f32.to_radians(),
        );
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    pub fn view_projection(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera() {
        let cam = FlyCamera::default();
        assert!(cam.position.y > 0.0);
        let vp = cam.view_projection();
        // Should produce a valid matrix (no NaN)
        assert!(!vp.col(0).x.is_nan());
    }

    #[test]
    fn camera_movement() {
        let mut cam = FlyCamera::default();
        let start = cam.position;
        cam.move_forward(1.0);
        assert_ne!(cam.position, start);
    }
}
