use std::sync::Arc;

use glam::Vec3;
use hai_pal::sync::RwLock;

use crate::nodes::Sprite;
#[cfg(feature = "text")]
use crate::nodes::Text;
use crate::traits::{Focusable, Node, NodeBaseTrait, RendererUpdatePayload};

use super::constants::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH};
use super::walk::walk_nodes_bottom_top;

pub struct HitTestResult {
    pub target: Arc<RwLock<dyn Node>>,
    pub parent_ids: Vec<u32>,
}

impl PartialEq for HitTestResult {
    fn eq(&self, other: &Self) -> bool {
        self.target.read().base().id() == other.target.read().base().id()
    }
}

pub fn hit_test<'a>(
    root_node: &Arc<RwLock<dyn Node>>,
    global_logical_x: f32,
    global_logical_y: f32,
    upload_payload: &RendererUpdatePayload,
) -> Option<HitTestResult> {
    let root_node = root_node.read();
    let mut focused_node = None;

    walk_nodes_bottom_top(
        &*root_node,
        &mut |child, _, parent_ids| {
            let child_ref = child.read();

            let p = child_ref
                .base()
                .global_transform()
                .inverse()
                .transform_point3(Vec3::new(
                    global_logical_x / VIEWPORT_WIDTH,
                    global_logical_y / VIEWPORT_HEIGHT,
                    1.0,
                ));

            let local_logical_x = p.x * VIEWPORT_WIDTH;
            let local_logical_y = p.y * VIEWPORT_HEIGHT;

            let hit = match child_ref.node_type() {
                "sprite" => {
                    let sprite = child_ref.as_any().downcast_ref::<Sprite>().unwrap();

                    // check if pointer is over the sprite
                    let hit = sprite.contains(local_logical_x, local_logical_y, upload_payload);

                    (hit, Some(sprite.base().label().clone()))
                }
                #[cfg(feature = "text")]
                "text" => {
                    let text = child_ref.as_any().downcast_ref::<Text>().unwrap();

                    // check if pointer is over the text
                    let hit = text.contains(local_logical_x, local_logical_y, &upload_payload);

                    (hit, Some(text.base().label().clone()))
                }
                _ => (false, None),
            };

            if hit.0 {
                focused_node = Some(HitTestResult {
                    target: child.clone(),
                    parent_ids: parent_ids.to_vec(),
                });
            }

            hit.0
        },
        &[],
        true,
    );

    focused_node
}
