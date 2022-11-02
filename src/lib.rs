#![feature(io_error_other)]
#![feature(is_sorted)]
#![feature(fn_traits)]
#![feature(const_fn_floating_point_arithmetic)]

mod app;
mod calib;
mod cam;
mod lin_reg;
mod meter;
pub use app::SpeckApp;

pub const SMALLEST_WAVE_LENGTH: u16 = 380;
pub const LARGEST_WAVE_LENGTH: u16 = 750;
