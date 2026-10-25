//! The campaign: six rival duelists, one per colour theme, and the Hex Warden
//! behind them. Beating a rival the first time earns its seal, packs of its
//! colours and a purse of Facets; a rematch win earns Facets only. Seals open
//! the later rivals and, all six, the Warden.
//!
//! State lives in the world's [`Progress`]: a beaten rival is the flag
//! `beat_<id>`, and the result of the last campaign duel is kept until its
//! post-duel line has played (see [`Progress::last_duel`]). Packs go to the
//! profile's wallet, and the first win against each rival also adds its promo
//! card ([`Rival::promo`]) to the collection.
//!
//! [`opponent`] is the one view of anyone the player can duel in the world
//! (a rival or a route duelist, [`crate::trainers`]), and [`record_opponent`]
//! the one way to record such a duel.

use crate::ai::{Difficulty, Style};
use crate::lang::T;
use crate::trainers::{self, trainer};

use crate::card::{CardId, Color};
use crate::decks::{Prebuilt, EMBER_RUSH, STORM_FRONT};
use crate::overworld::Progress;
use crate::profile::Profile;
use crate::set::*;

/// Seals there are to earn: one per rival, the Warden excluded.
pub const SEALS: usize = 6;

/// Facets a rival's first win pays; the Warden's.
pub const RIVAL_PURSE: u32 = 300;
pub const WARDEN_PURSE: u32 = 600;
/// Facets a rematch win pays per prize (no pack).
pub const REMATCH_PER_PRIZE: u32 = 10;

/// What a rival says in a duel, shown as a bubble from their portrait.
#[derive(Debug)]
pub struct RivalLines {
    /// When setup ends.
    pub intro: T,
    /// Their first knock-out of one of the player's creatures.
    pub first_ko: T,
    /// They need one prize or fewer to lose.
    pub ropes: T,
    /// The game-over screen, when the player won.
    pub defeat: T,
}

#[derive(Debug)]
pub struct Rival {
    /// Lowercase id used by dialogue (`duel:<id>`) and flags (`beat_<id>`).
    pub id: &'static str,
    pub name: &'static str,
    pub title: T,
    /// Where to find them, for the roster.
    pub place: T,
    pub colors: &'static [Color],
    pub deck: Prebuilt,
    pub difficulty: Difficulty,
    pub prizes: u8,
    /// Seals needed before they accept a duel.
    pub needs: usize,
    /// The first win earns a seal (every rival but the Warden).
    pub seal: bool,
    /// Index into [`crate::booster::BOOSTERS`] of the packs they give.
    pub booster: usize,
    /// Packs the first win grants.
    pub packs: u32,
    pub style: Style,
    pub lines: RivalLines,
}

impl Rival {
    pub fn flag(&self) -> String {
        format!("beat_{}", self.id)
    }

    pub fn beaten(&self, p: &Progress) -> bool {
        p.has_flag(&self.flag())
    }

    pub fn open(&self, p: &Progress) -> bool {
        seals(p) >= self.needs
    }

    /// The promo card the first win against this rival gives.
    pub fn promo(&self) -> CardId {
        let i = RIVALS
            .iter()
            .position(|r| r.id == self.id)
            .expect("a rival from RIVALS");
        PROMOS[i]
    }

    /// Packs a win would grant now: (booster, count); none for a rematch.
    pub fn prize_packs(&self, p: &Progress) -> (usize, u32) {
        let n = if self.beaten(p) { 0 } else { self.packs };
        (self.booster, n)
    }

    /// Facets a win would pay now.
    pub fn purse(&self, p: &Progress) -> u32 {
        match (self.beaten(p), self.seal) {
            (true, _) => REMATCH_PER_PRIZE * u32::from(self.prizes),
            (false, true) => RIVAL_PURSE,
            (false, false) => WARDEN_PURSE,
        }
    }
}

/// What a finished campaign duel gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reward {
    pub won: bool,
    /// First win against this rival.
    pub first: bool,
    /// A new seal was earned.
    pub seal: bool,
    /// (booster, count); count 0 on a loss.
    pub packs: (usize, u32),
    /// The promo card added to the collection (first win only).
    pub promo: Option<CardId>,
    /// Facets earned.
    pub facets: u32,
}

/// Rival by id.
pub fn rival(id: &str) -> Option<&'static Rival> {
    RIVALS.iter().find(|r| r.id == id)
}

