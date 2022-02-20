use hai_pal::fs;
use std::ops::{Deref, DerefMut};
use wgpu::{Device, Queue};
use winit::dpi::LogicalSize;

use crate::{node::Node, texture::Texture, traits::Focusable, types::Vertex};

pub const SPRITE_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[derive(Debug)]
pub struct Sprite {
    /// path name relative to assets folder
    pub asset_path: String,
    /// loaded texture
    pub texture: Texture,
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,
    /// node
    node: Node,
}

impl Deref for Sprite {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl DerefMut for Sprite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl Sprite {
    pub fn from_asset(device: &Device, queue: &Queue, asset_path: String) -> Self {
        let bytes = match fs::read(format!("assets/{}", asset_path)) {
            Ok(v) => v,
            Err(err) => {
                println!(
                    "[Error][Sprite] load bytes from asset '{}': {}",
                    asset_path,
                    err.to_string()
                );
                vec![]
            }
        };

        let texture = Texture::from_bytes(device, queue, &bytes, asset_path.clone()).unwrap();

        let node = Node::new(asset_path.clone(), Default::default(), Default::default());

        Sprite {
            asset_path,
            texture,
            vertices: None,
            node,
        }
    }

    pub fn calculate_vertices(&mut self, logical_size: LogicalSize<f64>, scale_factor: f64) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let width =
            (self.texture.width as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let height = (self.texture.height as f64 * scale_factor)
            / (logical_size.height * scale_factor) as f64
            * 2.;

        let a = self.transform_to_global.a;
        let b = self.transform_to_global.b;
        let c = self.transform_to_global.c;
        let d = self.transform_to_global.d;
        let tx = self.transform_to_global.tx;
        let ty = 1. - self.transform_to_global.ty;

        let w1 = -self.anchor.x * width;
        let w0 = w1 + width;
        let h1 = (-1. + self.anchor.y) * height;
        let h0 = h1 + height;

        // left top
        let p0x = a * w1 + c * h1 + tx - 1.;
        let p0y = b * w1 + d * h1 + ty;

        // left bottom
        let p1x = a * w0 + c * h1 + tx - 1.;
        let p1y = b * w0 + d * h1 + ty;

        // right top
        let p2x = a * w0 + c * h0 + tx - 1.;
        let p2y = b * w0 + d * h0 + ty;

        // right bottom
        let p3x = a * w1 + c * h0 + tx - 1.;
        let p3y = b * w1 + d * h0 + ty;

        let v = [
            Vertex {
                position: [p0x as f32, p0y as f32, 0.0],
                tex_coords: [0., 1.],
            },
            Vertex {
                position: [p1x as f32, p1y as f32, 0.0],
                tex_coords: [1., 1.],
            },
            Vertex {
                position: [p2x as f32, p2y as f32, 0.0],
                tex_coords: [1., 0.],
            },
            Vertex {
                position: [p3x as f32, p3y as f32, 0.0],
                tex_coords: [0., 0.],
            },
        ];

        self.vertices = Some(v);
    }
}

impl Focusable for Sprite {
    fn contains(&self, x: i32, y: i32) -> bool {
        if x > self.translate.x
            && x < self.texture.width as i32 + self.translate.x
            && y > self.translate.y
            && y < self.texture.height as i32 + self.translate.y
        {
            return true;
        }
        false
    }
}

impl PartialEq for Sprite {
    fn eq(&self, other: &Sprite) -> bool {
        self.id == other.id
    }
}
