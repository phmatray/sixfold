//! Play stats (`docs/WORLD_DESIGN.md` §12.2): named counters, fed by finished
//! duels ([`Stats::record_duel`], from [`Game::summary`](sixfold_engine::Game::summary))
//! and by the world ([`Stats::observe`] diffs the progress and the profile;
//! steps and seal-site visits are reported as they happen). Saved as the
//! `[stats]` section of the save: `s1;<key>=<n>,...`, sorted, zeros left out.
//! A key this build doesn't know is kept as it is.
//!
//! Achievements (§12.3): [`ACHIEVEMENTS`], each a pure condition on the stats,
//! the progress and the profile. [`award`] grants the newly earned ones once:
//! the flag `ach_<id>` and their Facets.

use std::collections::BTreeMap;

use crate::campaign::{champion, rival, seals, RIVALS, SEALS};
use crate::clock::{self, Phase};
use crate::duelists::{duelist, Duelist, Rule, DUELISTS};
use crate::overworld::Progress;
use crate::profile::Profile;
use sixfold_engine::ai::Difficulty;
use sixfold_engine::set::{self, PROMOS};
use sixfold_engine::T;
use sixfold_engine::{card, CardId, CardKind, EnergyCard};
use sixfold_engine::{End, GameSummary};

/// Hidden finds in the world's maps (`meta/tests/world_data.rs` counts them).
pub const FINDS: usize = 104;
/// Secrets (`flag:secret_<id>`) in the world's maps (`meta/tests/world_data.rs` counts them).
pub const SECRETS: usize = 28;
/// Curios (`curio_<name>` items) the maps give.
pub const CURIOS: usize = 12;
/// Waymarks charted in `web/world/atlas.json`.
pub const WAYMARKS: usize = 13;
/// The field items that open paths (`docs/WORLD_DESIGN.md` §7.1).
pub const FIELD_ITEMS: [&str; 4] = ["kiln_lantern", "ferry_pass", "wading_boots", "grip_gloves"];
/// The six seal sites, one per colour region, by map.
pub const SEAL_SITES: [&str; 6] = [
    "kilnmouth",
    "rodspire",
    "bellflats",
    "moonwell",
    "elder_ring",
    "bastion",
];

const PREFIX: &str = "s1;";

/// Named counters; a missing one is 0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats(BTreeMap<String, u64>);

/// What a finished duel was, for [`Stats::record_duel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duel {
    /// "Play a duel" against the AI at this level.
    Quick(Difficulty),
    /// A rival or a route duelist met in the world, by id.
    World(&'static str),
    Daily,
    Gauntlet,
}

