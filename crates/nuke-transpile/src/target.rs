use std::fmt;
use std::str::FromStr;

use nuke_syntax::Value;

use crate::error::Path;
use crate::{ghostty, gitconfig, ini, json, kdl, lua, nix, plist, toml, xml, yaml};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum Target {
    Json,
    Yaml,
    Toml,
    Xml,
    Kdl,
    Lua,
    Ini,
    Gitconfig,
    Nix,
    Ghostty,
    Plist,
}

impl Target {
    pub const ALL: [Self; 11] = [
        Self::Json,
        Self::Yaml,
        Self::Toml,
        Self::Xml,
        Self::Kdl,
        Self::Lua,
        Self::Ini,
        Self::Gitconfig,
        Self::Nix,
        Self::Ghostty,
        Self::Plist,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Xml => "xml",
            Self::Kdl => "kdl",
            Self::Lua => "lua",
            Self::Ini => "ini",
            Self::Gitconfig => "gitconfig",
            Self::Nix => "nix",
            Self::Ghostty => "ghostty",
            Self::Plist => "plist",
        }
    }

    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Json => &["json"],
            Self::Yaml => &["yaml", "yml"],
            Self::Toml => &["toml"],
            Self::Xml => &["xml"],
            Self::Kdl => &["kdl"],
            Self::Lua => &["lua"],
            Self::Ini => &["ini"],
            Self::Gitconfig => &[],
            Self::Nix => &["nix"],
            Self::Ghostty => &[],
            Self::Plist => &["plist"],
        }
    }

    pub fn from_extension(extension: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|target| target.extensions().contains(&extension))
    }

    pub fn render(self, value: &Value) -> Result<String, Refusal> {
        match self {
            Self::Json => json::to_string(value).map_err(Refusal::Json),
            Self::Yaml => yaml::to_string(value).map_err(Refusal::Yaml),
            Self::Toml => toml::to_string(value).map_err(Refusal::Toml),
            Self::Xml => xml::to_string(value).map_err(Refusal::Xml),
            Self::Kdl => kdl::to_string(value).map_err(Refusal::Kdl),
            Self::Lua => lua::to_string(value).map_err(Refusal::Lua),
            Self::Ini => ini::to_string(value).map_err(Refusal::Ini),
            Self::Gitconfig => gitconfig::to_string(value).map_err(Refusal::Gitconfig),
            Self::Nix => nix::to_string(value).map_err(Refusal::Nix),
            Self::Ghostty => ghostty::to_string(value).map_err(Refusal::Ghostty),
            Self::Plist => plist::to_string(value).map_err(Refusal::Plist),
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTarget(String);

impl UnknownTarget {
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnknownTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is no target this writes", self.0)
    }
}

impl std::error::Error for UnknownTarget {}

impl FromStr for Target {
    type Err = UnknownTarget;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|target| target.name() == name)
            .ok_or_else(|| UnknownTarget(name.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Json(json::Error),
    Yaml(yaml::Error),
    Toml(toml::Error),
    Xml(xml::Error),
    Kdl(kdl::Error),
    Lua(lua::Error),
    Ini(ini::Error),
    Gitconfig(gitconfig::Error),
    Nix(nix::Error),
    Ghostty(ghostty::Error),
    Plist(plist::Error),
}

impl Refusal {
    pub const fn target(&self) -> Target {
        match self {
            Self::Json(_) => Target::Json,
            Self::Yaml(_) => Target::Yaml,
            Self::Toml(_) => Target::Toml,
            Self::Xml(_) => Target::Xml,
            Self::Kdl(_) => Target::Kdl,
            Self::Lua(_) => Target::Lua,
            Self::Ini(_) => Target::Ini,
            Self::Gitconfig(_) => Target::Gitconfig,
            Self::Nix(_) => Target::Nix,
            Self::Ghostty(_) => Target::Ghostty,
            Self::Plist(_) => Target::Plist,
        }
    }

    pub const fn path(&self) -> &Path {
        match self {
            Self::Json(error) => error.path(),
            Self::Yaml(error) => error.path(),
            Self::Toml(error) => error.path(),
            Self::Xml(error) => error.path(),
            Self::Kdl(error) => error.path(),
            Self::Lua(error) => error.path(),
            Self::Ini(error) => error.path(),
            Self::Gitconfig(error) => error.path(),
            Self::Nix(error) => error.path(),
            Self::Ghostty(error) => error.path(),
            Self::Plist(error) => error.path(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "{error}"),
            Self::Yaml(error) => write!(f, "{error}"),
            Self::Toml(error) => write!(f, "{error}"),
            Self::Xml(error) => write!(f, "{error}"),
            Self::Kdl(error) => write!(f, "{error}"),
            Self::Lua(error) => write!(f, "{error}"),
            Self::Ini(error) => write!(f, "{error}"),
            Self::Gitconfig(error) => write!(f, "{error}"),
            Self::Nix(error) => write!(f, "{error}"),
            Self::Ghostty(error) => write!(f, "{error}"),
            Self::Plist(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Refusal {}
