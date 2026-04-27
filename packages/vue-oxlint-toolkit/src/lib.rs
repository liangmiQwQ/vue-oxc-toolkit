#![deny(clippy::all)]

use napi_derive::napi;

#[napi]
#[must_use]
pub const fn plus_100(input: u32) -> u32 {
  input + 100
}
