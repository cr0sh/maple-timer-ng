use std::ops::Range;

use assets_embedded::assets;
use assets_manager::asset::Png;
use image::{Bgra, GenericImageView, ImageBuffer, SubImage};
use once_cell::sync::OnceCell;
use rayon::prelude::*;

use crate::{view_ext::GenericImageViewExt, Matcher, RegularizedEqPixel};

pub struct JinHillahHpMatcher;

#[derive(Debug, Clone)]
pub struct JinHillahHpMatchResult {
    level: usize,
    remaining_pixels: u32,
}

impl JinHillahHpMatchResult {
    pub fn phase(&self) -> u32 {
        if self.level == 0 {
            1
        } else if self.level <= 4 {
            self.level as u32
        } else {
            unreachable!();
        }
    }

    pub fn hp_ratio(&self) -> f64 {
        self.remaining_pixels as f64 / MAX_PIXELS as f64
    }
}

static JIN_HILLAH_HP_ICON: OnceCell<ImageBuffer<Bgra<u8>, Vec<u8>>> = OnceCell::new();

const HP_BAR_COLORS: [(Bgra<u8>, Bgra<u8>); 4] = [
    (Bgra([102, 68, 204, 255]), Bgra([102, 68, 187, 255])),
    (Bgra([153, 102, 238, 255]), Bgra([153, 102, 221, 255])),
    (Bgra([34, 170, 170, 255]), Bgra([17, 153, 136, 255])),
    (Bgra([17, 119, 85, 255]), Bgra([17, 102, 68, 255])),
];

fn find_color(pair: (Bgra<u8>, Bgra<u8>)) -> usize {
    HP_BAR_COLORS
        .iter()
        .enumerate()
        .find(|(_, x)| x == &&pair)
        .map(|x| x.0)
        .unwrap_or(4)
}

const HP_X_OFFSET: Range<u32> = 40..796; // Note: Y is 9/10 and 8/9 for x = 1035
const MAX_PIXELS: u32 = HP_X_OFFSET.end - HP_X_OFFSET.start + 1;

impl<V: GenericImageView<Pixel = Bgra<u8>>> Matcher<V> for JinHillahHpMatcher {
    type MatchResult = JinHillahHpMatchResult;
    type CandidatesIter<'a>
    where
        V: 'a,
    = impl Iterator<Item = SubImage<&'a V::InnerImageView>> + 'a;

    fn init() {
        let asset = assets();
        let img = asset
            .load::<Png>("jinhillah_boss_hpbar_icon")
            .unwrap()
            .cloned()
            .0
            .to_bgra8();

        JIN_HILLAH_HP_ICON.set(img).unwrap();
    }

    fn view_dimensions(&self) -> (u32, u32) {
        (800, 37)
    }

    fn candidates_iter<'a>(&self, view: &'a V) -> Self::CandidatesIter<'a> {
        let icon = JIN_HILLAH_HP_ICON.get().unwrap();
        view.view(0, 0, view.width(), icon.height())
            .view_bounds_like((Matcher::<V>::view_dimensions(self).0, icon.height()), 1)
            .map(move |(x, y, w, h)| view.view(x, y, w, h))
    }

    fn check<'a>(&self, view: &SubImage<&'a V>) -> bool {
        let icon = JIN_HILLAH_HP_ICON.get().unwrap();
        view.view(3, 3, icon.width(), icon.height()).eq(icon)
    }

    fn match_image<'a>(&self, view: &SubImage<&'a V>) -> Option<Self::MatchResult> {
        let mut last_idx = 0;
        let mut changed_x = None;
        let it = HP_X_OFFSET
            .map(|x| {
                let p1 = view.get_pixel(x, 9);
                let p2 = view.get_pixel(x, 10);
                (p1, p2)
            })
            .chain(std::iter::once((
                view.get_pixel(HP_X_OFFSET.end, 8),
                view.get_pixel(HP_X_OFFSET.end, 9),
            )))
            .enumerate()
            .map(|(i, pair)| (i, find_color(pair)));

        for (i, idx) in it {
            if i == 0 {
                last_idx = idx;
                continue;
            }

            if idx == last_idx {
                continue;
            }

            if changed_x.is_some() {
                return None;
            }

            changed_x = Some(i as u32);
            last_idx = idx;
        }

        Some(JinHillahHpMatchResult {
            level: last_idx,
            remaining_pixels: changed_x.unwrap_or(MAX_PIXELS),
        })
    }
}

pub struct JinHillahReapMatcher(pub u32, pub u32);

#[allow(clippy::type_complexity)]
static JIN_HILLAH_REAP_MOTIONS: OnceCell<Vec<(ImageBuffer<Bgra<u8>, Vec<u8>>, usize)>> =
    OnceCell::new();

impl<V: GenericImageView<Pixel = Bgra<u8>> + std::marker::Sync> Matcher<V>
    for JinHillahReapMatcher
{
    type MatchResult = usize;

    type CandidatesIter<'a>
    where
        V: 'a,
    = impl Iterator<Item = SubImage<&'a V::InnerImageView>> + 'a;

    fn init() {
        JIN_HILLAH_REAP_MOTIONS.get_or_init(|| {
            let imgs = assets()
                .load_dir::<Png>("jinhillah_reap", false)
                .unwrap()
                .iter()
                .take(12)
                .map(Result::unwrap);
            imgs.map(|asset| {
                let img = asset.cloned().0.to_bgra8();
                let img = ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
                    *img.get_pixel(x / 2 + img.width() / 4, y / 2 + img.height() / 4)
                });
                let cnt = img.pixels().filter(|x| x.good_pixel()).count();
                (img, cnt)
            })
            .collect::<Vec<_>>()
        });
    }

    fn view_dimensions(&self) -> (u32, u32) {
        (self.0 / 2, self.1 / 2)
    }

    fn candidates_iter<'a>(&self, view: &'a V) -> Self::CandidatesIter<'a> {
        if (self.0, self.1) == view.dimensions() {
            const RANGE: i32 = 2;
            let offsets = (-RANGE..=RANGE).flat_map(|x| (-RANGE..=RANGE).map(move |y| (x, y)));
            let dims = JIN_HILLAH_REAP_MOTIONS
                .get()
                .unwrap()
                .first()
                .unwrap()
                .0
                .dimensions();
            let center = ((view.width() - dims.0) / 2, (view.height() - dims.1) / 2);
            Some(
                offsets
                    .map(move |(ox, oy)| {
                        ((center.0 as i32 + ox) as u32, (center.1 as i32 + oy) as u32)
                    })
                    .map(move |(x, y)| view.view(x, y, dims.0, dims.1)),
            )
            .into_iter()
            .flatten()
        } else {
            None.into_iter().flatten()
        }
    }

    fn match_image<'a>(&self, view: &SubImage<&'a V>) -> Option<Self::MatchResult> {
        JIN_HILLAH_REAP_MOTIONS
            .get()
            .unwrap()
            .par_iter()
            .enumerate()
            .find_map_any(|(i, (img, good_pixels))| {
                let mut bad_count_remaining = (*good_pixels as f64 * 0.3) as usize;
                for (x, y, p) in view.pixels() {
                    let q = img.get_pixel(x, y);
                    if q.good_pixel() && q != &p {
                        bad_count_remaining -= 1;
                        if bad_count_remaining == 0 {
                            break;
                        }
                    }
                }
                if bad_count_remaining > 0 {
                    return Some(i);
                }
                None
            })
    }
}
