use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Concat,
    Import,
}

impl Builtin {
    pub const ALL: [Self; 2] = [Self::Concat, Self::Import];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Concat => "concat",
            Self::Import => "import",
        }
    }

    pub const fn summary(self) -> &'static str {
        match self {
            Self::Concat => "takes a list of strings and gives them joined into one",
            Self::Import => "takes a string literal and gives the document it names",
        }
    }

    pub fn of(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|builtin| builtin.name() == name)
    }
}

impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name())
    }
}
