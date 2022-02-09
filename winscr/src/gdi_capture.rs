use std::marker::PhantomData;

use image::Bgra;
use thiserror::Error;
use windows::Win32::{
    Foundation::{GetLastError, HWND, RECT},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatedHDC, DeleteDC, DeleteObject,
        GetBitmapBits, GetDC, ReleaseDC, SelectObject, HBITMAP, HDC, SRCCOPY,
    },
    UI::WindowsAndMessaging::GetClientRect,
};

use crate::window_item;

type Result<T, E = GdiCaptureError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum GdiCaptureError {
    #[error("No such window")]
    NoSuchWindow,
    #[error("Windows GDI failure: code={0:?}")]
    Gdi(Option<u32>),
}

macro_rules! ensure_gdi_success {
    ($e:expr) => {{
        let result = $e;
        if result.0 == 0 {
            return Err(GdiCaptureError::Gdi(None));
        }
        result
    }};
}

pub struct GdiCapturer {
    hwnd: HWND,
    hdc: HDC,
    compatible_hdc: CreatedHDC,
    handle_bitmap: HBITMAP,
    width: i32,
    height: i32,
    buffer: Vec<u8>,
    _non_send: PhantomData<*mut ()>,
}

impl GdiCapturer {
    pub fn new(title: &str, class_name: &str, hidpi: bool) -> Result<GdiCapturer> {
        Self::new_nth(title, class_name, 0, hidpi)
    }
    pub fn new_nth(title: &str, class_name: &str, n: usize, hidpi: bool) -> Result<GdiCapturer> {
        unsafe {
            let wins = window_item::enumerate_capturable_windows();
            let hwnd = wins
                .iter()
                .filter(|item| item.matches_title_and_class_name(title, class_name))
                .nth(n)
                .ok_or(GdiCaptureError::NoSuchWindow)?
                .handle;

            let hdc = ensure_gdi_success!(GetDC(hwnd));
            let mut rect = RECT::default();
            if GetClientRect(hwnd, &mut rect).0 == 0 {
                return Err(GdiCaptureError::Gdi(Some(GetLastError())));
            }

            let width = (rect.right - rect.left).abs();
            let height = (rect.bottom - rect.top).abs();

            let width = if hidpi { width * 2 / 3 } else { width };

            let height = if hidpi { height * 2 / 3 } else { height };

            let compatible_hdc = ensure_gdi_success!(CreateCompatibleDC(hdc));
            let handle_bitmap = ensure_gdi_success!(CreateCompatibleBitmap(hdc, width, height));
            ensure_gdi_success!(SelectObject(compatible_hdc, handle_bitmap));
            Ok(GdiCapturer {
                hwnd,
                hdc,
                compatible_hdc,
                handle_bitmap,
                width,
                height,
                buffer: Vec::with_capacity((width * height * 4) as usize),
                _non_send: PhantomData,
            })
        }
    }

    pub fn capture(&mut self) -> Result<()> {
        unsafe {
            if BitBlt(
                self.compatible_hdc,
                0,
                0,
                self.width,
                self.height,
                self.hdc,
                0,
                0,
                SRCCOPY,
            )
            .0 == 0
            {
                return Err(GdiCaptureError::Gdi(Some(GetLastError())));
            }

            let len = self.width * self.height * 4;
            if GetBitmapBits(self.handle_bitmap, len, self.buffer.as_mut_ptr() as *mut _) == 0 {
                return Err(GdiCaptureError::Gdi(None));
            }

            self.buffer.set_len(len as usize);
        }
        Ok(())
    }

    pub fn get_image_buffer(&self) -> Option<image::ImageBuffer<Bgra<u8>, &[u8]>> {
        image::ImageBuffer::<Bgra<u8>, _>::from_raw(
            self.width as u32,
            self.height as u32,
            self.buffer.as_slice(),
        )
    }

    pub fn dimension(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }
}

impl Drop for GdiCapturer {
    fn drop(&mut self) {
        unsafe {
            ReleaseDC(self.hwnd, self.hdc);
            DeleteObject(self.handle_bitmap);
            DeleteDC(self.compatible_hdc);
        }
    }
}
