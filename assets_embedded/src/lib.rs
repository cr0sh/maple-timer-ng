use assets_manager::{
    source::{embed, Embedded},
    AssetCache,
};
use once_cell::sync::OnceCell;

static ASSET_CACHE: OnceCell<AssetCache<Embedded<'static>>> = OnceCell::new();

pub fn assets() -> &'static AssetCache<Embedded<'static>> {
    ASSET_CACHE.get_or_init(|| AssetCache::with_source(Embedded::from(embed!("assets"))))
}
