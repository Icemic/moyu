use std::sync::{Arc, Mutex};

use winit::dpi::LogicalSize;

use crate::types::Transform;

pub trait Node {
    #[allow(dead_code)]
    fn get_child(&self, index: usize) -> Option<Arc<Mutex<dyn Node>>> {
        if let Some(child) = self.children.get(index) {
            return Some(child.clone());
        }
        None
    }

    fn add_child(&mut self, child: Self)
    where
        Self: Sized,
    {
        self.children.push(Arc::new(Mutex::new(child)));
    }

    #[allow(dead_code)]
    fn insert_child(&mut self, index: usize, child: Self)
    where
        Self: Sized,
    {
        self.children.insert(index, Arc::new(Mutex::new(child)));
    }

    #[allow(dead_code)]
    fn remove_child(&mut self, child: Arc<Mutex<Self>>) -> Option<Arc<Mutex<dyn Node>>>
    where
        Self: Sized,
    {
        if let Some(index) = self.children.iter().position(|item| {
            let l = item.lock().unwrap();
            let r = child.lock().unwrap();
            *l == *r
        }) {
            return Some(self.children.remove(index));
        }
        None
    }

    #[allow(dead_code)]
    fn remove_child_at(&mut self, index: usize) -> Option<Arc<Mutex<dyn Node>>> {
        if index < self.children.len() {
            return Some(self.children.remove(index));
        }
        None
    }

    fn move_to(&mut self, x: i32, y: i32) {
        self.translate.x = x;
        self.translate.y = y;
    }

    fn calculate_transform(
        &mut self,
        parent_transform: &Transform,
        logical_size: LogicalSize<f64>,
        scale_factor: f64,
    ) {
        let x = self.translate.x;
        let y = self.translate.y;

        // TODO: use scale_factor as image_scale_factor means force stretch, to be fixed
        let tx = (x as f64 * scale_factor) / (logical_size.width * scale_factor) * 2.;
        let ty = (y as f64 * scale_factor) / (logical_size.height * scale_factor) * 2.;

        self.transform.tx = tx;
        self.transform.ty = ty;

        // TODO: rotate, scale and skew

        // refresh global transform matrix
        let mut transform_to_global = parent_transform.clone();
        transform_to_global.multiply(self.transform);
        self.transform_to_global = transform_to_global;
    }
}
