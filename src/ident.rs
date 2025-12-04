use string_interner::{DefaultStringInterner, DefaultSymbol};

pub type IdentId = DefaultSymbol;

#[derive(Debug)]
pub struct IdentArena {
    interner: DefaultStringInterner,
}

impl IdentArena {
    pub fn new() -> Self {
        IdentArena {
            interner: DefaultStringInterner::default(),
        }
    }

    pub fn intern(&mut self, name: &str) -> IdentId {
        self.interner.get_or_intern(name)
    }

    pub fn resolve(&self, id: IdentId) -> &str {
        self.interner.resolve(id).unwrap()
    }

    pub fn get(&self, name: &str) -> Option<IdentId> {
        self.interner.get(name)
    }
}

impl Default for IdentArena {
    fn default() -> Self {
        Self::new()
    }
}
