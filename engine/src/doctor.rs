//! The deck doctor: Mags Thale at the Deck Clinic reads a deck and says what
//! ails it (`docs/WORLD_DESIGN.md` §6). Each piece of [`Advice`] has a
//! severity and, where the player's own collection allows, a concrete
//! [`Swap`]. A deck with nothing to say gets [`Issue::Healthy`].

use crate::card::{card, card_id, CardDef, CardId, CardKind, Color};
use crate::collection::Collection;
use crate::deck::MAX_COPIES;
use crate::effect::Effect;
use crate::lang::tr;
use crate::set::{basic_energy, LUCKY_SATCHEL, TRAIL_COMPASS};

/// Fewer Basics than this and some opening hands have none.
pub const MIN_BASICS: usize = 10;
/// The energy count a deck should sit in.
pub const ENERGY: std::ops::RangeInclusive<usize> = 18..=28;
/// This many creature colours is too many to power.
pub const MAX_COLOURS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The deck will play badly.
    Problem,
    /// It could play better.
    Tip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    Healthy,
    /// This many Basic creatures, under [`MIN_BASICS`].
    FewBasics(usize),
    /// This many energy cards, outside [`ENERGY`].
    Energy(usize),
    /// An evolution with nothing in the deck to evolve from.
    BrokenLine(CardId),
    /// Attacks cost this colour and the deck has no energy of it.
    NoEnergyFor(Color),
    /// No draw or search trainer, though the collection holds one.
    NoDrawTrainers,
    /// This many creature colours.
    TooManyColours(usize),
    /// More copies of this evolution than of what it evolves from.
    TopHeavy(CardId),
}

/// Take one `out` out of the deck and put one `into` in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swap {
    pub out: CardId,
    pub into: CardId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Advice {
    pub severity: Severity,
    pub issue: Issue,
    pub swap: Option<Swap>,
}

fn label(id: CardId) -> &'static str {
    card(id).label()
}

/// The shown name of what `id` evolves from.
fn below(id: CardId) -> &'static str {
    card(id)
        .creature()
        .and_then(|c| c.evolves_from)
        .and_then(card_id)
        .map_or("?", label)
}

impl Advice {
    /// What Mags says, in the current language.
    pub fn text(&self) -> String {
        match self.issue {
            Issue::Healthy => tr("Clean bill of health.", "Rien à signaler.").into(),
            Issue::FewBasics(n) => tr(
                format!("Only {n} Basic creatures: some opening hands will have none. Aim for {MIN_BASICS} or more."),
                format!("Seulement {n} créatures de Base : certaines mains de départ n'en auront aucune. Visez-en {MIN_BASICS} ou plus."),
            ),
            Issue::Energy(n) if n < *ENERGY.start() => tr(
                format!("Only {n} energy: your attacks will stall. Aim for 18 to 28."),
                format!("Seulement {n} énergies : vos attaques vont piétiner. Visez entre 18 et 28."),
            ),
            Issue::Energy(n) => tr(
                format!("{n} energy is a lot: you'll draw it instead of creatures. Aim for 18 to 28."),
                format!("{n} énergies, c'est beaucoup : vous les piocherez à la place de vos créatures. Visez entre 18 et 28."),
            ),
            Issue::BrokenLine(id) => tr(
                format!("{} has nothing to evolve from: add {}.", label(id), below(id)),
                format!("{} n'a rien sur quoi évoluer : ajoutez {}.", label(id), below(id)),
            ),
            Issue::NoEnergyFor(c) => tr(
                format!("Some attacks need {c} energy, and the deck has none."),
                format!("Certaines attaques demandent de l'énergie {c}, et le deck n'en a aucune."),
            ),
            Issue::NoDrawTrainers => tr(
                format!("No draw or search trainers: a {} or a {} would keep your hand full.", label(LUCKY_SATCHEL), label(TRAIL_COMPASS)),
                format!("Aucune carte Dresseur de pioche ou de recherche : {} ou {} garderait votre main pleine.", label(LUCKY_SATCHEL), label(TRAIL_COMPASS)),
            ),
            Issue::TooManyColours(n) => tr(
                format!("{n} colours is a lot to power. Two is easier."),
                format!("{n} couleurs, c'est beaucoup à alimenter. Deux, c'est plus simple."),
            ),
            Issue::TopHeavy(id) => tr(
                format!("More {} than {} underneath: some will be stuck in your hand.", label(id), below(id)),
                format!("Plus de {} que de {} en dessous : certains resteront coincés dans votre main.", label(id), below(id)),
            ),
        }
    }
}

