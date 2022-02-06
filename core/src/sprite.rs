use std::{
    fs,
    ops::{Deref, DerefMut},
};
use winit::dpi::LogicalSize;

use crate::{node::Node, renderer::Renderer, texture::Texture, types::Vertex};

pub const SPRITE_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

#[derive(Debug)]
pub struct Sprite<'a> {
    /// path name relative to assets folder
    pub asset_path: &'a str,
    /// loaded texture
    pub texture: Texture,
    /// calculated vertices
    pub vertices: Option<[Vertex; 4]>,
    /// node
    node: Node<'a>,
}

impl<'a> Deref for Sprite<'a> {
    type Target = Node<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<'a> DerefMut for Sprite<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl<'a> Sprite<'a> {
    pub fn from_asset(renderer: &Renderer, asset_path: &'a str) -> Self {
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

        let texture =
            Texture::from_bytes(&renderer.device, &renderer.queue, &bytes, asset_path).unwrap();

        let node = Node::new(Some(asset_path), Default::default(), Default::default());

        Sprite {
            asset_path,
            texture,
            vertices: None,
            node,
        }
    }

    pub fn move_to(&mut self, tx: i32, ty: i32) {
        self.transform.tx = tx;
        self.transform.ty = ty;
    }

    pub fn calculate_vertices(&mut self, logical_size: LogicalSize<f64>, scale_factor: f64) {
        // (image_logical_size * image_scale_factor) / (screen_logical_size * screen_scale_factor) * coordinate_factor
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let width =
            (self.texture.width as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let height = (self.texture.height as f64 * scale_factor)
            / (logical_size.height * scale_factor) as f64
            * 2.;

        let a = self.transform.a;
        let b = self.transform.b;
        let c = self.transform.c;
        let d = self.transform.d;
        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let tx =
            (self.transform.tx as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let ty = 1.
            - (self.transform.ty as f64 * scale_factor) / (logical_size.height * scale_factor) * 2.;

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

        // println!(
        //     "{} {} {} {} {} {} {} {}",
        //     p0x, p0y, p3x, p3y, p1x, p1y, p2x, p2y,
        // );

        self.vertices = Some(v);
    }
}
