use std::sync::{Arc, Weak};

use moyu_pal::url::Url;

use crate::types::{Plain, Texture};

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
    /// Straight (non-premultiplied) alpha source, premultiplied on upload.
    Texture,
    /// Already premultiplied alpha source, uploaded as-is.
    ///
    /// Kept as a separate kind so the same path requested both ways does not
    /// share one cached texture: the two inputs would produce different pixels.
    TexturePremultiplied,
    Audio,
    Video,
    Font,
    Plain,
}

impl AssetKind {
    /// Maps a texture kind to whether its source alpha is already premultiplied.
    pub fn premultiplied(self) -> Option<bool> {
        match self {
            Self::Texture => Some(false),
            Self::TexturePremultiplied => Some(true),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Asset {
    Texture(Arc<Texture>),
    Audio, // TODO: add audio type
    Video, // TODO: add video type
    Font,  // TODO: add font type
    Plain(Arc<Plain>),
}

pub(crate) fn create_asset_id(kind: AssetKind, url: Url) -> AssetId {
    AssetId {
        url,
        kind,
        asset: Weak::new(),
    }
}
