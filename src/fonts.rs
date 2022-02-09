use assets_manager::{
    loader::{BytesLoader, Loader},
    Asset,
};

#[derive(Clone)]
pub struct RawFont(pub Vec<u8>);

impl Asset for RawFont {
    const EXTENSIONS: &'static [&'static str] = &["otf", "ttf"];
    type Loader = FontLoader;
}

pub enum FontLoader {}

impl Loader<RawFont> for FontLoader {
    fn load(
        content: std::borrow::Cow<[u8]>,
        ext: &str,
    ) -> Result<RawFont, assets_manager::BoxedError> {
        BytesLoader::load(content, ext).map(RawFont)
    }
}