/// What a duel is for; its music follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stakes {
    /// A route duelist, a quick duel, the coach.
    Ordinary,
    /// A rival, in their first colour.
    Rival(Color),
    /// The Warden: the one rival with no seal to give.
    Boss,
}

/// The stakes of a duel against `id`, a rival's or route duelist's id.
pub fn stakes(id: Option<&str>) -> Stakes {
    match id.and_then(rival) {
        Some(r) if !r.seal => Stakes::Boss,
        Some(r) => Stakes::Rival(r.colors[0]),
        None => Stakes::Ordinary,
    }
}

/// Seals earned so far.
pub fn seals(p: &Progress) -> usize {
    RIVALS.iter().filter(|r| r.seal && r.beaten(p)).count()
}

/// The Warden has been beaten: the player is the champion.
pub fn champion(p: &Progress) -> bool {
    RIVALS.iter().any(|r| !r.seal && r.beaten(p))
}

/// Saves from before promos existed: give the promo of every rival already
/// beaten that the collection lacks. Returns the cards added.
pub fn backfill_promos(p: &Progress, profile: &mut Profile) -> Vec<CardId> {
    let missing: Vec<CardId> = RIVALS
        .iter()
        .filter(|r| r.beaten(p) && profile.collection.count(r.promo()) == 0)
        .map(Rival::promo)
        .collect();
    missing.iter().for_each(|&id| profile.collection.add(id, 1));
    missing
}

/// Record a finished duel against `r`: remember the result for the post-duel
/// line, and on a win mark the rival beaten and grant the packs (and, the
/// first time, the rival's promo card).
pub fn record(r: &Rival, won: bool, p: &mut Progress, profile: &mut Profile) -> Reward {
    let first = won && !r.beaten(p);
    let packs = if won {
        r.prize_packs(p)
    } else {
        (r.booster, 0)
    };
    let facets = if won { r.purse(p) } else { 0 };
    if won {
        p.apply(&format!("flag:{}", r.flag()))
            .expect("rival ids are valid flag names");
        profile.wallet.grant(packs.0, packs.1);
        p.earn(facets);
    }
    let promo = first.then(|| r.promo());
    if let Some(id) = promo {
        profile.collection.add(id, 1);
    }
    p.set_last_duel(r.id, won);
    Reward {
        won,
        first,
        seal: first && r.seal,
        packs,
        promo,
        facets,
    }
}

/// Anyone the player can duel in the world, as the challenge sheet and the
/// duel screen show them.
#[derive(Debug, Clone)]
pub struct Opponent {
    pub id: &'static str,
    pub name: &'static str,
    /// A rival's title, a route duelist's class.
    pub title: T,
    pub deck: Vec<CardId>,
    pub prizes: u8,
    pub difficulty: Difficulty,
    pub style: Style,
    /// A rival's duel lines; a route duelist's only line is its map
    /// `challenge`.
    pub lines: Option<&'static RivalLines>,
    /// A campaign rival (seals, packs, promo), not a route duelist.
    pub rival: bool,
}

/// The rival or route duelist with this id.
pub fn opponent(id: &str) -> Option<Opponent> {
    if let Some(r) = rival(id) {
        return Some(Opponent {
            id: r.id,
            name: r.name,
            title: r.title,
            deck: r.deck.cards(),
            prizes: r.prizes,
            difficulty: r.difficulty,
            style: r.style,
            lines: Some(&r.lines),
            rival: true,
        });
    }
    trainer(id).map(|t| Opponent {
        id: t.id,
        name: t.name,
        title: t.class,
        deck: t.deck(),
        prizes: t.prizes,
        difficulty: t.difficulty,
        style: Style::Balanced,
        lines: None,
        rival: false,
    })
}

/// Record a finished duel against a rival or a route duelist (by id). A
/// duelist's reward is its purse on the first win, no packs. None for an
/// unknown id.
pub fn record_opponent(
    id: &str,
    won: bool,
    p: &mut Progress,
    profile: &mut Profile,
) -> Option<Reward> {
    if let Some(r) = rival(id) {
        return Some(record(r, won, p, profile));
    }
    let t = trainer(id)?;
    let first = won && !t.beaten(p);
    let facets = trainers::record(t, won, p);
    Some(Reward {
        won,
        first,
        seal: false,
        packs: (crate::booster::WILD, 0),
        promo: None,
        facets,
    })
}

