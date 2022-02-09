use std::hint::unreachable_unchecked;
use std::sync::mpsc::{channel, Receiver};

use image::Bgra;
use thiserror::Error;
use windows::core::{IInspectable, Interface};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use crate::d3d;
use crate::window_item::enumerate_capturable_windows;

#[derive(Debug, Error)]
pub enum D3dCaptureError {
    #[error("Cannot find specified window")]
    NoSuchWindow,
    #[error("No captured image on buffer")]
    NoCapturedImage,
    #[error("WinRT API error. code: {}, message: {}", .0.code().0, .0.message())]
    WinRt(windows::core::Error),
}

impl From<windows::core::Error> for D3dCaptureError {
    fn from(x: windows::core::Error) -> Self {
        Self::WinRt(x)
    }
}

pub enum D3dCaptureState {
    Initial,
    Captured {
        buffer: Vec<u8>,
        width: u32,
        height: u32,
    },
}

pub struct D3dCapturer {
    d3d_context: ID3D11DeviceContext,
    session: GraphicsCaptureSession,
    frame_pool: Direct3D11CaptureFramePool,
    receiver: Receiver<ID3D11Texture2D>,
    state: D3dCaptureState,
}

impl D3dCapturer {
    pub fn new(class_name: &str, window_name: &str) -> Result<D3dCapturer, D3dCaptureError> {
        Self::new_nth(class_name, window_name, 0)
    }

    pub fn new_nth(
        class_name: &str,
        window_name: &str,
        n: usize,
    ) -> Result<D3dCapturer, D3dCaptureError> {
        let wins = enumerate_capturable_windows();
        let item = wins
            .iter()
            .filter(|item| item.matches_title_and_class_name(window_name, class_name))
            .nth(n)
            .ok_or(D3dCaptureError::NoSuchWindow)?;

        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;

        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(item.handle)? };

        let item_size = item.Size()?;

        let d3d_device = d3d::create_d3d_device()?;
        let d3d_context = unsafe {
            let mut d3d_context = None;
            d3d_device.GetImmediateContext(&mut d3d_context);
            d3d_context.unwrap()
        };
        let device = d3d::create_direct3d_device(&d3d_device)?;
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            1,
            &item_size,
        )?;
        let session = frame_pool.CreateCaptureSession(item)?;

        let (sender, receiver) = channel();
        frame_pool.FrameArrived(
            TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new({
                let d3d_context = d3d_context.clone();
                move |frame_pool, _| unsafe {
                    let frame_pool = frame_pool.as_ref().unwrap();
                    let frame = frame_pool.TryGetNextFrame()?;
                    let source_texture: ID3D11Texture2D =
                        d3d::get_d3d_interface_from_object(&frame.Surface()?)?;
                    let mut desc = D3D11_TEXTURE2D_DESC::default();
                    source_texture.GetDesc(&mut desc);
                    desc.BindFlags = 0;
                    desc.MiscFlags = 0;
                    desc.Usage = D3D11_USAGE_STAGING;
                    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
                    let copy_texture = { d3d_device.CreateTexture2D(&desc, std::ptr::null())? };

                    d3d_context
                        .CopyResource(Some(copy_texture.cast()?), Some(source_texture.cast()?));

                    sender.send(copy_texture).unwrap();
                    Ok(())
                }
            }),
        )?;
        session.StartCapture()?;

        Ok(Self {
            session,
            d3d_context,
            frame_pool,
            receiver,
            state: D3dCaptureState::Initial,
        })
    }

    pub fn capture(&mut self) -> Result<(), D3dCaptureError> {
        let texture = self.receiver.recv().unwrap();
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc as *mut _) };

        let resource: ID3D11Resource = texture.cast()?;
        let mapped = unsafe {
            self.d3d_context
                .Map(Some(resource.clone()), 0, D3D11_MAP_READ, 0)?
        };

        let bytes_per_pixel = 4;
        let new_buffer_size = (desc.Width * desc.Height * bytes_per_pixel) as usize;

        let buffer = match self.state {
            D3dCaptureState::Initial => {
                self.state = D3dCaptureState::Captured {
                    buffer: Vec::with_capacity(new_buffer_size),
                    width: desc.Width,
                    height: desc.Height,
                };
                if let D3dCaptureState::Captured { ref mut buffer, .. } = self.state {
                    buffer
                } else {
                    unsafe { unreachable_unchecked() };
                }
            }
            D3dCaptureState::Captured {
                ref mut buffer,
                ref mut width,
                ref mut height,
            } => {
                buffer.reserve(new_buffer_size);
                *width = desc.Width;
                *height = desc.Height;

                buffer
            }
        };

        for row in 0..desc.Height {
            unsafe {
                buffer
                    .as_mut_ptr()
                    .offset((row * (desc.Width * bytes_per_pixel)) as isize)
                    .copy_from_nonoverlapping(
                        (mapped.pData as *const u8).offset((row * mapped.RowPitch) as isize),
                        (desc.Width * bytes_per_pixel) as usize,
                    );
            }
        }

        unsafe { buffer.set_len(new_buffer_size) };

        unsafe { self.d3d_context.Unmap(Some(resource), 0) };

        Ok(())
    }

    pub fn get_image_buffer(
        &mut self,
    ) -> Result<image::ImageBuffer<Bgra<u8>, &mut [u8]>, D3dCaptureError> {
        let (buffer, width, height) = if let D3dCaptureState::Captured {
            buffer,
            width,
            height,
        } = &mut self.state
        {
            (buffer.as_mut_slice(), *width, *height)
        } else {
            return Err(D3dCaptureError::NoCapturedImage);
        };

        Ok(image::ImageBuffer::from_raw(width, height, buffer).unwrap())
    }
}

impl Drop for D3dCapturer {
    fn drop(&mut self) {
        self.session.Close().unwrap();
        self.frame_pool.Close().unwrap();
    }
}
