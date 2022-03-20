use assets_embedded::assets;
use assets_manager::asset::Png;
use image::{Bgra, GenericImageView, ImageBuffer, Pixel, SubImage};
use once_cell::sync::OnceCell;
use smallvec::SmallVec;

use crate::{GenericImageViewExt, Matcher};

static BUFF_EDGES: OnceCell<Vec<(u32, u32)>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct BuffMatcher {
    icon: ImageBuffer<Bgra<u8>, Vec<u8>>,
    threshold: f64,
    dims: (u32, u32),
}

impl BuffMatcher {
    pub fn new(icon: ImageBuffer<Bgra<u8>, Vec<u8>>, threshold: f64, dims: (u32, u32)) -> Self {
        assert!(threshold <= 1.0);
        assert_eq!(icon.width(), 32);
        assert_eq!(icon.height(), 32);
        Self {
            icon,
            threshold,
            dims,
        }
    }

    fn has_edges<I>(subimage: &SubImage<&I>) -> bool
    where
        I: GenericImageView<Pixel = Bgra<u8>>,
    {
        let mut fail = 0;
        for &(x, y) in BUFF_EDGES.get().unwrap() {
            let (r, g, b, a) = subimage.get_pixel(x, y).channels4();
            const BLACK: u8 = 50;
            let max = u8::MIN;
            if r > BLACK || g > BLACK || b > BLACK || a < max {
                fail += 1;
                if fail >= BUFF_EDGES.get().unwrap().len() / 3 {
                    return false;
                }
            }
        }
        true
    }

    fn is_opaque(&self, x: u32, y: u32) -> bool {
        self.icon.get_pixel(x, y).channels4().3 == u8::MAX
    }

    fn ncc(x: &[f64], y: &[f64]) -> f64 {
        let n = x.len() as f64;
        let x_mean = x.iter().sum::<f64>() / n;
        let y_mean = y.iter().sum::<f64>() / n;
        let x_variance = x.iter().map(|x| (x - x_mean).powi(2)).sum::<f64>() / n;
        let y_variance = y.iter().map(|y| (y - y_mean).powi(2)).sum::<f64>() / n;
        let stddev_prod = (x_variance * y_variance).sqrt() + f64::EPSILON;

        x.iter()
            .zip(y.iter())
            .map(|(x, y)| (x - x_mean) * (y - y_mean) / stddev_prod)
            .sum::<f64>()
            / n
    }

    fn row_ncc<I>(&self, target: &SubImage<&I>, y: u32) -> (f64, f64, f64)
    where
        I: GenericImageView<Pixel = Bgra<u8>>,
    {
        let w = self.icon.width();
        let x_iter = (0..w).filter(|&x| self.is_opaque(x, y));
        macro_rules! select_channel {
            ($img:expr, $i:tt) => {
                x_iter
                    .clone()
                    .map(|x| Into::<f64>::into($img.get_pixel(x, y).channels4().$i))
                    .collect::<SmallVec<[f64; 32]>>()
            };
        }
        let (icon_r, target_r) = (select_channel!(self.icon, 0), select_channel!(target, 0));
        let (icon_g, target_g) = (select_channel!(self.icon, 1), select_channel!(target, 1));
        let (icon_b, target_b) = (select_channel!(self.icon, 2), select_channel!(target, 2));

        (
            Self::ncc(&icon_r, &target_r),
            Self::ncc(&icon_g, &target_g),
            Self::ncc(&icon_b, &target_b),
        )
    }
}

impl<V: GenericImageView<Pixel = Bgra<u8>>> Matcher<V> for BuffMatcher {
    type MatchResult = ();

    type CandidatesIter<'a> = impl Iterator<Item = SubImage<&'a V::InnerImageView>> + 'a where V: 'a;

    fn init() {
        let im = assets()
            .load::<Png>("buff_edge")
            .unwrap()
            .cloned()
            .0
            .to_bgra8();
        BUFF_EDGES
            .set(
                im.enumerate_pixels()
                    .filter(|(_, _, p)| p.0[3] == 255)
                    .map(|(x, y, _)| (x, y))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
    }

    fn view_dimensions(&self) -> (u32, u32) {
        self.dims
    }

    fn candidates_iter<'a>(&self, view: &'a V) -> Self::CandidatesIter<'a>
    where
        V: GenericImageView<Pixel = Bgra<u8>>,
    {
        view.view(
            (view.width() - 3) % 32,
            3,
            view.width() - ((view.width() - 3) % 32),
            (view.height() - 3).min(400),
        )
        .view_bounds_like((32, 32), 32)
        .map(|(x, y, w, h)| view.view(x, y, w, h))
        .chain(
            view.view(
                (view.width() - 3) % 32,
                81,
                view.width() - ((view.width() - 3) % 32),
                (view.height() - 81).min(400),
            )
            .view_bounds_like((32, 32), 32)
            .map(|(x, y, w, h)| view.view(x, y, w, h)),
        )
        .filter(|v| Self::has_edges(v))
    }

    fn match_image<'a>(&self, view: &SubImage<&'a V>) -> Option<Self::MatchResult> {
        let h = self.icon.height();
        let mut fail = 0;
        for y in 0..h {
            let (r, g, b) = self.row_ncc(view, y);
            if r < self.threshold || g < self.threshold || b < self.threshold {
                fail += 1;
                if fail >= h / 3 {
                    return None;
                }
            }
        }

        Some(())
    }

    fn check<'a>(&self, view: &SubImage<&'a V>) -> bool {
        self.match_image(view).is_some()
    }
}
