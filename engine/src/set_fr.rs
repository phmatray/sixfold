//! French text for the card set: one row per card of [`crate::set::CARDS`],
//! in the same order. Each row repeats the English name so the tests below
//! catch a row that drifted. The English `name` stays the identity key; these
//! are only what the UI shows (see [`crate::CardDef::label`]).

/// One card's French text.
#[derive(Debug)]
pub struct CardFr {
    /// The card's English name (alignment check only).
    #[cfg_attr(not(test), allow(dead_code))]
    pub en: &'static str,
    pub name: &'static str,
    /// (name, rules text) per attack, in the card's order. An attack with no
    /// English text has none here either.
    pub attacks: &'static [(&'static str, &'static str)],
    /// The Power's (name, rules text).
    pub power: Option<(&'static str, &'static str)>,
    /// A trainer's rules text (empty for creatures and energy).
    pub text: &'static str,
}

/// A creature (or Trainer-as-Basic).
const fn c(
    en: &'static str,
    name: &'static str,
    attacks: &'static [(&'static str, &'static str)],
    power: Option<(&'static str, &'static str)>,
) -> CardFr {
    CardFr {
        en,
        name,
        attacks,
        power,
        text: "",
    }
}

/// A trainer.
const fn t(en: &'static str, name: &'static str, text: &'static str) -> CardFr {
    CardFr {
        en,
        name,
        attacks: &[],
        power: None,
        text,
    }
}

/// An energy card.
const fn e(en: &'static str, name: &'static str) -> CardFr {
    CardFr {
        en,
        name,
        attacks: &[],
        power: None,
        text: "",
    }
}

#[rustfmt::skip]
pub static FR: &[CardFr] = &[
    c("Cindrel", "Braisillon", &[
        ("Pichenette ardente", "Lancez une pièce. Si c'est face, cette attaque inflige 10 dégâts de plus."),
        ("Ruée de cendres", ""),
    ], None),
];

#[cfg(test)]
mod tests {
    use super::FR;
    use crate::card::CardKind;
    use crate::set::CARDS;

    /// One French row per card, in order, with the same attacks, Power and
    /// trainer text, nothing left empty that English fills, and unique names.
    #[test]
    fn french_rows_match_the_set() {
        assert_eq!(FR.len(), CARDS.len(), "one French row per card");
        for (i, (c, f)) in CARDS.iter().zip(FR).enumerate() {
            assert_eq!(f.en, c.name, "row {i} is out of order");
            assert!(!f.name.trim().is_empty(), "{}: no French name", c.name);
            assert!(
                FR[..i].iter().all(|g| g.name != f.name),
                "{}: French name {} used twice",
                c.name,
                f.name
            );
            let (attacks, power, text) = match &c.kind {
                CardKind::Creature(cr) | CardKind::TrainerBasic(cr) => {
                    (cr.attacks, cr.power.as_ref(), "")
                }
                CardKind::Trainer(t) => (&[][..], None, t.text),
                CardKind::Energy(_) => (&[][..], None, ""),
            };
            assert_eq!(f.attacks.len(), attacks.len(), "{}: attack count", c.name);
            for (a, (name, rules)) in attacks.iter().zip(f.attacks) {
                assert!(
                    !name.is_empty(),
                    "{}: {} has no French name",
                    c.name,
                    a.name
                );
                assert_eq!(
                    a.text.is_empty(),
                    rules.is_empty(),
                    "{}: {} text",
                    c.name,
                    a.name
                );
            }
            assert_eq!(power.is_some(), f.power.is_some(), "{}: Power", c.name);
            if let (Some(p), Some((name, rules))) = (power, f.power) {
                assert!(
                    !name.is_empty() && !rules.is_empty(),
                    "{}: {}",
                    c.name,
                    p.name
                );
            }
            assert_eq!(
                text.is_empty(),
                f.text.is_empty(),
                "{}: trainer text",
                c.name
            );
        }
    }

    /// The accessors switch with the language; the identity name doesn't.
    #[test]
    fn labels_follow_the_language() {
        use crate::lang::{set_lang, Lang};
        let c = &CARDS[0];
        let a = &c.creature().unwrap().attacks[0];
        set_lang(Lang::Fr);
        let fr = (c.label(), a.label(), a.rules(), c.name);
        set_lang(Lang::En);
        assert_eq!(fr.0, FR[0].name);
        assert_eq!(fr.1, FR[0].attacks[0].0);
        assert_eq!(fr.2, FR[0].attacks[0].1);
        assert_eq!(fr.3, "Cindrel");
        assert_eq!(c.label(), "Cindrel");
        assert_eq!(a.label(), a.name);
    }
}
