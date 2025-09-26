use std::sync::{Arc, Weak};

use crate::types::Texture;

#[derive(Debug, Clone)]
pub struct AssetId {
    src: String,
    kind: AssetKind,
    asset: Weak<Asset>,
}

impl PartialEq for AssetId {
    fn eq(&self, other: &Self) -> bool {
        self.src == other.src && self.kind == other.kind
    }
}

impl Eq for AssetId {}

impl std::hash::Hash for AssetId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.src.hash(state);
        self.kind.hash(state);
    }
}

impl AssetId {
    pub fn src(&self) -> &str {
        &self.src
    }

    pub fn kind(&self) -> AssetKind {
        self.kind
    }

    pub(crate) fn attach_asset(&mut self, asset: &Arc<Asset>) {
        self.asset = Arc::downgrade(asset);
    }

    pub fn asset(&self) -> Option<Arc<Asset>> {
        self.asset.upgrade()
    }

    pub fn asset_unchecked(&self) -> Arc<Asset> {
        self.asset.upgrade().unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    Texture,
    Audio,
    Video,
    Font,
    Plain,
}

#[derive(Debug)]
pub enum Asset {
    Texture(Arc<Texture>),
    Audio,         // TODO: add audio type
    Video,         // TODO: add video type
    Font,          // TODO: add font type
    Plain(String), // TODO: add plain type
}

pub(crate) fn create_asset_id(kind: AssetKind, src: String) -> AssetId {
    AssetId {
        src,
        kind,
        asset: Weak::new(),
    }
}
