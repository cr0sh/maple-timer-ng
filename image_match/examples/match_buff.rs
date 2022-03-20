use assets_embedded::assets;
use assets_manager::{asset::Png, AssetCache};
use image::{Bgra, DynamicImage, GenericImageView, ImageBuffer};
use image_match::{buff::BuffMatcher, Matcher};

fn main() {
    let examples = AssetCache::new("example_assets").unwrap();
    <BuffMatcher as Matcher<ImageBuffer<Bgra<u8>, Vec<u8>>>>::init();
    let buff_matcher = BuffMatcher::new(
        assets()
            .load::<Png>("v_buficon")
            .unwrap()
            .cloned()
            .0
            .to_bgra8(),
        0.6,
        (1280, 720),
    );

    let img = examples
        .load::<Png>("test/2")
        .unwrap()
        .cloned()
        .0
        .to_bgra8();
    let mut palette = img.clone();
    let colors = vec![
        Bgra([255u8, 0u8, 0u8, 255u8]),
        Bgra([0u8, 255u8, 0u8, 255u8]),
        Bgra([0u8, 0u8, 255u8, 255u8]),
        Bgra([255u8, 255u8, 0u8, 255u8]),
        Bgra([0u8, 255u8, 255u8, 255u8]),
        Bgra([255u8, 0u8, 255u8, 255u8]),
    ];
    buff_matcher
        .candidates_iter(&img)
        .enumerate()
        .map(|(i, img)| {
            let (x, y, w, h) = img.bounds();
            let check = buff_matcher.check(&img);
            assert_eq!(w, 32);
            assert_eq!(h, 32);
            for x in x..(x + w) {
                for y in y..(y + h) {
                    *palette.get_pixel_mut(x, y) = colors[i as usize % 6];
                }
            }
            (img, check)
        })
        .filter(|x| x.1)
        .for_each(|x| {
            let result = buff_matcher.match_image(&x.0);
            println!("bounds: {:?}, ok: {}", x.0.bounds(), result.is_some());
        });

    DynamicImage::ImageBgra8(palette)
        .to_rgba8()
        .save("out/buff.png")
        .unwrap();
}