fn valid_key(k: &str) -> bool {
    !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

impl Stats {
    pub fn get(&self, key: &str) -> u64 {
        self.0.get(key).copied().unwrap_or(0)
    }

    pub fn add(&mut self, key: &str, n: u64) {
        if n > 0 {
            let v = self.get(key).saturating_add(n);
            self.set(key, v);
        }
    }

    /// Raise a counter to `n` (a best, or a count read from elsewhere).
    pub fn max(&mut self, key: &str, n: u64) {
        if n > self.get(key) {
            self.set(key, n);
        }
    }

    fn set(&mut self, key: &str, n: u64) {
        debug_assert!(valid_key(key), "bad stats key {key:?}");
        if n == 0 {
            self.0.remove(key);
        } else {
            self.0.insert(key.to_string(), n);
        }
    }

    pub fn encode(&self) -> String {
        let body: Vec<String> = self.0.iter().map(|(k, n)| format!("{k}={n}")).collect();
        format!("{PREFIX}{}", body.join(","))
    }

    /// Reads [`encode`](Self::encode)'s string; anything malformed is skipped.
    pub fn decode(s: &str) -> Stats {
        let mut st = Stats::default();
        for part in s.trim().strip_prefix(PREFIX).unwrap_or("").split(',') {
            if let Some((k, n)) = part.split_once('=') {
                // Up to 1.10 the route duelists' wins were `trainer_wins` (#192);
                // an old tab may write both, so the higher one wins.
                let k = if k == "trainer_wins" {
                    "duelist_wins"
                } else {
                    k
                };
                if let (true, Ok(n)) = (valid_key(k), n.parse::<u64>()) {
                    st.set(k, n.max(st.get(k)));
                }
            }
        }
        st
    }

    /// A save from before stats: count what the progress already shows
    /// (finds, secrets, duelists and rivals beaten). Never lowers a counter.
    pub fn backfill(&mut self, p: &Progress) {
        let n = |x: usize| x as u64;
        self.max("finds", n(p.count("found_")));
        self.max("secrets", n(p.count("secret_")));
        self.max(
            "duelist_wins",
            n(DUELISTS.iter().filter(|t| t.beaten(p)).count()),
        );
        self.max(
            "rival_wins",
            n(RIVALS.iter().filter(|r| r.beaten(p)).count()),
        );
        self.max("wins", self.get("duelist_wins") + self.get("rival_wins"));
        self.max("duels", self.get("wins"));
    }

    /// Count a finished duel, played as side 0 with `deck`. An unfinished
    /// one (no winner) counts for nothing.
    pub fn record_duel(&mut self, s: &GameSummary, duel: Duel, deck: &[CardId]) {
        let Some(w) = s.winner else {
            return;
        };
        let won = w == 0;
        let one = u64::from(won);
        self.add("duels", 1);
        self.add("prizes", s.taken[0].into());
        self.add("kos", s.kos[0].into());
        self.add("bench_kos", s.bench_kos[0].into());
        self.add("stage2_played", s.stage2[0].into());
        self.max("heads_run_best", s.heads_run[0].into());
        if won {
            self.add("wins", 1);
            self.add("streak", 1);
            self.max("streak_best", self.get("streak"));
        } else {
            self.set("streak", 0);
        }
        match duel {
            Duel::Quick(level) => {
                self.add("quick_wins", one);
                self.add(
                    "quick_wins_hard",
                    one * u64::from(level == Difficulty::Hard),
                );
            }
            Duel::World(id) if rival(id).is_some() => {
                self.add("rival_wins", one);
                self.add(
                    "mono_rival_wins",
                    one * u64::from(Rule::MonoColour.allows(deck)),
                );
            }
            Duel::World(id) => {
                self.add("duelist_wins", one);
                if duelist(id).is_some_and(Duelist::story) {
                    self.add("rook_lost", 1 - one);
                }
            }
            Duel::Daily | Duel::Gauntlet => {}
        }
        if won {
            self.add("deck_out_wins", u64::from(s.end == Some(End::DeckOut)));
            self.add(
                "empty_bench_wins",
                u64::from(s.end == Some(End::NoCreatures)),
            );
            self.add("sudden_death_wins", u64::from(s.sudden_death));
            self.add("comebacks", u64::from(s.brink));
            let clean = s.prizes == 6 && s.taken[1] == 0 && !s.sudden_death;
            self.add("clean_sweeps", u64::from(clean));
        }
    }

    /// Count what changed between two moments of the world: Facets in and
    /// out, packs opened, cards sold, game minutes and nightfalls, daily duels
    /// won, finds, secrets.
    pub fn observe(&mut self, before: (&Progress, &Profile), after: (&Progress, &Profile)) {
        let ((p0, f0), (p1, f1)) = (before, after);
        let (c0, c1) = (u64::from(p0.coins()), u64::from(p1.coins()));
        self.add("coins_in", c1.saturating_sub(c0));
        self.add("coins_out", c0.saturating_sub(c1));
        let (w0, w1) = (f0.wallet.total(), f1.wallet.total());
        self.add("packs", w0.saturating_sub(w1).into());
        // Only the shop takes cards out of the collection.
        let cards = |f: &Profile| f.collection.iter().map(|(_, n)| n as u64).sum::<u64>();
        self.add("sold", cards(f0).saturating_sub(cards(f1)));
        let mins = (p1.minute() + clock::DAY - p0.minute()) % clock::DAY;
        self.add("minutes", mins.into());
        self.add(
            "nights",
            u64::from(p0.phase() != Phase::Night && p1.phase() == Phase::Night),
        );
        self.add(
            "daily_days",
            u64::from(p1.daily().is_some() && p1.daily() != p0.daily()),
        );
        self.max("finds", p1.count("found_") as u64);
        self.max("secrets", p1.count("secret_") as u64);
    }

    /// The hero walked into a map: at a seal site, note the game minute (for
    /// the "all six within one day" achievement).
    pub fn visit(&mut self, map: &str) {
        if SEAL_SITES.contains(&map) {
            let now = self.get("minutes") + 1;
            self.set(&format!("site_{map}"), now);
        }
    }

    /// Every counter, in key order (the stats page picks what it shows).
    pub fn iter(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0.iter().map(|(k, n)| (k.as_str(), *n))
    }
}

/// One achievement (`docs/WORLD_DESIGN.md` §12.3).
#[derive(Debug)]
pub struct Achievement {
    pub id: &'static str,
    pub name: T,
    /// What earns it.
    pub goal: T,
    /// Shown as "???" until earned.
    pub hidden: bool,
    /// Facets it pays, once.
    pub facets: u32,
    test: fn(&Stats, &Progress, &Profile) -> bool,
}

impl Achievement {
    /// The progress flag set when it is earned.
    pub fn flag(&self) -> String {
        format!("ach_{}", self.id)
    }

    pub fn holds(&self, s: &Stats, p: &Progress, pf: &Profile) -> bool {
        (self.test)(s, p, pf)
    }
}

/// Route duelists (not Rook's story duels).
fn route() -> impl Iterator<Item = &'static Duelist> {
    DUELISTS.iter().filter(|t| !t.story())
}

