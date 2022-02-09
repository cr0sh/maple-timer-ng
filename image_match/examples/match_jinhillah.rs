use assets_manager::{asset::Png, AssetCache};
use image::{Bgra, GenericImageView, ImageBuffer};
use image_match::{
    jinhillah::{JinHillahHpMatchResult, JinHillahHpMatcher},
    Matcher,
};

fn main() {
    let assets = AssetCache::new("example_assets").unwrap();
    let hp_matcher = JinHillahHpMatcher;
    <JinHillahHpMatcher as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::init();

    let imgs = assets
        .load_dir::<Png>("bars", false)
        .unwrap()
        .iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for img in imgs {
        let name = img.id();
        let img = img.cloned().0.to_bgra8();
        let mut _bound = Default::default();
        hp_matcher
            .candidates_iter(&img)
            .map(|x| {
                _bound = x.bounds();
                let check = hp_matcher.check(&x);
                (x, check)
            })
            .filter(|x| x.1)
            .for_each(|x| {
                let result = hp_matcher.match_image(&x.0);
                println!(
                    "name: {}, {:?}, ratio {:?}",
                    name,
                    result,
                    result.as_ref().map(JinHillahHpMatchResult::hp_ratio)
                );
            })
    }
}
