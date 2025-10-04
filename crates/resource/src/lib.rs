mod loaders;
pub mod types;
mod utils;

use dashmap::DashMap;
use std::sync::Arc;
use wgpu::{Device, Queue};

use crate::loaders::*;
use crate::types::*;

#[derive(Debug)]
pub struct ResourceManager {
    device: Device,
    queue: Queue,
    assets_map: Arc<DashMap<Arc<AssetId>, Arc<Asset>>>,
}

impl ResourceManager {
    pub fn new(device: Device, queue: Queue) -> Self {
        let assets_map = Arc::new(DashMap::new());

        {
            let assets_map_weak = Arc::downgrade(&assets_map);
            moyu_pal::task::spawn(async move {
                loop {
                    moyu_pal::time::sleep(std::time::Duration::from_secs(10)).await;

                    let assets_map = match assets_map_weak.upgrade() {
                        Some(v) => v,
                        None => {
                            log::debug!("resource manager dropped, stop sweeping thread");
                            break;
                        }
                    };

                    log::debug!("sweeping unused assets...");
                    sweep(&assets_map);
                    log::debug!("sweeping unused assets done");
                }
            });
        }

        Self {
            device,
            queue,
            assets_map,
        }
    }

    pub fn try_get_asset(&self, asset_id: &Arc<AssetId>) -> Option<Arc<Asset>> {
        if let Some(asset) = self.assets_map.get(asset_id) {
            return Some(asset.clone());
        }

        None
    }

    pub fn load_asset(&self, kind: AssetKind, src: &str) -> Arc<AssetId> {
        let mut asset_id = create_asset_id(kind, src.to_string());

        if let Some(asset) = self.assets_map.get(&asset_id) {
            return asset.key().clone();
        }

        match kind {
            AssetKind::Texture => {
                let texture = load_texture(&self.device, &self.queue, src);
                let asset = Arc::new(Asset::Texture(texture));
                asset_id.attach_asset(&asset);
                let asset_id = Arc::new(asset_id);
                self.assets_map.insert(asset_id.clone(), asset);
                asset_id
            }
            _ => {
                todo!()
            }
        }
    }

    pub fn insert_asset(&self, kind: AssetKind, src: &str, data: Vec<u8>) -> Arc<AssetId> {
        let mut asset_id = create_asset_id(kind, src.to_string());

        let asset = match kind {
            AssetKind::Texture => {
                let texture = Arc::new(Texture::new());
                if let Err(err) =
                    load_image_to_texture(&texture, &self.device, &self.queue, &data, None)
                {
                    log::error!("failed to load texture from raw image data: {}", err);
                    texture.set_status(TextureStatus::Error);
                }
                Asset::Texture(texture)
            }
            _ => todo!(),
        };
        let asset = Arc::new(asset);
        asset_id.attach_asset(&asset);
        let asset_id = Arc::new(asset_id);
        self.assets_map.insert(asset_id.clone(), asset);
        asset_id
    }
}

fn sweep(assets_map: &DashMap<Arc<AssetId>, Arc<Asset>>) {
    assets_map.retain(|k, _| {
        let key_ref_count = Arc::strong_count(k);
        if key_ref_count > 1 {
            true
        } else {
            log::debug!("drop unused texture {:?} {}", *k, key_ref_count);
            false
        }
    });
}