fn all_beaten(p: &Progress, keep: fn(&Duelist) -> bool) -> bool {
    let mut some = route().filter(|t| keep(t)).peekable();
    some.peek().is_some() && some.all(|t| t.beaten(p))
}

/// Every card a player can collect: not a promo, not basic energy.
pub fn collectable() -> impl Iterator<Item = CardId> {
    set::ids().filter(|&id| {
        !set::is_promo(id) && !matches!(card(id).kind, CardKind::Energy(EnergyCard::Basic(_)))
    })
}

/// All six seal sites visited within one game day (1,440 game minutes).
fn ring(s: &Stats) -> bool {
    let at: Vec<u64> = SEAL_SITES
        .iter()
        .map(|m| s.get(&format!("site_{m}")))
        .collect();
    let (lo, hi) = (at.iter().min(), at.iter().max());
    lo.zip(hi)
        .is_some_and(|(&lo, &hi)| lo > 0 && hi - lo <= u64::from(clock::DAY))
}

macro_rules! ach {
    ($id:literal, $en:literal / $fr:literal, $goal:literal / $goal_fr:literal, $facets:literal, $hidden:literal, $test:expr) => {
        Achievement {
            id: $id,
            name: T::new($en, $fr),
            goal: T::new($goal, $goal_fr),
            hidden: $hidden,
            facets: $facets,
            test: $test,
        }
    };
}