impl Swap {
    pub fn text(&self) -> String {
        tr(
            format!("Swap 1 {} for 1 {}.", label(self.out), label(self.into)),
            format!(
                "Remplacez 1 {} par 1 {}.",
                label(self.out),
                label(self.into)
            ),
        )
    }
}

/// A deck and the collection it is built from, counted.
struct Chart<'a> {
    deck: &'a [CardId],
    owned: &'a Collection,
}

impl Chart<'_> {
    fn n(&self, id: CardId) -> usize {
        self.deck.iter().filter(|&&c| c == id).count()
    }

    /// One more copy may go in: owned beyond the deck's, under the limit.
    fn can_add(&self, id: CardId) -> bool {
        let d = card(id);
        let n = self.n(id);
        self.owned.count(id) > n && (d.is_basic_energy() || n < MAX_COPIES)
    }

    fn cards(&self) -> impl Iterator<Item = &'static CardDef> + '_ {
        self.deck.iter().map(|&id| card(id))
    }

    fn colours(&self) -> Vec<Color> {
        let mut c: Vec<Color> = self
            .cards()
            .filter_map(CardDef::creature)
            .map(|cr| cr.color)
            .filter(|&c| c != Color::Colorless)
            .collect();
        c.sort_by_key(|&c| c as u8);
        c.dedup();
        c
    }

    /// The most-copied card in the deck that passes `keep`.
    fn most(&self, keep: impl Fn(&CardDef) -> bool) -> Option<CardId> {
        let mut ids: Vec<CardId> = self.deck.to_vec();
        ids.sort();
        ids.dedup();
        ids.into_iter()
            .filter(|&id| keep(card(id)))
            .max_by_key(|&id| (self.n(id), std::cmp::Reverse(id)))
    }

    /// What the deck can best spare: a basic energy while it has plenty, else
    /// its most-copied trainer.
    fn spare(&self) -> Option<CardId> {
        let energy = self.cards().filter(|c| c.is_basic_energy()).count();
        if energy > *ENERGY.start() + 2 {
            return self.most(CardDef::is_basic_energy);
        }
        self.most(|c| matches!(c.kind, CardKind::Trainer(_)))
    }

    /// The sturdiest Basic of the deck's colours (or colorless) that can go in.
    fn basic_to_add(&self) -> Option<CardId> {
        let colours = self.colours();
        crate::set::ids()
            .filter(|&id| {
                card(id).creature().is_some_and(|c| {
                    card(id).is_basic()
                        && (c.color == Color::Colorless || colours.contains(&c.color))
                })
            })
            .filter(|&id| self.can_add(id) && !crate::set::is_promo(id))
            .max_by_key(|&id| card(id).creature().map_or(0, |c| c.hp))
    }
}

fn swap(out: Option<CardId>, into: Option<CardId>) -> Option<Swap> {
    Some(Swap {
        out: out?,
        into: into?,
    })
}

