pub mod error;
pub mod ghostty;
pub mod gitconfig;
pub mod ini;
pub mod json;
pub mod kdl;
pub mod lua;
pub(crate) mod name;
pub mod nix;
pub mod plist;
pub(crate) mod table;
pub mod target;
pub mod toml;
pub mod xml;
pub(crate) mod xml_text;
pub mod yaml;

pub use error::{Error, Path, Segment};
pub use target::{Refusal, Target, UnknownTarget};