/// The 41, in the order the page lists them.
pub static ACHIEVEMENTS: [Achievement; 41] = [
    ach!("first_win", "First Light" / "Première lueur", "Win any duel." / "Gagner un duel.", 25, false, |s, _, _| s.get("wins") >= 1),
    ach!("first_seal", "Hexed" / "Premier sceau", "Earn a seal." / "Gagner un sceau.", 50, false, |_, p, _| seals(p) >= 1),
    ach!("three_seals", "Halfway Round" / "À mi-chemin", "Earn three seals." / "Gagner trois sceaux.", 75, false, |_, p, _| seals(p) >= 3),
    ach!("all_seals", "Sixfold" / "Les Six", "Earn all six seals." / "Gagner les six sceaux.", 150, false, |_, p, _| seals(p) >= SEALS),
    ach!("champion", "Champion" / "Champion", "Beat Sable." / "Battre Sable.", 250, false, |_, p, _| champion(p)),
    ach!("seventh_face", "The Seventh Face" / "La septième face", "Finish Rook's story." / "Terminer l'histoire de Rook.", 200, true, |_, p, _| p.has_flag("rook_4")),
    ach!("rook_sweep", "Blank Cheque" / "Chèque en blanc", "Win all four of Rook's duels, each at the first try." / "Gagner les quatre duels de Rook, chacun du premier coup.", 150, true,
        |s, p, _| s.get("rook_lost") == 0 && (1..=4).all(|n| p.has_flag(&format!("beat_rook_{n}")))),
    ach!("gauntlet", "Unbroken" / "Invaincu", "Clear the Gauntlet." / "Venir à bout de l'Épreuve.", 250, false, |_, p, _| p.has_flag(crate::endgame::CLEAR_FLAG)),
    ach!("daily_7", "Regular" / "Habitué", "Win the daily duel on seven different days." / "Gagner le duel du jour sept jours différents.", 100, false, |s, _, _| s.get("daily_days") >= 7),
    ach!("duelists_10", "Waylaid" / "Pris en embuscade", "Beat ten route duelists." / "Battre dix duellistes des routes.", 50, false, |_, p, _| route().filter(|t| t.beaten(p)).count() >= 10),
    ach!("duelists_all", "Nobody Left" / "Plus personne", "Beat every route duelist." / "Battre tous les duellistes des routes.", 200, false, |_, p, _| all_beaten(p, |_| true)),
    ach!("night_owl", "Night Owl" / "Oiseau de nuit", "Beat every night duelist." / "Battre tous les duellistes de la nuit.", 150, false, |_, p, _| all_beaten(p, |t| t.night)),
    ach!("rules_all", "House Rules" / "Règles maison", "Beat every challenge-rule duelist." / "Battre tous les duellistes à règle imposée.", 150, false, |_, p, _| all_beaten(p, |t| t.rule.is_some())),
    ach!("finds_10", "Magpie" / "Pie voleuse", "Make ten finds." / "Faire dix trouvailles.", 50, false, |_, p, _| p.count("found_") >= 10),
    ach!("finds_all", "Nothing Hidden" / "Rien de caché", "Make every find." / "Faire toutes les trouvailles.", 250, false, |_, p, _| p.count("found_") >= FINDS),
    ach!("curios_all", "Curator" / "Conservateur", "Collect every curio." / "Réunir toutes les curiosités.", 100, false, |_, p, _| p.count("curio_") >= CURIOS),
    ach!("secrets_1", "Curious" / "Curieux", "Find a secret." / "Trouver un secret.", 25, false, |_, p, _| p.count("secret_") >= 1),
    ach!("secrets_half", "Sleuth" / "Fin limier", "Find half the secrets." / "Trouver la moitié des secrets.", 100, false, |_, p, _| p.count("secret_") >= SECRETS.div_ceil(2)),
    ach!("secrets_all", "Nothing Escapes You" / "Rien ne vous échappe", "Find every secret." / "Trouver tous les secrets.", 250, false, |_, p, _| p.count("secret_") >= SECRETS),
    ach!("waymarks", "Wayfarer" / "Grand voyageur", "Touch every waymark." / "Toucher toutes les bornes.", 75, false, |_, p, _| p.waymarks().count() >= WAYMARKS),
    ach!("ring_day", "Around the Ring" / "Le tour de l'anneau", "Visit all six seal sites within one game day." / "Visiter les six sites de sceau en une journée de jeu.", 150, true, |s, _, _| ring(s)),
    ach!("items_all", "Well Equipped" / "Bien équipé", "Own all four field items." / "Posséder les quatre objets de terrain.", 75, false, |_, p, _| FIELD_ITEMS.iter().all(|i| p.has_item(i))),
    ach!("nights_10", "Insomniac" / "Insomniaque", "See ten nights fall in the world." / "Voir tomber dix nuits sur le monde.", 50, false, |s, _, _| s.get("nights") >= 10),
    ach!("cards_50", "Half a Binder" / "Un demi-classeur", "Own 50 different cards." / "Posséder 50 cartes différentes.", 75, false, |_, _, pf| collectable().filter(|&id| pf.collection.count(id) > 0).count() >= 50),
    ach!("cards_all", "Complete Set" / "Collection complète", "Own every card but the promos." / "Posséder toutes les cartes hors promos.", 250, false, |_, _, pf| collectable().all(|id| pf.collection.count(id) > 0)),
    ach!("promos_all", "Signed Copies" / "Exemplaires dédicacés", "Own all seven promos." / "Posséder les sept cartes promo.", 150, false, |_, _, pf| PROMOS.iter().all(|&id| pf.collection.count(id) > 0)),
    ach!("packs_25", "Pack Rat" / "Collectionneur compulsif", "Open 25 packs." / "Ouvrir 25 boosters.", 50, false, |s, _, _| s.get("packs") >= 25),
    ach!("craft_first", "Hot off the Press" / "Tout juste imprimé", "Craft a card." / "Fabriquer une carte.", 25, false, |s, _, _| s.get("crafts") >= 1),
    ach!("sold_100", "Clearance" / "Liquidation", "Sell 100 cards." / "Vendre 100 cartes.", 50, false, |s, _, _| s.get("sold") >= 100),
    ach!("hoard", "Facet Hoard" / "Trésor de Facettes", "Hold ⬡ 2,000 at once." / "Détenir ⬡ 2 000 d'un coup.", 50, false, |_, p, _| p.coins() >= 2000),
    ach!("spender", "Big Spender" / "Panier percé", "Spend ⬡ 5,000 in all." / "Dépenser ⬡ 5 000 en tout.", 75, false, |s, _, _| s.get("coins_out") >= 5000),
    ach!("clean_sweep", "Clean Sweep" / "Sans bavure", "Win a 6-prize duel without giving up a prize." / "Gagner un duel à 6 récompenses sans en céder une seule.", 100, false, |s, _, _| s.get("clean_sweeps") >= 1),
    ach!("comeback", "Comeback" / "Retournement", "Win after needing four prizes or more while the opponent needed one." / "Gagner alors qu'il vous fallait quatre récompenses ou plus et à l'adversaire une seule.", 100, true, |s, _, _| s.get("comebacks") >= 1),
    ach!("deck_out", "Patience" / "Patience", "Win because the opponent's deck ran out." / "Gagner parce que le deck adverse est épuisé.", 50, false, |s, _, _| s.get("deck_out_wins") >= 1),
    ach!("empty_bench", "Empty Bench" / "Banc vide", "Win by leaving the opponent no Hexon in play." / "Gagner en ne laissant aucun Hexon en jeu à l'adversaire.", 50, false, |s, _, _| s.get("empty_bench_wins") >= 1),
    ach!("sudden_death", "Sudden Death" / "Mort subite", "Win a sudden-death duel." / "Gagner un duel en mort subite.", 75, false, |s, _, _| s.get("sudden_death_wins") >= 1),
    ach!("full_bloom", "Full Bloom" / "Pleine floraison", "Play an Awaken 2 Hexon." / "Jouer un Hexon d'Éveil 2.", 25, false, |s, _, _| s.get("stage2_played") >= 1),
    ach!("ko_200", "Knockout Artist" / "Artiste du K.O.", "Score 200 knock-outs." / "Réussir 200 K.O.", 100, false, |s, _, _| s.get("kos") >= 200),
    ach!("heads_5", "Lucky Streak" / "La main heureuse", "Flip five heads in a row in one duel." / "Obtenir cinq faces d'affilée dans un duel.", 50, false, |s, _, _| s.get("heads_run_best") >= 5),
    ach!("mono_rival", "True Colours" / "Haut en couleur", "Beat a rival with a one-colour deck." / "Battre un rival avec un deck d'une seule couleur.", 75, false, |s, _, _| s.get("mono_rival_wins") >= 1),
    ach!("streak_5", "On a Roll" / "Sur une lancée", "Win five duels in a row." / "Gagner cinq duels d'affilée.", 75, false, |s, _, _| s.get("streak_best") >= 5),
];

