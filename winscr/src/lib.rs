#[allow(dead_code)]
mod d3d;
#[allow(dead_code)]
#[doc(hidden)]
pub mod d3d_capture;
pub mod gdi_capture;
mod window_item;

#[cfg(not(target_os = "windows"))]
compile_error!("This crate only supports Windows.");
