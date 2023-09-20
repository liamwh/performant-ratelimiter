pub mod version0;
pub use version0::*;

pub mod version1;
pub use version1::*;

pub mod version2;
pub use version2::*;

pub mod version3;
pub use version3::*;

pub const MAX_REQUESTS: usize = 100;
pub const MAX_REQUESTS_DURATION_SECONDS: i64 = 60;