/// Every achievement whose condition holds now, earned before or not.
pub fn earned(s: &Stats, p: &Progress, pf: &Profile) -> Vec<&'static Achievement> {
    ACHIEVEMENTS.iter().filter(|a| a.holds(s, p, pf)).collect()
}

/// Grant the achievements newly earned: each sets its flag and pays its
/// Facets, once (a granted one never pays again). Returns them.
pub fn award(s: &Stats, p: &mut Progress, pf: &Profile) -> Vec<&'static Achievement> {
    let new: Vec<_> = earned(s, p, pf)
        .into_iter()
        .filter(|a| !p.has_flag(&a.flag()))
        .collect();
    for a in &new {
        p.apply(&format!("flag:{}", a.flag()))
            .expect("achievement ids are valid flag names");
        p.earn(a.facets);
    }
    new
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::record;
    use sixfold_engine::fixtures::*;

    fn won(end: End) -> GameSummary {
        GameSummary {
            winner: Some(0),
            end: Some(end),
            prizes: 6,
            taken: [6, 2],
            kos: [6, 2],
            bench_kos: [1, 0],
            stage2: [2, 0],
            heads_run: [3, 1],
            ..GameSummary::default()
        }
    }

    #[test]
    fn stats_round_trip_sorted_without_zeros() {
        let mut s = Stats::default();
        s.add("wins", 3);
        s.add("duels", 5);
        s.add("kos", 0);
        s.max("heads_run_best", 4);
        s.max("heads_run_best", 2);
        assert_eq!(s.encode(), "s1;duels=5,heads_run_best=4,wins=3");
        assert_eq!(Stats::decode(&s.encode()), s);
        assert_eq!(s.get("kos"), 0);
    }

    #[test]
    fn stats_decode_skips_junk_and_keeps_unknown_keys() {
        let s = Stats::decode("s1;wins=2,Bad=1,x=,=3,later_thing=7,kos=-1");
        assert_eq!(s.get("wins"), 2);
        assert_eq!(s.get("later_thing"), 7);
        assert_eq!(s.iter().count(), 2);
        assert_eq!(Stats::decode(""), Stats::default());
        assert_eq!(Stats::decode("s9;wins=2"), Stats::default());
    }

    #[test]
    fn backfill_counts_what_the_progress_shows() {
        let mut p = Progress::default();
        for f in [
            "found_m_flowers",
            "found_w_lookout",
            "secret_fern_glade",
            "beat_t_tobin",
            "beat_bramble",
        ] {
            p.apply(&format!("flag:{f}")).unwrap();
        }
        let mut s = Stats::default();
        s.backfill(&p);
        assert_eq!(
            (s.get("finds"), s.get("duelist_wins"), s.get("rival_wins")),
            (2, 1, 1)
        );
        assert_eq!(s.get("secrets"), 1);
        assert_eq!((s.get("wins"), s.get("duels")), (2, 2));
        // Never lowers what was counted.
        s.add("wins", 10);
        s.backfill(&p);
        assert_eq!(s.get("wins"), 12);
    }

    #[test]
    fn record_duel_counts_wins_streaks_and_how() {
        let mut s = Stats::default();
        let deck = RIVALS[0].deck.cards();
        s.record_duel(&won(End::DeckOut), Duel::Quick(Difficulty::Hard), &deck);
        s.record_duel(&won(End::NoCreatures), Duel::World("t_tobin"), &deck);
        assert_eq!((s.get("duels"), s.get("wins"), s.get("streak")), (2, 2, 2));
        assert_eq!((s.get("quick_wins"), s.get("quick_wins_hard")), (1, 1));
        assert_eq!((s.get("duelist_wins"), s.get("rival_wins")), (1, 0));
        assert_eq!((s.get("deck_out_wins"), s.get("empty_bench_wins")), (1, 1));
        assert_eq!(
            (s.get("prizes"), s.get("kos"), s.get("bench_kos")),
            (12, 12, 2)
        );
        assert_eq!((s.get("stage2_played"), s.get("heads_run_best")), (4, 3));
        let lost = GameSummary {
            winner: Some(1),
            ..won(End::Prizes)
        };
        s.record_duel(&lost, Duel::World("rook_1"), &deck);
        assert_eq!((s.get("streak"), s.get("streak_best")), (0, 2));
        assert_eq!(
            (s.get("rook_lost"), s.get("wins"), s.get("duels")),
            (1, 2, 3)
        );
        // An unfinished duel counts for nothing.
        let before = s.clone();
        s.record_duel(&GameSummary::default(), Duel::Daily, &deck);
        assert_eq!(s, before);
    }

    #[test]
    fn record_duel_spots_the_special_wins() {
        let mono = RIVALS[0].deck.cards();
        assert!(Rule::MonoColour.allows(&mono));
        let mut s = Stats::default();
        let clean = GameSummary {
            taken: [6, 0],
            brink: true,
            ..won(End::Prizes)
        };
        s.record_duel(&clean, Duel::World("bramble"), &mono);
        assert_eq!((s.get("rival_wins"), s.get("mono_rival_wins")), (1, 1));
        assert_eq!((s.get("clean_sweeps"), s.get("comebacks")), (1, 1));
        // Four prizes is no clean sweep, even untouched; nor is a sudden death.
        let short = GameSummary {
            prizes: 4,
            ..clean.clone()
        };
        s.record_duel(&short, Duel::Gauntlet, &mono);
        let sudden = GameSummary {
            sudden_death: true,
            ..clean
        };
        s.record_duel(&sudden, Duel::Gauntlet, &mono);
        assert_eq!((s.get("clean_sweeps"), s.get("sudden_death_wins")), (1, 1));
    }

    #[test]
    fn observe_counts_facets_packs_time_and_nights() {
        let p0 = Progress::default();
        let f0 = Profile::new_player();
        let (mut p1, mut f1) = (p0.clone(), f0.clone());
        p1.earn(250);
        f1.wallet.take(0);
        p1.set_minute(clock::Phase::Night.start() + 30);
        let mut s = Stats::default();
        s.observe((&p0, &f0), (&p1, &f1));
        assert_eq!(
            (s.get("coins_in"), s.get("coins_out"), s.get("packs")),
            (250, 0, 1)
        );
        assert_eq!(s.get("nights"), 1);
        assert_eq!(
            s.get("minutes"),
            u64::from(clock::Phase::Night.start() + 30 - clock::START)
        );
        // Spending, and the clock wrapping past midnight.
        let mut p2 = p1.clone();
        p2.spend(100);
        p2.set_minute(60);
        s.observe((&p1, &f1), (&p2, &f1));
        assert_eq!((s.get("coins_out"), s.get("nights")), (100, 1));
        // Cards sold to the trader.
        let mut f2 = f1.clone();
        let spare = f2.collection.iter().next().unwrap().0;
        f2.collection.remove(spare, 1);
        s.observe((&p2, &f1), (&p2, &f2));
        assert_eq!(s.get("sold"), 1);
        // A daily duel won on a new day.
        let mut pr = f1.clone();
        crate::endgame::record_daily(&mut p2.clone(), &mut pr, 3, false);
        let mut p3 = p2.clone();
        crate::endgame::record_daily(&mut p3, &mut pr, 3, true);
        s.observe((&p2, &f1), (&p3, &pr));
        assert_eq!(s.get("daily_days"), 1);
        s.observe((&p3, &pr), (&p3, &pr));
        assert_eq!(s.get("daily_days"), 1);
        // A secret discovered.
        let mut p4 = p3.clone();
        p4.apply("flag:secret_moon_glade").unwrap();
        s.observe((&p3, &pr), (&p4, &pr));
        assert_eq!(s.get("secrets"), 1);
    }

    #[test]
    fn a_real_rival_win_feeds_the_counters() {
        let r = &RIVALS[0];
        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        let before = (p.clone(), pr.clone());
        record(r, true, &mut p, &mut pr);
        let mut s = Stats::default();
        s.observe((&before.0, &before.1), (&p, &pr));
        assert_eq!(s.get("coins_in"), u64::from(crate::campaign::RIVAL_PURSE));
        // A whole duel through the engine, won by deck-out.
        let mut g = sixfold_engine::testkit::main_phase(0);
        sixfold_engine::testkit::empty_deck(&mut g, 1);
        sixfold_engine::testkit::ok(&mut g, sixfold_engine::Action::Done);
        s.record_duel(&g.summary(), Duel::World(r.id), &[GRASS_E]);
        assert_eq!((s.get("wins"), s.get("deck_out_wins")), (1, 1));
    }

    /// The least that earns achievement `id`, from a new player.
    fn reach(id: &str) -> (Stats, Progress, Profile) {
        let (mut s, mut p, mut pf) = (Stats::default(), Progress::default(), Profile::new_player());
        let flag = |p: &mut Progress, f: &str| _ = p.apply(&format!("flag:{f}")).unwrap();
        let beat = |p: &mut Progress, keep: fn(&Duelist) -> bool, n: usize| {
            for t in route().filter(|t| keep(t)).take(n) {
                p.apply(&format!("flag:{}", t.flag())).unwrap();
            }
        };
        let seal_rivals = |p: &mut Progress, n: usize| {
            for r in RIVALS.iter().filter(|r| r.seal).take(n) {
                p.apply(&format!("flag:{}", r.flag())).unwrap();
            }
        };
        match id {
            "first_win" => s.add("wins", 1),
            "first_seal" => seal_rivals(&mut p, 1),
            "three_seals" => seal_rivals(&mut p, 3),
            "all_seals" => seal_rivals(&mut p, 6),
            "champion" => flag(&mut p, "beat_warden"),
            "seventh_face" => flag(&mut p, "rook_4"),
            "rook_sweep" => (1..=4).for_each(|n| flag(&mut p, &format!("beat_rook_{n}"))),
            "gauntlet" => flag(&mut p, crate::endgame::CLEAR_FLAG),
            "daily_7" => s.add("daily_days", 7),
            "duelists_10" => beat(&mut p, |_| true, 10),
            "duelists_all" => beat(&mut p, |_| true, usize::MAX),
            "night_owl" => beat(&mut p, |t| t.night, usize::MAX),
            "rules_all" => beat(&mut p, |t| t.rule.is_some(), usize::MAX),
            "finds_10" => (0..10).for_each(|i| flag(&mut p, &format!("found_f{i}"))),
            "finds_all" => (0..FINDS).for_each(|i| flag(&mut p, &format!("found_f{i}"))),
            "curios_all" => {
                (0..CURIOS).for_each(|i| _ = p.apply(&format!("give:curio_{i}")).unwrap())
            }
            "secrets_1" => flag(&mut p, "secret_s0"),
            "secrets_half" => {
                (0..SECRETS.div_ceil(2)).for_each(|i| flag(&mut p, &format!("secret_s{i}")))
            }
            "secrets_all" => (0..SECRETS).for_each(|i| flag(&mut p, &format!("secret_s{i}"))),
            "waymarks" => (0..WAYMARKS).for_each(|i| flag(&mut p, &format!("way_w{i}"))),
            "ring_day" => {
                for m in SEAL_SITES {
                    s.add("minutes", 200);
                    s.visit(m);
                }
            }
            "items_all" => FIELD_ITEMS
                .iter()
                .for_each(|i| _ = p.apply(&format!("give:{i}")).unwrap()),
            "nights_10" => s.add("nights", 10),
            "cards_50" => collectable()
                .take(50)
                .for_each(|id| pf.collection.add(id, 1)),
            "cards_all" => collectable().for_each(|id| pf.collection.add(id, 1)),
            "promos_all" => PROMOS.iter().for_each(|&id| pf.collection.add(id, 1)),
            "packs_25" => s.add("packs", 25),
            "craft_first" => s.add("crafts", 1),
            "sold_100" => s.add("sold", 100),
            "hoard" => p.earn(2000),
            "spender" => s.add("coins_out", 5000),
            "clean_sweep" => s.add("clean_sweeps", 1),
            "comeback" => s.add("comebacks", 1),
            "deck_out" => s.add("deck_out_wins", 1),
            "empty_bench" => s.add("empty_bench_wins", 1),
            "sudden_death" => s.add("sudden_death_wins", 1),
            "full_bloom" => s.add("stage2_played", 1),
            "ko_200" => s.add("kos", 200),
            "heads_5" => s.add("heads_run_best", 5),
            "mono_rival" => s.add("mono_rival_wins", 1),
            "streak_5" => s.add("streak_best", 5),
            other => panic!("no test for achievement {other}"),
        }
        (s, p, pf)
    }

    #[test]
    fn a_new_player_has_earned_nothing() {
        let (s, p, pf) = (Stats::default(), Progress::default(), Profile::new_player());
        assert!(earned(&s, &p, &pf).is_empty(), "{:?}", earned(&s, &p, &pf));
    }

    /// One condition per achievement: the least that earns it does.
    #[test]
    fn every_achievement_has_a_condition_that_earns_it() {
        for a in &ACHIEVEMENTS {
            let (s, p, pf) = reach(a.id);
            assert!(a.holds(&s, &p, &pf), "{} not earned", a.id);
        }
    }

    /// Thresholds: one short of the count earns nothing.
    #[test]
    fn one_short_earns_nothing() {
        let (mut s, mut p, pf) = (Stats::default(), Progress::default(), Profile::new_player());
        for (k, n) in [
            ("daily_days", 6),
            ("nights", 9),
            ("packs", 24),
            ("sold", 99),
            ("coins_out", 4999),
            ("kos", 199),
            ("heads_run_best", 4),
            ("streak_best", 4),
        ] {
            s.add(k, n);
        }
        p.earn(1899);
        (0..9).for_each(|i| _ = p.apply(&format!("flag:found_f{i}")).unwrap());
        for t in route().take(9) {
            p.apply(&format!("flag:{}", t.flag())).unwrap();
        }
        // Three of Rook's four, or all four after a loss.
        (1..=3).for_each(|n| _ = p.apply(&format!("flag:beat_rook_{n}")).unwrap());
        assert!(earned(&s, &p, &pf).is_empty(), "{:?}", earned(&s, &p, &pf));
        p.apply("flag:beat_rook_4").unwrap();
        s.add("rook_lost", 1);
        assert!(earned(&s, &p, &pf).is_empty());
        // Five seal sites, or six spread over more than a day.
        let mut s = Stats::default();
        for m in &SEAL_SITES[..5] {
            s.visit(m);
        }
        assert!(!ring(&s));
        s.add("minutes", u64::from(clock::DAY) + 1);
        s.visit(SEAL_SITES[5]);
        assert!(!ring(&s));
        // Revisiting one isn't enough; all six again within a day is.
        s.visit(SEAL_SITES[0]);
        assert!(!ring(&s));
        for m in SEAL_SITES {
            s.visit(m);
        }
        assert!(ring(&s));
        // Every secret but one.
        let (s, mut p) = (Stats::default(), Progress::default());
        (1..SECRETS).for_each(|i| _ = p.apply(&format!("flag:secret_s{i}")).unwrap());
        assert!(!earned(&s, &p, &pf).iter().any(|a| a.id == "secrets_all"));
    }

    #[test]
    fn achievements_are_41_unique_and_pay_4350() {
        let ids: std::collections::HashSet<_> = ACHIEVEMENTS.iter().map(|a| a.id).collect();
        assert_eq!(ids.len(), 41);
        assert_eq!(ACHIEVEMENTS.iter().map(|a| a.facets).sum::<u32>(), 4350);
        let hidden: Vec<_> = ACHIEVEMENTS
            .iter()
            .filter(|a| a.hidden)
            .map(|a| a.id)
            .collect();
        assert_eq!(
            hidden,
            ["seventh_face", "rook_sweep", "ring_day", "comeback"]
        );
        assert!(ACHIEVEMENTS.iter().all(|a| valid_key(&a.flag())));
    }

    /// "All" means what the data holds: every route duelist, night duelist
    /// and challenge-rule duelist in `DUELISTS`, every card in the set.
    #[test]
    fn all_totals_come_from_the_data() {
        assert_eq!(route().filter(|t| t.night).count(), 6);
        assert_eq!(route().filter(|t| t.rule.is_some()).count(), 6);
        let (s, mut p, pf) = reach("duelists_all");
        let last = route().last().unwrap();
        p = Progress::decode(&p.encode().replace(&format!("f:{},", last.flag()), ""));
        assert!(!earned(&s, &p, &pf).iter().any(|a| a.id == "duelists_all"));
        let (s, p, mut pf) = reach("cards_all");
        let one = collectable().last().unwrap();
        pf.collection.remove(one, 1);
        assert!(!earned(&s, &p, &pf).iter().any(|a| a.id == "cards_all"));
        assert!(collectable().all(|id| !set::is_promo(id)));
    }

    #[test]
    fn award_flags_and_pays_once() {
        let (mut s, mut p, pf) = (Stats::default(), Progress::default(), Profile::new_player());
        s.add("wins", 1);
        s.add("stage2_played", 1);
        let coins = p.coins();
        let got: Vec<_> = award(&s, &mut p, &pf).iter().map(|a| a.id).collect();
        assert_eq!(got, ["first_win", "full_bloom"]);
        assert!(p.has_flag("ach_first_win"));
        assert_eq!(p.coins(), coins + 50);
        // Checking again pays nothing.
        assert!(award(&s, &mut p, &pf).is_empty());
        assert_eq!(p.coins(), coins + 50);
    }
}