/// Read a 60-card deck built from `owned`: most serious first.
pub fn diagnose(deck: &[CardId], owned: &Collection) -> Vec<Advice> {
    let x = Chart { deck, owned };
    let mut out = vec![];
    let mut say = |severity, issue, swap| {
        out.push(Advice {
            severity,
            issue,
            swap,
        })
    };

    let basics = x.cards().filter(|c| c.is_basic()).count();
    if basics < MIN_BASICS {
        say(
            Severity::Problem,
            Issue::FewBasics(basics),
            swap(x.spare(), x.basic_to_add()),
        );
    }

    let energy = x
        .cards()
        .filter(|c| matches!(c.kind, CardKind::Energy(_)))
        .count();
    if !ENERGY.contains(&energy) {
        let s = if energy < *ENERGY.start() {
            // The colour the most attack symbols ask for.
            let wanted = x
                .colours()
                .into_iter()
                .max_by_key(|&col| {
                    x.cards()
                        .filter_map(CardDef::creature)
                        .flat_map(|c| c.attacks.iter().flat_map(|a| a.cost))
                        .filter(|&&c| c == col)
                        .count()
                })
                .and_then(basic_energy);
            swap(x.most(|c| matches!(c.kind, CardKind::Trainer(_))), wanted)
        } else {
            let into = x.basic_to_add().or_else(|| {
                [LUCKY_SATCHEL, TRAIL_COMPASS]
                    .into_iter()
                    .find(|&t| x.can_add(t))
            });
            swap(x.most(CardDef::is_basic_energy), into)
        };
        say(Severity::Problem, Issue::Energy(energy), s);
    }

    let mut evolutions: Vec<CardId> = deck
        .iter()
        .copied()
        .filter(|&id| {
            card(id)
                .creature()
                .is_some_and(|c| c.evolves_from.is_some())
        })
        .collect();
    evolutions.sort();
    evolutions.dedup();
    for &evo in &evolutions {
        let from = card(evo)
            .creature()
            .and_then(|c| c.evolves_from)
            .and_then(card_id);
        let under = from.map_or(0, |f| x.n(f));
        let into = from.filter(|&f| x.can_add(f));
        if under == 0 {
            say(
                Severity::Problem,
                Issue::BrokenLine(evo),
                swap(Some(evo), into),
            );
        } else if x.n(evo) > under {
            say(Severity::Tip, Issue::TopHeavy(evo), swap(Some(evo), into));
        }
    }

    let mut costs: Vec<Color> = x
        .cards()
        .filter_map(CardDef::creature)
        .flat_map(|c| c.attacks.iter().flat_map(|a| a.cost.iter().copied()))
        .filter(|&c| c != Color::Colorless)
        .collect();
    costs.sort_by_key(|&c| c as u8);
    costs.dedup();
    for c in costs {
        let Some(e) = basic_energy(c) else { continue };
        if x.n(e) == 0 {
            say(
                Severity::Problem,
                Issue::NoEnergyFor(c),
                swap(x.most(CardDef::is_basic_energy), Some(e)),
            );
        }
    }

    let draws = x.cards().any(|c| {
        matches!(&c.kind, CardKind::Trainer(t) if matches!(t.effects.first(), Some(Effect::Draw(_) | Effect::Search(_))))
    });
    let helper = [LUCKY_SATCHEL, TRAIL_COMPASS]
        .into_iter()
        .find(|&t| x.can_add(t));
    if !draws && helper.is_some() {
        say(
            Severity::Tip,
            Issue::NoDrawTrainers,
            swap(x.spare(), helper),
        );
    }

    let colours = x.colours().len();
    if colours > MAX_COLOURS {
        say(Severity::Tip, Issue::TooManyColours(colours), None);
    }

    out.sort_by_key(|a| a.severity);
    if out.is_empty() {
        out.push(Advice {
            severity: Severity::Tip,
            issue: Issue::Healthy,
            swap: None,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decks::{PREBUILT, STARTER};
    use crate::set::*;

    fn issues(deck: &[CardId], owned: &Collection) -> Vec<Issue> {
        diagnose(deck, owned).into_iter().map(|a| a.issue).collect()
    }

    fn all() -> Collection {
        let mut c = Collection::new();
        crate::set::ids().for_each(|id| c.add(id, 8));
        c
    }

    fn with(deck: &mut [CardId], out: CardId, into: CardId, n: usize) {
        for _ in 0..n {
            let i = deck.iter().position(|&c| c == out).expect("in the deck");
            deck[i] = into;
        }
    }

    /// The shipped decks are healthy, or close.
    #[test]
    fn the_prebuilt_decks_get_a_clean_bill() {
        assert_eq!(issues(&STARTER.cards(), &all()), [Issue::Healthy]);
        for d in PREBUILT {
            let problems: Vec<Advice> = diagnose(&d.cards(), &all())
                .into_iter()
                .filter(|a| a.severity == Severity::Problem)
                .collect();
            assert!(problems.is_empty(), "{}: {problems:?}", d.name);
        }
    }

    #[test]
    fn few_basics_and_energy_counts() {
        // Starter: Basics swapped for energy.
        let mut d = STARTER.cards();
        let basics: Vec<CardId> = d
            .iter()
            .copied()
            .filter(|&id| card(id).is_basic())
            .collect();
        for &b in &basics[..basics.len() - 8] {
            with(&mut d, b, FIRE_ENERGY, 1);
        }
        let a = diagnose(&d, &all());
        assert_eq!(a[0].issue, Issue::FewBasics(8));
        let s = a[0].swap.unwrap();
        assert!(card(s.out).is_basic_energy() && card(s.into).is_basic());
        assert!(a
            .iter()
            .any(|a| matches!(a.issue, Issue::Energy(n) if n > 28)));

        let mut low = STARTER.cards();
        let energy = low.iter().filter(|&&c| card(c).is_basic_energy()).count();
        let take = energy - 15;
        with(&mut low, FIRE_ENERGY, LUCKY_SATCHEL, 2);
        with(&mut low, WATER_ENERGY, TRAIL_COMPASS, 2);
        with(&mut low, FIRE_ENERGY, MOSS_POULTICE, 2);
        with(&mut low, WATER_ENERGY, RELAY_BATON, take - 6);
        let a = diagnose(&low, &all());
        let e = a
            .iter()
            .find(|a| matches!(a.issue, Issue::Energy(15)))
            .expect("too little energy");
        assert!(card(e.swap.unwrap().into).is_basic_energy());
        assert_eq!(e.severity, Severity::Problem);
    }

    #[test]
    fn broken_and_top_heavy_lines_name_the_card_below() {
        let mut d = STARTER.cards();
        // Every Flarrow's Cindrel out: a broken line.
        let cindrel = d.iter().filter(|&&c| c == CINDREL).count();
        with(&mut d, CINDREL, FIRE_ENERGY, cindrel);
        let a = diagnose(&d, &all());
        let b = a
            .iter()
            .find(|a| a.issue == Issue::BrokenLine(FLARROW))
            .unwrap();
        assert_eq!(
            b.swap,
            Some(Swap {
                out: FLARROW,
                into: CINDREL
            })
        );
        assert_eq!(b.text(), "Flarrow has nothing to evolve from: add Cindrel.");
        // With one Cindrel back, two Flarrow sit on one: top-heavy.
        with(&mut d, FIRE_ENERGY, CINDREL, 1);
        assert!(issues(&d, &all()).contains(&Issue::TopHeavy(FLARROW)));
        // Nothing to add from a collection without spare Cindrel.
        let mut poor = all();
        poor.remove(CINDREL, 7);
        let t = diagnose(&d, &poor)
            .into_iter()
            .find(|a| a.issue == Issue::TopHeavy(FLARROW))
            .unwrap();
        assert_eq!(t.swap, None);
    }

    #[test]
    fn missing_energy_colours_draw_and_too_many_colours() {
        let mut d = STARTER.cards();
        // No Water energy for the Water creatures.
        let water = d.iter().filter(|&&c| c == WATER_ENERGY).count();
        with(&mut d, WATER_ENERGY, FIRE_ENERGY, water);
        assert!(issues(&d, &all()).contains(&Issue::NoEnergyFor(Color::Water)));
        // No draw or search trainers, while the collection has some.
        let mut nd = STARTER.cards();
        for t in [LUCKY_SATCHEL, TRAIL_COMPASS, DOWSING_ROD] {
            let n = nd.iter().filter(|&&c| c == t).count();
            with(&mut nd, t, FIRE_ENERGY, n);
        }
        let a = diagnose(&nd, &all());
        let tip = a.iter().find(|a| a.issue == Issue::NoDrawTrainers).unwrap();
        assert_eq!(tip.severity, Severity::Tip);
        assert!(matches!(tip.swap, Some(Swap { into, .. }) if into == LUCKY_SATCHEL));
        assert!(!issues(&nd, &Collection::new()).contains(&Issue::NoDrawTrainers));
        // Four creature colours.
        let mut four = STARTER.cards();
        with(&mut four, FIRE_ENERGY, SPARKIT, 2);
        with(&mut four, WATER_ENERGY, SPRIGGET, 2);
        let a = issues(&four, &all());
        assert!(a.contains(&Issue::TooManyColours(4)), "{a:?}");
        // Problems first.
        let sev: Vec<Severity> = diagnose(&four, &all()).iter().map(|a| a.severity).collect();
        assert!(sev.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn advice_reads_in_french() {
        use crate::lang::{set_lang, Lang};
        set_lang(Lang::Fr);
        let a = Advice {
            severity: Severity::Tip,
            issue: Issue::Healthy,
            swap: None,
        };
        let s = Swap {
            out: FIRE_ENERGY,
            into: CINDREL,
        };
        let (t, st) = (a.text(), s.text());
        let few = Advice {
            issue: Issue::FewBasics(7),
            ..a
        }
        .text();
        set_lang(Lang::En);
        assert_eq!(t, "Rien à signaler.");
        assert!(st.starts_with("Remplacez 1 "), "{st}");
        assert!(few.starts_with("Seulement 7 créatures de Base"));
    }
}
