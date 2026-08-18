pub mod error;
pub mod gitconfig;
pub mod ini;
pub mod json;
pub mod kdl;
pub mod lua;
pub mod nix;
pub mod target;
pub mod toml;
pub mod xml;
pub mod yaml;

pub use error::{Error, Path, Segment};
pub use target::{Refusal, Target, UnknownTarget};
