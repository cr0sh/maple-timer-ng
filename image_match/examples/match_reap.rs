use std::ops::ControlFlow;

use assets_manager::{asset::Png, AssetCache};
use image::{Bgra, GenericImageView, ImageBuffer};
use image_match::{jinhillah::JinHillahReapMatcher, Matcher};

fn main() {
    let assets = AssetCache::new("example_assets").unwrap();

    let reap_matcher = JinHillahReapMatcher(1280, 720);
    <JinHillahReapMatcher as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::init();
    let imgs = assets
        .load_dir::<Png>("reap", false)
        .unwrap()
        .iter()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    for img in imgs {
        let name = img.id();
        let img = img.cloned().0.to_bgra8();
        println!("{}", name);
        let mut _bound = Default::default();
        reap_matcher
            .candidates_iter(&img)
            .map(|x| {
                _bound = x.bounds();
                let check = reap_matcher.check(&x);
                (x, check)
            })
            .filter(|x| x.1)
            .try_for_each(|x| {
                let result = reap_matcher.match_image(&x.0);
                println!("name: {}, {:?}", name, result);
                if result.is_some() {
                    return ControlFlow::Break(());
                };

                return ControlFlow::Continue(());
            });
    }
}
