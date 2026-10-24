//! The display language. Every text the engine shows (card text, the log,
//! button labels, campaign data) exists in English and French; the language
//! is a per-thread setting the UI picks once at startup.
//!
//! Call sites hold both texts side by side ([`tr`], [`T`]), so a text can't
//! lack its translation. Identity strings (card names used by "evolves from",
//! deck rules and saves) stay English whatever the language: see
//! [`CardDef::label`](crate::CardDef::label) for the shown name.

use std::cell::Cell;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Fr,
}

impl Lang {
    /// The BCP 47 code, for `<html lang>`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        }
    }
}

thread_local! {
    static LANG: Cell<Lang> = const { Cell::new(Lang::En) };
}

pub fn lang() -> Lang {
    LANG.with(Cell::get)
}

pub fn set_lang(l: Lang) {
    LANG.with(|c| c.set(l));
}

/// French is on.
pub fn fr() -> bool {
    lang() == Lang::Fr
}

/// The English or the French value, by the current language.
pub fn tr<S>(en: S, fr: S) -> S {
    if self::fr() {
        fr
    } else {
        en
    }
}

/// A static text in both languages, for data tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct T {
    pub en: &'static str,
    pub fr: &'static str,
}

impl T {
    pub const fn new(en: &'static str, fr: &'static str) -> T {
        T { en, fr }
    }

    /// The text in the current language.
    pub fn get(self) -> &'static str {
        tr(self.en, self.fr)
    }
}

impl fmt::Display for T {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_follows_the_language() {
        let hello = T::new("Hello", "Bonjour");
        assert_eq!(hello.get(), "Hello");
        set_lang(Lang::Fr);
        assert_eq!(hello.to_string(), "Bonjour");
        assert_eq!(tr(1, 2), 2);
        set_lang(Lang::En);
        assert_eq!(tr("a", "b"), "a");
    }
}
