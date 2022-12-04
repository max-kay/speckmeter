#![feature(io_error_other)]
#![feature(is_sorted)]
#![feature(fn_traits)]
#![feature(const_fn_floating_point_arithmetic)]

mod app;
mod calib;
mod cam;
mod csv;
mod log;
mod lin_reg;
mod line_search;
mod line_tracer;
mod meter;
pub use app::SpeckApp;

pub const SMALLEST_WAVELENGTH: u16 = 380;
pub const LARGEST_WAVELENGTH: u16 = 750;
