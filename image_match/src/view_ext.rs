use image::{Bgra, GenericImageView, ImageBuffer, Pixel, Rgba};

pub trait GenericImageViewExt: GenericImageView {
    type ViewsLike: Iterator<Item = (u32, u32, u32, u32)>;
    fn view_bounds_like(
        &self,
        dim: (u32, u32), // width, height
        stride: usize,
    ) -> Self::ViewsLike;

    fn eq<V: GenericImageView>(&self, other: &V) -> bool
    where
        V::Pixel: PartialEq<Self::Pixel>;

    /// Similar to [`GenericImageViewExt::eq`], but filters out pixels with these conditions:
    /// `alpha != 255 || r == g == b == 0 || r == g == b == 255`.
    ///
    /// Note that these conditions might change in future. They are for reference only.
    fn regularized_eq<V: GenericImageView>(&self, other: &V) -> bool
    where
        Self::Pixel: RegularizedEqPixel,
        V::Pixel: PartialEq<Self::Pixel> + RegularizedEqPixel;

    /// Similar to [`GenericImageViewExt::eq`], but dimensions of the two images are not required to match.
    /// Specifically, dimension of `other` must be equal or larger than one of `self`.
    fn part_eq<V: GenericImageView>(&self, other: &V) -> bool
    where
        V::Pixel: PartialEq<Self::Pixel>;

    fn regularized_part_eq<V: GenericImageView>(&self, other: &V) -> bool
    where
        Self::Pixel: RegularizedEqPixel,
        V::Pixel: PartialEq<Self::Pixel> + RegularizedEqPixel;

    fn to_image_buffer(
        &self,
    ) -> ImageBuffer<Self::Pixel, Vec<<<Self as GenericImageView>::Pixel as Pixel>::Subpixel>>
    where
        Self::Pixel: 'static;
}

impl<V: GenericImageView> GenericImageViewExt for V {
    type ViewsLike = impl Iterator<Item = (u32, u32, u32, u32)>;

    fn view_bounds_like<'a>(
        &self,
        dim: (u32, u32), // width, height
        stride: usize,
    ) -> Self::ViewsLike {
        let my_dim = self.dimensions();
        let (x_, y_, w, h) = self.bounds();

        assert!(my_dim.0 >= dim.0);
        assert!(my_dim.1 >= dim.1);

        (x_..=(x_ + w - dim.0)).step_by(stride).flat_map(move |x| {
            (y_..=(y_ + h - dim.1))
                .step_by(stride)
                .map(move |y| (x, y, dim.0, dim.1)) // x, y, w, h
        })
    }

    fn eq<W: GenericImageView>(&self, other: &W) -> bool
    where
        W::Pixel: PartialEq<V::Pixel>,
    {
        debug_assert_eq!(self.dimensions(), other.dimensions());

        self.pixels().zip(other.pixels()).all(|(x, y)| y.2 == x.2)
    }

    fn regularized_eq<W: GenericImageView>(&self, other: &W) -> bool
    where
        V::Pixel: RegularizedEqPixel,
        W::Pixel: PartialEq<V::Pixel> + RegularizedEqPixel,
    {
        debug_assert_eq!(self.dimensions(), other.dimensions());

        self.pixels()
            .zip(other.pixels())
            .all(|(x, y)| y.2 == x.2 && x.2.good_pixel() && y.2.good_pixel())
    }

    fn part_eq<W: GenericImageView>(&self, other: &W) -> bool
    where
        W::Pixel: PartialEq<V::Pixel>,
    {
        debug_assert!(self.width() < other.width());
        debug_assert!(self.height() < other.height());

        self.pixels().zip(other.pixels()).all(|(x, y)| y.2 == x.2)
    }

    fn regularized_part_eq<W: GenericImageView>(&self, other: &W) -> bool
    where
        V::Pixel: RegularizedEqPixel,
        W::Pixel: PartialEq<V::Pixel> + RegularizedEqPixel,
    {
        debug_assert!(self.width() < other.width());
        debug_assert!(self.height() < other.height());

        self.pixels()
            .zip(other.pixels())
            .all(|(x, y)| y.2 == x.2 && x.2.good_pixel() && y.2.good_pixel())
    }

    fn to_image_buffer(
        &self,
    ) -> ImageBuffer<Self::Pixel, Vec<<<V as GenericImageView>::Pixel as Pixel>::Subpixel>>
    where
        Self::Pixel: 'static,
    {
        ImageBuffer::from_fn(self.width(), self.height(), |x, y| self.get_pixel(x, y))
    }
}

pub trait RegularizedEqPixel: Pixel {
    fn good_pixel(&self) -> bool;
}

impl RegularizedEqPixel for Bgra<u8> {
    fn good_pixel(&self) -> bool {
        self.0[3] == 255
            && !(self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0)
            && !(self.0[0] == 255 && self.0[1] == 255 && self.0[2] == 255)
    }
}

impl RegularizedEqPixel for Rgba<u8> {
    fn good_pixel(&self) -> bool {
        self.0[3] == 255
            && !(self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0)
            && !(self.0[0] == 255 && self.0[1] == 255 && self.0[2] == 255)
    }
}
