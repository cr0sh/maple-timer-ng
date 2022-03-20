#![feature(type_alias_impl_trait, generic_associated_types)]

pub mod buff;
pub mod jinhillah;
mod view_ext;

use std::{cell::Cell, fmt::Debug};

use image::{Bgra, GenericImageView, SubImage};
pub use view_ext::*;

pub trait Matcher<V: GenericImageView<Pixel = Bgra<u8>>> {
    type MatchResult: Debug;
    type CandidatesIter<'a>: Iterator<Item = SubImage<&'a V::InnerImageView>>
    where
        V: 'a;

    // Runned once at program initialization. Intended for Once(Cell)-like static resources.
    fn init() {}

    /// `(width, height)` pair of views that the matcher wants.
    /// The caller must ensure that views supplied to [`Matcher::check`] and [`Matcher::match_image`]
    /// have the dimension returned.
    fn view_dimensions(&self) -> (u32, u32);

    /// Iterator of candidate views to be supplied into [`Matcher::check`].
    fn candidates_iter<'a>(&self, view: &'a V) -> Self::CandidatesIter<'a>
    where
        V: GenericImageView<Pixel = Bgra<u8>>;

    /// Returns true if the view is a desired portion of screen that the matcher wants.
    /// The view will be provied to [`Matcher::match_image`], and views of subsequent screenshots
    /// with same bound may be supplied to `match_image` further.
    fn check<'a>(&self, view: &SubImage<&'a V>) -> bool {
        self.match_image(view).is_some()
    }

    /// Main match routine.
    fn match_image<'a>(&self, view: &SubImage<&'a V>) -> Option<Self::MatchResult>;
}

pub struct BoundsCachedMatcher<T>(T, Cell<Option<(u32, u32, u32, u32)>>);

impl<T> BoundsCachedMatcher<T> {
    pub fn new(x: T) -> Self {
        Self(x, Cell::new(None))
    }
}

impl<
        T: Matcher<V>,
        V: GenericImageView<Pixel = Bgra<u8>> + GenericImageView<InnerImageView = V>,
    > Matcher<V> for BoundsCachedMatcher<T>
{
    type MatchResult = T::MatchResult;

    type CandidatesIter<'a> = std::vec::IntoIter<SubImage<&'a V::InnerImageView>>
        where <V as GenericImageView>::InnerImageView: 'a, V: 'a;

    fn view_dimensions(&self) -> (u32, u32) {
        self.0.view_dimensions()
    }

    fn candidates_iter<'a>(&self, view: &'a V) -> Self::CandidatesIter<'a>
    where
        V: GenericImageView<Pixel = Bgra<u8>>,
    {
        if let Some(bounds) = self.1.get() {
            vec![view.inner().view(bounds.0, bounds.1, bounds.2, bounds.3)].into_iter()
        } else {
            self.0.candidates_iter(view).collect::<Vec<_>>().into_iter()
        }
    }

    fn check<'a>(&self, view: &SubImage<&'a V>) -> bool {
        if self.0.check(view) {
            self.1.set(Some(view.bounds()));
            true
        } else {
            false
        }
    }

    fn match_image<'a>(&self, view: &SubImage<&'a V>) -> Option<Self::MatchResult> {
        self.0.match_image(view)
    }
}

#[test]
fn check_sanity() {
    let mut img = image::RgbaImage::new(30, 30);
    img.enumerate_pixels_mut()
        .for_each(|(x, y, p)| *p = image::Rgba([x as u8, y as u8, (x + y) as u8, 255]));
    let view = img.view(2, 2, 10, 10);
    let view = view.view(1, 1, 6, 6);
    let bounds = view.bounds();
    let view2 = view.inner().view(bounds.0, bounds.1, bounds.2, bounds.3);
    assert!(view.eq(&view2));
}
