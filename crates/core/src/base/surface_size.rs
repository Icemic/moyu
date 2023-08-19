use winit::dpi::PhysicalSize;

/// Size struct for surface, it handles all size related stuff
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub struct SurfaceSize {
    /// logical width
    width: f64,
    /// logical height
    height: f64,
    /// scale factor, aka _device pixel ratio_
    scale_factor: f64,
}

#[allow(dead_code)]
impl SurfaceSize {
    pub fn new(logical_width: f64, logical_height: f64, scale_factor: f64) -> Self {
        Self {
            width: logical_width,
            height: logical_height,
            scale_factor,
        }
    }

    pub fn from_physical_size(physical_size: &PhysicalSize<u32>, scale_factor: f64) -> Self {
        let width = physical_size.width as f64 / scale_factor;
        let height = physical_size.height as f64 / scale_factor;

        Self {
            width,
            height,
            scale_factor,
        }
    }

    pub fn logical_size(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    pub fn physical_size(&self) -> (u32, u32) {
        let width = (self.width * self.scale_factor) as u32;
        let height = (self.height * self.scale_factor) as u32;
        (width, height)
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn set_logical_size(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    pub fn set_physical_size(&mut self, width: u32, height: u32) {
        self.width = width as f64 / self.scale_factor;
        self.height = height as f64 / self.scale_factor;
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }
}
