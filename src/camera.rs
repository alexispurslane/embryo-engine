use crate::entity::transform_component::PitchYawRoll;
use crate::utils::Degrees;

pub trait Camera {
    fn displace(&mut self, v: glam::Vec3, dt: u128);
    fn rotate(&mut self, rot: PitchYawRoll);
    fn view(&self) -> glam::Mat4;
    fn project(&self, width: i32, height: i32) -> glam::Mat4;
}

pub struct FlyingCamera {
    speed: f32,
    up: glam::Vec3,
    pos: glam::Vec3,
    front: glam::Vec3,
    rotation: PitchYawRoll,
    fov: Degrees,
}

impl FlyingCamera {
    pub fn new(
        speed: f32,
        up: glam::Vec3,
        pos: glam::Vec3,
        front: glam::Vec3,
        rotation: PitchYawRoll,
        fov: Degrees,
    ) -> Self {
        Self {
            speed,
            up,
            pos,
            front,
            rotation,
            fov,
        }
    }
}

impl Camera for FlyingCamera {
    fn displace(&mut self, v: glam::Vec3, dt: u128) {
        self.pos += (self.speed * (dt as f32 / 1000.0))
            * ((v.x * self.front)
                + (v.y * self.front.cross(self.up).normalize())
                + (v.z * self.up));
    }

    fn rotate(&mut self, rot: PitchYawRoll) {
        let PitchYawRoll { pitch, yaw, .. } = rot;
        self.rotation.yaw = (self.rotation.yaw + yaw) % 360.0;
        self.rotation.pitch = (self.rotation.pitch + pitch).clamp(-89.0, 89.0);
        self.front = glam::vec3(
            self.rotation.yaw.to_radians().cos() * self.rotation.pitch.to_radians().cos(),
            self.rotation.pitch.to_radians().sin(),
            self.rotation.yaw.to_radians().sin() * self.rotation.pitch.to_radians().cos(),
        )
        .normalize();
    }

    fn view(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(self.pos, self.pos + self.front, self.up)
    }

    fn project(&self, width: i32, height: i32) -> glam::Mat4 {
        glam::Mat4::perspective_rh_gl(
            self.fov.to_radians(),
            width as f32 / height as f32,
            0.1,
            100.0,
        )
    }
}
