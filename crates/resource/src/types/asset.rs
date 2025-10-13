use std::sync::{Arc, Weak};

use moyu_pal::url::Url;

use crate::types::Texture;

#[derive(Debug, Clone)]
pub struct AssetId {
    url: Url,
    kind: AssetKind,
    asset: Weak<Asset>,
}

impl PartialEq for AssetId {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url && self.kind == other.kind
    }
}

impl Eq for AssetId {}

impl std::hash::Hash for AssetId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
        self.kind.hash(state);
    }
}

impl AssetId {
    pub fn url(&self) -> &Url {
        &self.url
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

pub(crate) fn create_asset_id(kind: AssetKind, url: Url) -> AssetId {
    AssetId {
        url,
        kind,
        asset: Weak::new(),
    }
}
