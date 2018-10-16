#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

mod inibin;

pub use self::inibin::{inibin_hash, inibin_incremental_hash, IniBin, Value};

#[cfg(feature = "serde")]
mod de;
#[cfg(feature = "serde")]
mod error;
#[cfg(feature = "serde")]
pub use self::de::*;
#[cfg(feature = "serde")]
pub use self::error::*;