/// Facets a quick duel win against the AI pays, by its level.
pub fn quick_duel_purse(level: Difficulty) -> u32 {
    match level {
        Difficulty::Easy => 10,
        Difficulty::Normal => 20,
        Difficulty::Hard => 40,
    }
}

/// Record a quick duel ("Play a duel") against the AI at `level`: a win pays
/// [`quick_duel_purse`], returned; a loss nothing.
pub fn record_quick_duel(p: &mut Progress, won: bool, level: Difficulty) -> u32 {
    let n = if won { quick_duel_purse(level) } else { 0 };
    p.earn(n);
    n
}

use Color::*;

/// The rivals in campaign order, the Warden last.
pub static RIVALS: [Rival; 7] = [
    Rival {
        id: "bramble",
        name: "Bramble",
        title: T::new("Hedge Tender", "Soigne-haies"),
        place: T::new("Fernhollow Trail, by the bridge", "Sentier de Creuse-Fougère, près du pont"),
        colors: &[Grass],
        deck: Prebuilt {
            name: T::new("Thicket", "Fourré"),
            description: T::new("Grove Basics and their first Awakenings, healing on the side.", "Des Hexons Bosquet de Base et leurs premiers Éveils, avec des soins en appoint."),
            list: &[
                (SPRIGGET, 4),
                (THORNBUCK, 3),
                (MIREBUD, 3),
                (BOGBLOSSOM, 2),
                (POLLENPUP, 3),
                (HONEYMANE, 2),
                (FERNWHISK, 2),
                (BRIARHOG, 1),
                (LUCKY_SATCHEL, 2),
                (MOSS_POULTICE, 2),
                (CLEARWATER_VIAL, 2),
                (DOWSING_ROD, 2),
                (GRASS_ENERGY, 32),
            ],
        },
        // Normal since 1.3: the Easy AI got weaker, and Bramble (Grass
        // against the starter's Fire) is the softest rival either way.
        difficulty: Difficulty::Normal,
        prizes: 2,
        needs: 0,
        seal: true,
        booster: 3, // Grove
        packs: 2,
        style: Style::Stall,
        lines: RivalLines {
            intro: T::new("Easy now. A hedge grows one leaf at a time, and so will this duel.", "Doucement. Une haie pousse une feuille à la fois, et ce duel aussi."),
            first_ko: T::new("Pruned. It'll grow back stronger. Yours might not.", "Taillé. Ça repoussera plus fort. Le tien, je n'en suis pas sûr."),
            ropes: T::new("You've found the gap in my hedge. Mind the thorns on the way through.", "Tu as trouvé le trou dans ma haie. Attention aux épines en passant."),
            defeat: T::new("Well grown. Go on, I've some pride to mulch.", "Bien poussé. Va, j'ai de la fierté à pailler."),
        },
    },
    Rival {
        id: "marlo",
        name: "Marlo",
        title: T::new("Tide Reader", "Liseur de marées"),
        place: T::new("Saltwind Boardwalk, at the end of the pier", "Promenade de Ventsalé, au bout de la jetée"),
        colors: &[Water, Psychic],
        deck: Prebuilt {
            name: T::new("Low Tide", "Marée basse"),
            description: T::new("Tide Hexons that grow sturdy, with a Dream sting.", "Des Hexons Marée qui deviennent coriaces, avec une piqûre Songe."),
            list: &[
                (PUDDOCK, 4),
                (BROOKHORN, 3),
                (KELPUP, 4),
                (KELPWARDEN, 3),
                (RILLET, 3),
                (STREAMFIN, 2),
                (WISPUN, 2),
                (WISPELLE, 1),
                (LUCKY_SATCHEL, 2),
                (TRAIL_COMPASS, 2),
                (MOSS_POULTICE, 2),
                (RELAY_BATON, 2),
                (DRAIN_BRUSH, 2),
                (WATER_ENERGY, 21),
                (PSYCHIC_ENERGY, 7),
            ],
        },
        difficulty: Difficulty::Easy,
        prizes: 3,
        needs: 0,
        seal: true,
        booster: 1, // Tide
        packs: 2,
        style: Style::Stall,
        lines: RivalLines {
            intro: T::new("The tide's turning. Question is which of us it carries out.", "La marée tourne. Reste à savoir lequel de nous deux elle emporte."),
            first_ko: T::new("Out with the ebb. It always comes back in, you know. Mine does.", "Parti avec le reflux. Ça revient toujours, tu sais. Chez moi, en tout cas."),
            ropes: T::new("Slack water. Nothing moves, except you, apparently.", "L'étale. Plus rien ne bouge, sauf toi, visiblement."),
            defeat: T::new("High-water mark. I'll chalk it on the piling.", "Laisse de haute mer. Je la marque à la craie sur le pilotis."),
        },
    },
    Rival {
        id: "cinza",
        name: "Cinza",
        title: T::new("Kiln Runner", "Coureuse du four"),
        place: T::new("Emberstep Quarry, down on the quarry floor", "Carrière de Pas-de-Braise, tout au fond"),
        colors: &[Fire, Fighting],
        deck: Prebuilt {
            name: EMBER_RUSH.name,
            description: EMBER_RUSH.description,
            list: EMBER_RUSH.list,
        },
        difficulty: Difficulty::Normal,
        prizes: 4,
        needs: 0,
        seal: true,
        booster: 0, // Ember
        packs: 2,
        style: Style::Aggressive,
        lines: RivalLines {
            intro: T::new("No warm-up, no waiting. The kiln's already hot!", "Pas d'échauffement, pas d'attente. Le four est déjà chaud !"),
            first_ko: T::new("Ha! Out of the fire and straight into the ash!", "Ha ! Sorti du feu, direct dans la cendre !"),
            ropes: T::new("Okay, okay, I'm stoking, I'm stoking!", "D'accord, d'accord, je rallume, je rallume !"),
            defeat: T::new("Burnt out. Rematch the second I've cooled down. So, now?", "Grillée. Revanche dès que j'ai refroidi. Donc, maintenant ?"),
        },
    },
    Rival {
        id: "quill",
        name: "Quill",
        title: T::new("Weather Watcher", "Vigie du ciel"),
        place: T::new("Saltwind Boardwalk, under the lighthouse", "Promenade de Ventsalé, sous le phare"),
        colors: &[Lightning, Water],
        deck: Prebuilt {
            name: STORM_FRONT.name,
            description: STORM_FRONT.description,
            list: STORM_FRONT.list,
        },
        difficulty: Difficulty::Normal,
        prizes: 4,
        needs: 3,
        seal: true,
        booster: 4, // Storm
        packs: 2,
        style: Style::BenchSniper,
        lines: RivalLines {
            intro: T::new("Barometer's falling. Expect scattered strikes, mostly on your bench.", "Le baromètre chute. Prévois des éclairs épars, surtout sur ton Banc."),
            first_ko: T::new("Called it. I always log a direct hit.", "Je l'avais prévu. Je note toujours un coup au but."),
            ropes: T::new("Unexpected front moving in. I'll have to revise the forecast.", "Front inattendu. Je vais devoir revoir mes prévisions."),
            defeat: T::new("Clear skies for you. Mark it in the logbook: today, I was wrong.", "Ciel dégagé pour toi. Note-le au carnet : aujourd'hui, je me suis trompé."),
        },
    },
    Rival {
        id: "orla",
        name: "Orla",
        title: T::new("Quarry Foreman", "Contremaîtresse"),
        place: T::new("Emberstep Quarry, up on the ledge", "Carrière de Pas-de-Braise, en haut de la corniche"),
        colors: &[Fighting],
        deck: Prebuilt {
            name: T::new("Bedrock", "Roche-mère"),
            description: T::new("Two Stone lines up to Awaken 2 and a wall of trainers.", "Deux lignées Roc jusqu'à l'Éveil 2 et un mur de cartes Dresseur."),
            list: &[
                (BRAWLET, 4),
                (IRONBRAWL, 3),
                (SHALEPUP, 4),
                (SHALEHOUND, 3),
                (CRAGMASTIFF, 2),
                (KNOLLOP, 3),
                (KNOLLGRIM, 2),
                (KNOLLOSSUS, 1),
                (BOULDERGUT, 2),
                (LUCKY_SATCHEL, 4),
                (TRAIL_COMPASS, 3),
                (SPIKE_CHARM, 2),
                (BARK_SHIELD, 2),
                (FIELD_GUIDE, 1),
                (HEARTY_BROTH, 1),
                (SNARE_HOOK, 1),
                (FIGHTING_ENERGY, 19),
                (TWOFOLD_ENERGY, 3),
            ],
        },
        difficulty: Difficulty::Normal,
        prizes: 5,
        needs: 3,
        seal: true,
        booster: 5, // Stone
        packs: 2,
        style: Style::Aggressive,
        lines: RivalLines {
            intro: T::new("Hard hat on. We're breaking ground.", "Casque sur la tête. On attaque la roche."),
            first_ko: T::new("That's one block shifted. Plenty more in the pile.", "Un bloc de déplacé. Il en reste plein le tas."),
            ropes: T::new("Hm. You've found the fault line. Most don't.", "Hm. Tu as trouvé la faille. La plupart ne la voient pas."),
            defeat: T::new("Solid work. I'd hire you, if you could lift a crate.", "Du travail solide. Je t'embaucherais, si tu savais porter une caisse."),
        },
    },
    Rival {
        id: "lumen",
        name: "Lumen",
        title: T::new("Night Gardener", "Jardinière de nuit"),
        place: T::new("Fernhollow Trail, in the moonflower glade", "Sentier de Creuse-Fougère, dans la clairière des fleurs de lune"),
        colors: &[Psychic, Water],
        deck: Prebuilt {
            name: T::new("Moonlit Tide", "Marée de lune"),
            description: T::new("Dream Awaken 2 Hexons that put the opponent to sleep, backed by Tide.", "Des Hexons Songe d'Éveil 2 qui endorment l'adversaire, épaulés par des Hexons Marée."),
            list: &[
                (WISPUN, 4),
                (WISPELLE, 3),
                (WRAITHELLE, 2),
                (GLIMMOX, 2),
                (DOZEL, 3),
                (SOMNITAR, 2),
                (PUDDOCK, 3),
                (BROOKHORN, 2),
                (TIDEMAW, 1),
                (LUCKY_SATCHEL, 3),
                (TRAIL_COMPASS, 2),
                (MOSS_POULTICE, 2),
                (DRAIN_BRUSH, 2),
                (BARK_SHIELD, 2),
                (RECALL_HORN, 1),
                (UNWIND_SPRAY, 1),
                (BLOOM_SEED, 1),
                (PSYCHIC_ENERGY, 14),
                (WATER_ENERGY, 10),
            ],
        },
        difficulty: Difficulty::Hard,
        prizes: 6,
        needs: 5,
        seal: true,
        booster: 6, // Dream
        packs: 2,
        style: Style::BenchSniper,
        lines: RivalLines {
            intro: T::new("Shh. The moonflowers are opening. Let's not wake them.", "Chut. Les fleurs de lune s'ouvrent. Ne les réveillons pas."),
            first_ko: T::new("Sleep now. The moths will keep watch.", "Dors, maintenant. Les phalènes veilleront."),
            ropes: T::new("You walk through dreams like someone who knows the way.", "Tu traverses les rêves comme quelqu'un qui connaît le chemin."),
            defeat: T::new("Morning, already? Then go. You've woken the garden.", "Déjà le matin ? Alors va. Tu as réveillé le jardin."),
        },
    },
    Rival {
        id: "warden",
        name: "Sable",
        title: T::new("The Hex Warden", "La Gardienne des Six"),
        place: T::new("Behind the sealed door in the duel hall", "Derrière la porte scellée de la salle des duels"),
        colors: &[Water, Fighting],
        deck: Prebuilt {
            name: T::new("Sixfold Crown", "Couronne des Six"),
            description: T::new("Tide and Stone Awaken 2 Hexons, energy removal, healing and stage skips.", "Des Hexons Marée et Roc d'Éveil 2, du retrait d'énergie, des soins et des Éveils express."),
            list: &[
                (PUDDOCK, 4),
                (BROOKHORN, 3),
                (TIDEMAW, 2),
                (RILLET, 3),
                (STREAMFIN, 2),
                (GULFWYRM, 1),
                (SHELLMOTE, 2),
                (BRAWLET, 3),
                (IRONBRAWL, 2),
                (BOULDERGUT, 2),
                (LUCKY_SATCHEL, 4),
                (TRAIL_COMPASS, 3),
                (DRAIN_BRUSH, 2),
                (SNARE_HOOK, 1),
                (BLOOM_SEED, 2),
                (MOSS_POULTICE, 2),
                (WATER_ENERGY, 14),
                (FIGHTING_ENERGY, 8),
            ],
        },
        difficulty: Difficulty::Hard,
        prizes: 6,
        needs: SEALS,
        seal: false,
        booster: 2, // Wild
        packs: 3,
        style: Style::Balanced,
        lines: RivalLines {
            intro: T::new("Six seals. Let's see the hand that carried them.", "Six sceaux. Voyons la main qui les a portés."),
            first_ko: T::new("Every champion falls once in this room. Usually more.", "Tout champion tombe au moins une fois dans cette salle. Souvent plus."),
            ropes: T::new("There it is: the look they all get, just before.", "Le voilà : ce regard qu'ils ont tous, juste avant."),
            defeat: T::new("The door was always going to open for you. I just had to be sure.", "La porte allait s'ouvrir pour toi, de toute façon. Je devais juste en être sûre."),
        },
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::Ai;
    use crate::booster::BOOSTERS;
    use crate::decks::STARTER;
    use crate::{validate_deck, Config, Game};

    /// The slope test's games per rival, the most a rival may sit below the
    /// one before (seeded noise at this sample), and the largest rise.
    const SLOPE_GAMES: u64 = 40;
    const SLOPE_DIP: u64 = 10;
    const SLOPE_STEP: u64 = 30;

    fn beat(p: &mut Progress, ids: &[&str]) {
        let mut pr = Profile::new_player();
        for id in ids {
            record(rival(id).unwrap(), true, p, &mut pr);
        }
    }

    #[test]
    fn rivals_are_well_formed() {
        assert_eq!(RIVALS.iter().filter(|r| r.seal).count(), SEALS);
        for (i, r) in RIVALS.iter().enumerate() {
            assert_eq!(validate_deck(&r.deck.cards()), Ok(()), "{}", r.name);
            assert!((1..=6).contains(&r.prizes), "{}", r.name);
            assert!(r.booster < BOOSTERS.len() && r.packs > 0, "{}", r.name);
            assert!(r.id.bytes().all(|b| b.is_ascii_lowercase()), "{}", r.id);
            assert!(RIVALS[..i].iter().all(|o| o.id != r.id && o.name != r.name));
            // Gating only grows along the order, and a rival never needs its own seal.
            assert!(RIVALS[..i].iter().all(|o| o.needs <= r.needs));
            assert!(r.needs <= RIVALS[..i].iter().filter(|o| o.seal).count());
        }
        assert_eq!(RIVALS.last().unwrap().needs, SEALS);
        assert_eq!(RIVALS.iter().filter(|r| r.needs == 0).count(), 3);
    }

    #[test]
    fn first_win_earns_a_seal_packs_and_facets_rematch_facets_loss_nothing() {
        let mut p = Progress::default();
        let mut pr = Profile::new_player();
        pr.wallet = Default::default();
        let coins = p.coins();
        let r = rival("bramble").unwrap();
        assert_eq!(r.purse(&p), RIVAL_PURSE);
        let lost = record(r, false, &mut p, &mut pr);
        assert_eq!((lost.seal, lost.packs.1, pr.wallet.total()), (false, 0, 0));
        assert_eq!((lost.promo, pr.collection.count(THISTLEWICK)), (None, 0));
        assert_eq!((lost.facets, p.coins()), (0, coins));
        assert_eq!(p.last_duel(), Some(("bramble", false)));
        assert_eq!(seals(&p), 0);

        let won = record(r, true, &mut p, &mut pr);
        assert!(won.first && won.seal);
        assert_eq!(won.packs, (3, 2));
        assert_eq!(pr.wallet.packs(3), 2);
        assert_eq!(
            (won.promo, pr.collection.count(THISTLEWICK)),
            (Some(THISTLEWICK), 1)
        );
        assert_eq!((won.facets, p.coins()), (300, coins + 300));
        assert!(p.has_flag("beat_bramble"));
        assert_eq!(p.last_duel(), Some(("bramble", true)));
        assert_eq!(seals(&p), 1);

        // A rematch: ⬡ 10 a prize, no pack, no second promo.
        assert_eq!(r.purse(&p), 20);
        let again = record(r, true, &mut p, &mut pr);
        assert!(!again.first && !again.seal && again.promo.is_none());
        assert_eq!((again.packs, again.facets), ((3, 0), 20));
        assert_eq!((pr.wallet.packs(3), seals(&p)), (2, 1));
        assert_eq!(pr.collection.count(THISTLEWICK), 1, "one promo per rival");
        assert_eq!(p.coins(), coins + 320);
    }

    #[test]
    fn the_warden_pays_most_and_rematches_pay_by_prizes() {
        let p = Progress::default();
        assert_eq!(rival("warden").unwrap().purse(&p), WARDEN_PURSE);
        let mut champ = p.clone();
        beat(&mut champ, &["bramble", "lumen", "warden"]);
        let rematch: Vec<u32> = ["bramble", "lumen", "warden"]
            .map(|id| rival(id).unwrap().purse(&champ))
            .to_vec();
        assert_eq!(rematch, [20, 60, 60]);
    }

    #[test]
    fn opponents_cover_rivals_and_route_duelists() {
        let r = opponent("quill").unwrap();
        assert!(r.rival && r.style == Style::BenchSniper && r.prizes == 4);
        assert_eq!(
            r.lines.unwrap().intro.en,
            rival("quill").unwrap().lines.intro.en
        );
        let t = opponent("t_hazel").unwrap();
        assert!(!t.rival && t.lines.is_none() && t.style == Style::Balanced);
        assert_eq!((t.name, t.title.en, t.prizes), ("Hazel", "Beekeeper", 2));
        assert_eq!(validate_deck(&t.deck), Ok(()));
        assert!(opponent("nobody").is_none());

        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        let packs = pr.wallet.total();
        let rw = record_opponent("t_hazel", true, &mut p, &mut pr).unwrap();
        assert!(rw.first && !rw.seal && rw.packs.1 == 0 && rw.promo.is_none());
        assert_eq!(rw.facets, 45);
        assert_eq!(pr.wallet.total(), packs);
        let again = record_opponent("t_hazel", true, &mut p, &mut pr).unwrap();
        assert!(!again.first && again.facets == 0);
        assert!(
            record_opponent("bramble", true, &mut p, &mut pr)
                .unwrap()
                .seal
        );
        assert!(record_opponent("nobody", true, &mut p, &mut pr).is_none());
        assert_eq!(seals(&p), 1, "a route duelist is never a seal");
    }

    #[test]
    fn quick_duels_pay_facets_by_level() {
        let mut p = Progress::default();
        let coins = p.coins();
        let paid: Vec<u32> = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard]
            .map(|d| record_quick_duel(&mut p, true, d))
            .to_vec();
        assert_eq!(paid, [10, 20, 40]);
        assert_eq!(record_quick_duel(&mut p, false, Difficulty::Hard), 0);
        assert_eq!(p.coins(), coins + 70);
    }

    /// Every rival has four duel lines in both languages, and a style.
    #[test]
    fn rivals_have_their_lines_and_styles() {
        for r in &RIVALS {
            let l = &r.lines;
            for t in [l.intro, l.first_ko, l.ropes, l.defeat] {
                assert!(
                    !t.en.is_empty() && !t.fr.is_empty() && t.en != t.fr,
                    "{}",
                    r.name
                );
            }
        }
        let styles: Vec<Style> = RIVALS.iter().map(|r| r.style).collect();
        use Style::*;
        assert_eq!(
            styles,
            [
                Stall,
                Stall,
                Aggressive,
                BenchSniper,
                Aggressive,
                BenchSniper,
                Balanced
            ]
        );
    }

    #[test]
    fn old_saves_get_the_promos_of_rivals_already_beaten() {
        let p = Progress::decode("f:beat_bramble,f:beat_orla");
        let mut pr = Profile::new_player();
        assert_eq!(backfill_promos(&p, &mut pr), [THISTLEWICK, SLATEBROW]);
        assert_eq!(backfill_promos(&p, &mut pr), [], "only once");
        assert_eq!(pr.collection.count(SLATEBROW), 1);
    }

    /// Each rival gives its own promo, the Warden the last one.
    #[test]
    fn every_rival_has_its_own_promo() {
        let promos: Vec<CardId> = RIVALS.iter().map(Rival::promo).collect();
        assert_eq!(promos, PROMOS);
        assert_eq!(rival("warden").unwrap().promo(), HEXCREST);
    }

    #[test]
    fn seals_open_the_later_rivals_and_the_warden_gives_none() {
        let mut p = Progress::default();
        let open = |p: &Progress| RIVALS.iter().filter(|r| r.open(p)).count();
        assert_eq!(open(&p), 3);
        beat(&mut p, &["bramble", "marlo"]);
        assert_eq!(open(&p), 3);
        beat(&mut p, &["cinza"]);
        assert_eq!(open(&p), 5);
        beat(&mut p, &["quill", "orla"]);
        assert_eq!(open(&p), 6);
        assert!(!rival("warden").unwrap().open(&p));
        beat(&mut p, &["lumen"]);
        assert!(rival("warden").unwrap().open(&p));
        assert!(!champion(&p));
        beat(&mut p, &["warden"]);
        assert!(champion(&p));
        assert_eq!(seals(&p), SEALS);
        assert!(p.has_flag("beat_warden"));
    }

    /// Every rival's AI finishes a game against the starter deck.
    #[test]
    fn rival_decks_play_out_against_the_starter() {
        for (i, r) in RIVALS.iter().enumerate() {
            let seed = 100 + i as u64;
            let config = Config { prizes: r.prizes };
            let mut g = Game::new([STARTER.cards(), r.deck.cards()], seed, config).unwrap();
            let mut ais = [
                Ai::new(seed, Difficulty::Normal),
                Ai::new(seed ^ 1, r.difficulty),
            ];
            for _ in 0..20_000 {
                if g.winner().is_some() {
                    break;
                }
                let me = g.view(0).waiting_on.unwrap();
                let a = ais[me].choose_action(&g, me);
                g.apply(a).unwrap();
            }
            assert!(g.winner().is_some(), "{} never finished", r.name);
        }
    }

    /// A rival's win rate (percent) against the starter deck played by a
    /// Normal AI, over `games` seeded games at the rival's prizes and level.
    fn rate_vs_starter(r: &Rival, style: Style, games: u64) -> u64 {
        rate_with(r, r.prizes, r.difficulty, style, games)
    }

    fn rate_with(r: &Rival, prizes: u8, level: Difficulty, style: Style, games: u64) -> u64 {
        let mut wins = 0;
        for seed in 0..games {
            let config = Config { prizes };
            let mut g = Game::new([STARTER.cards(), r.deck.cards()], seed, config).unwrap();
            let mut ais = [
                Ai::new(seed, Difficulty::Normal),
                Ai::new(seed ^ 1, level).with_style(style),
            ];
            while g.winner().is_none() {
                let me = g.view(0).waiting_on.unwrap();
                let a = ais[me].choose_action(&g, me);
                g.apply(a).unwrap();
            }
            wins += u64::from(g.winner() == Some(1));
        }
        wins * 100 / games
    }

    /// Tuning aid: each rival's win rate against the starter deck (the
    /// starter played by a Normal AI), played Balanced and with its style.
    /// `cargo test --release -p sixfold-engine rival_strength -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn rival_strength_against_the_starter() {
        for r in &RIVALS {
            println!(
                "{:8} balanced {:>3}%  {:?} {:>3}% ({} prizes)",
                r.name,
                rate_vs_starter(r, Style::Balanced, 60),
                r.style,
                rate_vs_starter(r, r.style, 60),
                r.prizes
            );
        }
    }

    /// The difficulty slope: against the starter deck, each rival as it
    /// plays wins a little more often than the one before, in campaign
    /// order (`docs/BALANCE.md`, "Difficulty slope"). 40 seeded games each.
    #[test]
    fn rivals_rise_gently_against_the_starter() {
        let rates: Vec<u64> = RIVALS
            .iter()
            .map(|r| rate_vs_starter(r, r.style, SLOPE_GAMES))
            .collect();
        eprintln!("rivals against the starter, in campaign order: {rates:?}");
        assert!(rates[0] <= 30, "the first rival wins {}%", rates[0]);
        assert!(
            rates[RIVALS.len() - 1] >= 75,
            "the Warden wins {}%",
            rates[RIVALS.len() - 1]
        );
        for w in rates.windows(2) {
            assert!(
                w[1] + SLOPE_DIP >= w[0] && w[1] <= w[0] + SLOPE_STEP,
                "a step from {}% to {}%: {rates:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn the_warden_is_the_boss_and_rivals_bring_their_colour() {
        assert_eq!(stakes(Some("warden")), Stakes::Boss);
        assert_eq!(stakes(Some("quill")), Stakes::Rival(Color::Lightning));
        assert_eq!(stakes(Some("orla")), Stakes::Rival(Color::Fighting));
        assert_eq!(stakes(Some(trainers::TRAINERS[0].id)), Stakes::Ordinary);
        assert_eq!(stakes(None), Stakes::Ordinary);
        // Every rival but the Warden plays a rival's theme.
        let bosses = RIVALS.iter().filter(|r| stakes(Some(r.id)) == Stakes::Boss);
        assert_eq!(bosses.count(), 1);
    }
}
