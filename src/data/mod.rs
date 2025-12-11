pub mod location;
pub mod shared_booking;

#[cfg(not(target_arch = "wasm32"))]
pub mod booking;
#[cfg(not(target_arch = "wasm32"))]
pub mod rta;
