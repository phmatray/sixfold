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

/// A save from before the keepers' threshold that had already met a keeper:
/// every keeper accepts it ([`Rival::earned`]), set once by
/// [`Progress::decode`].
pub const VOUCHED: &str = "vouched";
/// Every progress from this build on holds it: a save without it predates
/// the threshold, and is vouched for on load if it already met a keeper.
pub const KEEPERS: &str = "keepers";

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
    /// The challenge panel, when they won't duel yet: too few of their
    /// region's duelists bested ([`Rival::earned`]).
    pub refuse: T,
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
    /// Their region, as a sentence names it ("the Saltreach"); empty for the
    /// Warden.
    pub region: T,
    /// The route duelists of their region (its maps and its town's
    /// interiors; `engine/tests/world_data.rs` holds the list to the maps).
    pub duelists: &'static [&'static str],
    /// How many of them must be bested before they accept a duel (never all).
    pub earn: usize,
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

    /// Their region's duelists beaten so far (rook ids never count).
    pub fn bested(&self, p: &Progress) -> usize {
        self.duelists
            .iter()
            .filter(|id| p.has_flag(&format!("beat_{id}")))
            .count()
    }

    /// Enough of their region's duelists bested for them to duel: always for
    /// the Warden, once beaten, and for a save from before the threshold
    /// ([`VOUCHED`]).
    pub fn earned(&self, p: &Progress) -> bool {
        !self.seal || self.beaten(p) || p.has_flag(VOUCHED) || self.bested(p) >= self.earn
    }

    /// They accept a duel now: the seals are met and the duel is earned.
    pub fn accepts(&self, p: &Progress) -> bool {
        self.open(p) && self.earned(p)
    }

    /// Their region's duelists still to best before they accept (0 once earned).
    pub fn to_best(&self, p: &Progress) -> usize {
        if self.earned(p) {
            0
        } else {
            self.earn.saturating_sub(self.bested(p))
        }
    }

    /// The roster's count while the duel is still to earn, naming the region:
    /// "Saltreach duelists bested: 2 of 3".
    pub fn bested_line(&self, p: &Progress) -> String {
        let (n, of) = (self.bested(p), self.earn);
        let region = self.region.en.trim_start_matches("the ");
        crate::tr(
            format!("{region} duelists bested: {n}\u{a0}of\u{a0}{of}"),
            format!(
                "Duellistes {} battus\u{a0}: {n}\u{a0}sur\u{a0}{of}",
                self.region_de()
            ),
        )
    }

    /// The maps that still hold a duelist of their region not yet beaten,
    /// none once earned.
    pub fn waiting(&self, p: &Progress) -> Vec<&'static Ground> {
        if self.earned(p) {
            return vec![];
        }
        GROUNDS
            .iter()
            .filter(|g| {
                g.duelists
                    .iter()
                    .any(|id| self.duelists.contains(id) && !p.has_flag(&format!("beat_{id}")))
            })
            .collect()
    }

    /// Where the duelists still to best are, as a sentence: "Some still wait
    /// on the Bellflats and in Gullhaven." None once earned.
    pub fn waiting_line(&self, p: &Progress) -> Option<String> {
        let at: Vec<&str> = self.waiting(p).iter().map(|g| g.at.get()).collect();
        let (last, rest) = at.split_last()?;
        let list = if rest.is_empty() {
            last.to_string()
        } else {
            // "or", as the Journal says it (#155): any of them will do.
            format!("{} {} {last}", rest.join(", "), crate::tr("or", "ou"))
        };
        Some(crate::tr(
            format!("Some still wait {list}."),
            format!("Il en reste {list}."),
        ))
    }

    /// The region after French "de": "de la Côte Salée", "du Bois-Profond",
    /// "des Braisières".
    pub fn region_de(&self) -> String {
        let r = self.region.fr;
        if let Some(rest) = r.strip_prefix("les ") {
            format!("des {rest}")
        } else if let Some(rest) = r.strip_prefix("le ") {
            format!("du {rest}")
        } else {
            format!("de {r}")
        }
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

/// A map that holds route duelists of a keeper's region, and where that is,
/// as a sentence says it ("on the Bellflats", "sur les Grèves de la
/// Cloche"). `engine/tests/world_data.rs` holds it to the maps and their titles.
#[derive(Debug)]
pub struct Ground {
    pub map: &'static str,
    pub at: T,
    pub duelists: &'static [&'static str],
}

/// Every map with a keeper's region's duelists, region by region.
#[rustfmt::skip]
pub static GROUNDS: [Ground; 20] = [
    Ground { map: "elder_ring", at: T::new("in the Elder Ring", "au Cercle des Anciens"), duelists: &["t_fenna", "t_heath", "t_rue"] },
    Ground { map: "rootholm", at: T::new("in Rootholm", "à Racinemont"), duelists: &["t_jory", "t_linden"] },
    Ground { map: "woodland", at: T::new("on Fernhollow Trail", "sur le Sentier de Creuse-Fougère"), duelists: &["t_bryn", "t_hazel", "t_ossian", "t_sorrel", "t_vesper"] },
    Ground { map: "bellflats", at: T::new("on the Bellflats", "sur les Grèves de la Cloche"), duelists: &["t_finch", "t_harrow", "t_mirela", "t_liesl"] },
    Ground { map: "gullhaven", at: T::new("in Gullhaven", "à Havre-aux-Mouettes"), duelists: &["t_dorrit", "t_brann"] },
    Ground { map: "seaside", at: T::new("on the Saltwind Boardwalk", "sur la Promenade de Ventsalé"), duelists: &["t_pell", "t_nell", "t_skye", "t_morrow"] },
    Ground { map: "cinderwick", at: T::new("in Cinderwick", "à Mèche-Cendre"), duelists: &["t_basil", "t_tavish"] },
    Ground { map: "kilnmouth", at: T::new("in the Kilnmouth", "dans la Gueule-du-Four"), duelists: &["t_carys", "t_dunstan", "t_vela"] },
    Ground { map: "quarry", at: T::new("in Emberstep Quarry", "à la Carrière de Pas-de-Braise"), duelists: &["t_tamsin", "t_rusk", "t_cole", "t_marigold"] },
    Ground { map: "downs", at: T::new("on the Thrumming Downs", "sur les Landes Vibrantes"), duelists: &["t_odo", "t_hew", "t_maren", "t_tull"] },
    Ground { map: "rodspire", at: T::new("on the Rodspire", "sur l'Aiguille-Foudre"), duelists: &["t_corra", "t_ves"] },
    Ground { map: "stormwatch", at: T::new("in Stormwatch", "à Guette-Orage"), duelists: &["t_greer"] },
    Ground { map: "anvilstead", at: T::new("in Anvilstead", "à Bourg-l'Enclume"), duelists: &["t_dov"] },
    Ground { map: "climb_school", at: T::new("at the Anvilstead climbing school", "à l'école d'escalade de Bourg-l'Enclume"), duelists: &["t_ida", "t_garth"] },
    Ground { map: "bastion", at: T::new("at the Bastion", "au Bastion"), duelists: &["t_petra", "t_morwen"] },
    Ground { map: "screepass", at: T::new("in Scree Pass", "au Col des Éboulis"), duelists: &["t_mica", "t_ulric"] },
    Ground { map: "undermine", at: T::new("in the Undermine", "dans les Galeries Basses"), duelists: &["t_jet"] },
    Ground { map: "hushlane", at: T::new("on Hushwillow Lane", "dans l'Allée des Saules-Murmures"), duelists: &["t_iver", "t_luce"] },
    Ground { map: "lanternmere", at: T::new("in Lanternmere", "à Lac-aux-Lanternes"), duelists: &["t_yara", "t_ambrose", "t_clem"] },
    Ground { map: "moonwell", at: T::new("in the Moonwell Garden", "au Jardin du Puits-de-Lune"), duelists: &["t_maud", "t_noor"] },
];

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
    /// One of Rook's story duels: his own theme.
    Story,
}

/// The stakes of a duel against `id`, a rival's or route duelist's id.
pub fn stakes(id: Option<&str>) -> Stakes {
    if id.and_then(trainer).is_some_and(|t| t.story()) {
        return Stakes::Story;
    }
    match id.and_then(rival) {
        Some(r) if !r.seal => Stakes::Boss,
        Some(r) => r
            .colors
            .first()
            .map_or(Stakes::Ordinary, |&c| Stakes::Rival(c)),
        None => Stakes::Ordinary,
    }
}

/// A duel with a seal (or the champion's title) at stake.
#[derive(Debug, Clone, Copy)]
pub struct SealDuel {
    pub rival: &'static Rival,
    /// The Warden: the champion's title rather than a seal.
    pub grand: bool,
}

/// Some while `id` is a seal keeper not yet beaten (`grand: false`) or the
/// Warden before the first win (`grand: true`); None for rematches, route
/// duelists, Rook and post-game duels.
pub fn seal_duel(id: Option<&str>, p: &Progress) -> Option<SealDuel> {
    let rival = id.and_then(rival).filter(|r| !r.beaten(p))?;
    Some(SealDuel {
        rival,
        grand: !rival.seal,
    })
}

impl SealDuel {
    /// The title card's heading, written out per region so the French
    /// articles are right.
    pub fn heading(&self) -> T {
        if self.grand {
            return T::new("The Champion's Duel", "Le Duel du champion");
        }
        match self.rival.id {
            "bramble" => T::new("Seal of the Grove", "Sceau du Bosquet"),
            "marlo" => T::new("Seal of the Tide", "Sceau de la Marée"),
            "cinza" => T::new("Seal of the Ember", "Sceau de la Braise"),
            "quill" => T::new("Seal of the Storm", "Sceau de l'Orage"),
            "orla" => T::new("Seal of the Stone", "Sceau du Roc"),
            "lumen" => T::new("Seal of the Dream", "Sceau du Songe"),
            _ => T::new("A Seal at Stake", "Un sceau en jeu"),
        }
    }
}

/// The next rival to duel: the first not yet beaten who accepts, else the
/// first whose seals are met (their region's duelists still to best).
pub fn next_rival(p: &Progress) -> Option<&'static Rival> {
    let unbeaten = || RIVALS.iter().filter(|r| !r.beaten(p));
    unbeaten()
        .find(|r| r.accepts(p))
        .or_else(|| unbeaten().find(|r| r.open(p)))
}

/// Seals earned so far.
pub fn seals(p: &Progress) -> usize {
    RIVALS.iter().filter(|r| r.seal && r.beaten(p)).count()
}

/// The seal stones' lit faces (#153): the first colour of every seal keeper
/// beaten, but for a first win whose stage victory (`victory_<id>`, which
/// sets `sealed_<id>`) is still to play: that face lights in the scene.
pub fn lit_faces(p: &Progress) -> Vec<Color> {
    RIVALS
        .iter()
        .filter(|r| r.seal && r.beaten(p))
        .filter(|r| p.last_duel() != Some((r.id, true)) || p.has_flag(&format!("sealed_{}", r.id)))
        .filter_map(|r| r.colors.first().copied())
        .collect()
}

/// The Warden has been beaten: the player is the champion.
pub fn champion(p: &Progress) -> bool {
    RIVALS.iter().any(|r| !r.seal && r.beaten(p))
}

/// The champion's title, once the Warden falls: "Champion", and "Champion of
/// the Seventh Face" once Rook's story is done (`flag:rook_4`, §8.2).
pub fn title(p: &Progress) -> Option<T> {
    champion(p).then(|| {
        if p.has_flag("rook_4") {
            T::new(
                "Champion of the Seventh Face",
                "Champion de la septième face",
            )
        } else {
            T::new("Champion", "Champion")
        }
    })
}

/// What Rook says in his duels (all four).
pub static ROOK_LINES: RivalLines = RivalLines {
    intro: T::new(
        "No colour, no excuses. Let's see what yours are worth.",
        "Pas de couleur, pas d'excuses. Voyons ce que valent les tiennes.",
    ),
    first_ko: T::new(
        "That's one more tally mark. For me, this time.",
        "Un trait de plus sur ma manche. Pour moi, cette fois.",
    ),
    ropes: T::new(
        "Go on, then. Finish it. I'll still be here.",
        "Vas-y. Termine. Je serai encore là.",
    ),
    defeat: T::new(
        "Blank again. Fine. I'll carve it myself one day.",
        "Encore blanc. Bon. Je la graverai moi-même, un jour.",
    ),
    // Rook never refuses: his scenes start his duels.
    refuse: T::new("Not now.", "Pas maintenant."),
};

/// The duel lines of a rival or of Rook, by id (route duelists have none).
pub fn lines(id: &str) -> Option<&'static RivalLines> {
    rival(id)
        .map(|r| &r.lines)
        .or_else(|| trainer(id).filter(|t| t.story()).map(|_| &ROOK_LINES))
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
    // A rematch won: the seal was won on stage already (or, on a save from
    // before the stage, long ago), so `victory_<id>` never plays for it.
    if won && !first && r.seal {
        p.apply(&format!("flag:sealed_{}", r.id))
            .expect("rival ids are valid flag names");
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
        lines: lines(t.id),
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

/// Facets a lost duel pays for showing up, and again for each prize the
/// player took: every loss still moves the player on (`docs/BALANCE.md`,
/// consolation).
pub const CONSOLATION_PER_PRIZE: u32 = 5;

/// Pay the consolation for a lost duel in which the player took `prizes`
/// prizes; returns the Facets earned (never 0).
pub fn record_consolation(p: &mut Progress, prizes: usize) -> u32 {
    let n = CONSOLATION_PER_PRIZE.saturating_mul(prizes as u32 + 1);
    p.earn(n);
    n
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
        place: T::new("The Elder Ring, by the seal stone at its heart", "Le Cercle des Anciens, près de la pierre de sceau, en son cœur"),
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
        region: T::new("the Deepgrove", "le Bois-Profond"),
        duelists: &["t_fenna", "t_heath", "t_rue", "t_jory", "t_linden", "t_bryn", "t_hazel", "t_ossian", "t_sorrel", "t_vesper"],
        earn: 3,
        booster: 3, // Grove
        packs: 2,
        style: Style::Stall,
        lines: RivalLines {
            intro: T::new("Easy now. A hedge grows one leaf at a time, and so will this duel.", "Doucement. Une haie pousse une feuille à la fois, et ce duel aussi."),
            first_ko: T::new("Pruned. It'll grow back stronger. Yours might not.", "Taillé. Ça repoussera plus fort. Le tien, je n'en suis pas sûr."),
            ropes: T::new("You've found the gap in my hedge. Mind the thorns on the way through.", "Tu as trouvé le trou dans ma haie. Attention aux épines en passant."),
            defeat: T::new("Well grown. Go on, I've some pride to mulch.", "Bien poussé. Va, j'ai de la fierté à pailler."),
            // A short line under the keeper's own words (#155): the panel never says it all again.
            refuse: T::new("Come back when the roots have heard of you.", "Reviens quand les racines auront entendu parler de toi."),
        },
    },
    Rival {
        id: "marlo",
        name: "Marlo",
        title: T::new("Tide Reader", "Liseur de marées"),
        place: T::new("The Bellflats, by the seal stone on the bell-tower islet", "Les Grèves de la Cloche, près de la pierre de sceau sur l'îlot du clocher"),
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
        region: T::new("the Saltreach", "la Côte Salée"),
        duelists: &["t_finch", "t_harrow", "t_mirela", "t_liesl", "t_dorrit", "t_brann", "t_pell", "t_nell", "t_skye", "t_morrow"],
        earn: 3,
        booster: 1, // Tide
        packs: 2,
        style: Style::Stall,
        lines: RivalLines {
            intro: T::new("The tide's turning. Question is which of us it carries out.", "La marée tourne. Reste à savoir lequel de nous deux elle emporte."),
            first_ko: T::new("Out with the ebb. It always comes back in, you know. Mine does.", "Parti avec le reflux. Ça revient toujours, tu sais. Chez moi, en tout cas."),
            ropes: T::new("Slack water. Nothing moves, except you, apparently.", "L'étale. Plus rien ne bouge, sauf toi, visiblement."),
            defeat: T::new("High-water mark. I'll chalk it on the piling.", "Laisse de haute mer. Je la marque à la craie sur le pilotis."),
            refuse: T::new("Come back with the flats behind you.", "Reviens quand l'estran parlera pour toi."),
        },
    },
    Rival {
        id: "cinza",
        name: "Cinza",
        title: T::new("Kiln Runner", "Coureuse du four"),
        place: T::new("The Kilnmouth, by the seal stone on the crater floor", "La Gueule-du-Four, près de la pierre de sceau au fond du cratère"),
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
        region: T::new("the Emberlands", "les Braisières"),
        duelists: &["t_basil", "t_tavish", "t_carys", "t_dunstan", "t_vela", "t_tamsin", "t_rusk", "t_cole", "t_marigold"],
        earn: 3,
        booster: 0, // Ember
        packs: 2,
        style: Style::Aggressive,
        lines: RivalLines {
            intro: T::new("No warm-up, no waiting. The kiln's already hot!", "Pas d'échauffement, pas d'attente. Le four est déjà chaud\u{202f}!"),
            first_ko: T::new("Ha! Out of the fire and straight into the ash!", "Ha\u{202f}! Sorti du feu, direct dans la cendre\u{202f}!"),
            ropes: T::new("Okay, okay, I'm stoking, I'm stoking!", "D'accord, d'accord, je rallume, je rallume\u{202f}!"),
            defeat: T::new("Burnt out. Rematch the second I've cooled down. So, now?", "Grillée. Revanche dès que j'ai refroidi. Donc, maintenant\u{202f}?"),
            refuse: T::new("Come back warmed up. I'll keep the kiln hot.", "Reviens après l'échauffement. Je garde le four chaud."),
        },
    },
    Rival {
        id: "quill",
        name: "Quill",
        title: T::new("Weather Watcher", "Vigie du ciel"),
        place: T::new("The Rodspire, by the seal stone on top of the stack", "L'Aiguille-Foudre, près de la pierre de sceau en haut du rocher"),
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
        region: T::new("the Galewold", "les Hautes-Rafales"),
        duelists: &["t_odo", "t_hew", "t_maren", "t_tull", "t_corra", "t_ves", "t_greer"],
        earn: 4,
        booster: 4, // Storm
        packs: 2,
        style: Style::BenchSniper,
        lines: RivalLines {
            intro: T::new("Barometer's falling. Expect scattered strikes, mostly on your bench.", "Le baromètre chute. Prévois des éclairs épars, surtout sur ton Banc."),
            first_ko: T::new("Called it. I always log a direct hit.", "Je l'avais prévu. Je note toujours un coup au but."),
            ropes: T::new("Unexpected front moving in. I'll have to revise the forecast.", "Front inattendu. Je vais devoir revoir mes prévisions."),
            defeat: T::new("Clear skies for you. Mark it in the logbook: today, I was wrong.", "Ciel dégagé pour toi. Note-le au carnet\u{a0}: aujourd'hui, je me suis trompé."),
            refuse: T::new("Come back when there's something to write down.", "Reviens quand il y aura quelque chose à noter."),
        },
    },
    Rival {
        id: "orla",
        name: "Orla",
        title: T::new("Quarry Foreman", "Contremaîtresse"),
        place: T::new("The Bastion, by the seal stone on the top terrace", "Le Bastion, près de la pierre de sceau sur la haute terrasse"),
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
        region: T::new("the Stonebacks", "les Échines de Pierre"),
        duelists: &["t_dov", "t_petra", "t_morwen", "t_mica", "t_ulric", "t_jet", "t_ida", "t_garth"],
        earn: 3,
        booster: 5, // Stone
        packs: 2,
        style: Style::Aggressive,
        lines: RivalLines {
            intro: T::new("Hard hat on. We're breaking ground.", "Casque sur la tête. On attaque la roche."),
            first_ko: T::new("That's one block shifted. Plenty more in the pile.", "Un bloc de déplacé. Il en reste plein le tas."),
            ropes: T::new("Hm. You've found the fault line. Most don't.", "Hm. Tu as trouvé la faille. La plupart ne la voient pas."),
            defeat: T::new("Solid work. I'd hire you, if you could lift a crate.", "Du travail solide. Je t'embaucherais, si tu savais porter une caisse."),
            refuse: T::new("Come back once you've put in your shifts.", "Reviens quand tu auras fait tes quarts."),
        },
    },
    Rival {
        id: "lumen",
        name: "Lumen",
        title: T::new("Night Gardener", "Jardinière de nuit"),
        place: T::new("The Moonwell Garden, between the moonwell and the seal stone", "Le Jardin du Puits-de-Lune, entre le puits et la pierre de sceau"),
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
        region: T::new("the Hushlands", "les Terres du Murmure"),
        duelists: &["t_iver", "t_luce", "t_yara", "t_ambrose", "t_clem", "t_maud", "t_noor"],
        earn: 3,
        booster: 6, // Dream
        packs: 2,
        style: Style::BenchSniper,
        lines: RivalLines {
            intro: T::new("Shh. The moonflowers are opening. Let's not wake them.", "Chut. Les fleurs de lune s'ouvrent. Ne les réveillons pas."),
            first_ko: T::new("Sleep now. The moths will keep watch.", "Dors, maintenant. Les phalènes veilleront."),
            ropes: T::new("You walk through dreams like someone who knows the way.", "Tu traverses les rêves comme quelqu'un qui connaît le chemin."),
            defeat: T::new("Morning, already? Then go. You've woken the garden.", "Déjà le matin\u{202f}? Alors va. Tu as réveillé le jardin."),
            refuse: T::new("Come back when the garden can say your name.", "Reviens quand le jardin saura dire ton nom."),
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
        region: T::new("", ""),
        duelists: &[],
        earn: 0,
        booster: 2, // Wild
        packs: 3,
        style: Style::Balanced,
        lines: RivalLines {
            intro: T::new("Six seals. Let's see the hand that carried them.", "Six sceaux. Voyons la main qui les a portés."),
            first_ko: T::new("Every champion falls once in this room. Usually more.", "Tout champion tombe au moins une fois dans cette salle. Souvent plus."),
            ropes: T::new("There it is: the look they all get, just before.", "Le voilà\u{a0}: ce regard qu'ils ont tous, juste avant."),
            defeat: T::new("The door was always going to open for you. I just had to be sure.", "La porte allait s'ouvrir pour toi, de toute façon. Je devais juste en être sûre."),
            refuse: T::new("Six seals first. Then we'll see.", "Six sceaux d'abord. Ensuite, on verra."),
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

    #[test]
    fn a_loss_pays_a_little_and_more_per_prize_taken() {
        let mut p = Progress::default();
        let start = p.coins();
        assert_eq!(record_consolation(&mut p, 0), CONSOLATION_PER_PRIZE);
        assert_eq!(record_consolation(&mut p, 3), 4 * CONSOLATION_PER_PRIZE);
        assert_eq!(p.coins(), start + 5 * CONSOLATION_PER_PRIZE);
    }

    fn beat(p: &mut Progress, ids: &[&str]) {
        let mut pr = Profile::new_player();
        for id in ids {
            record(rival(id).unwrap(), true, p, &mut pr);
        }
    }

    #[test]
    fn a_seal_stone_lights_the_faces_of_the_seals_held() {
        let mut p = Progress::default();
        assert!(lit_faces(&p).is_empty());
        // Marlo's first win: the face waits dark for the stage, which lights it.
        beat(&mut p, &["marlo"]);
        assert!(lit_faces(&p).is_empty(), "the victory scene lights it");
        p.apply("flag:sealed_marlo").unwrap();
        assert_eq!(lit_faces(&p), [Color::Water]);
        // A save from before the stage (the result settled, no `sealed_`) keeps it lit,
        // and so does a rematch won; the Warden has no face.
        let mut old = Progress::default();
        beat(&mut old, &["marlo", "bramble"]);
        old.settle();
        assert_eq!(lit_faces(&old), [Color::Grass, Color::Water]);
        // Its rematch won is no stage victory: it marks the seal as staged.
        beat(&mut old, &["marlo"]);
        assert!(old.check("won:marlo") && old.has_flag("sealed_marlo"));
        assert_eq!(lit_faces(&old), [Color::Grass, Color::Water]);
        beat(&mut p, &["marlo", "warden"]);
        assert_eq!(lit_faces(&p), [Color::Water]);
    }

    #[test]
    fn a_seal_is_at_stake_until_its_keeper_falls() {
        let mut p = Progress::default();
        let s = seal_duel(Some("marlo"), &p).expect("Marlo's seal is at stake");
        assert_eq!((s.rival.id, s.grand), ("marlo", false));
        assert_eq!(s.heading(), T::new("Seal of the Tide", "Sceau de la Marée"));
        beat(&mut p, &["marlo"]);
        assert!(seal_duel(Some("marlo"), &p).is_none(), "a rematch");

        let h = seal_duel(Some("bramble"), &p).unwrap().heading();
        assert_eq!(h, T::new("Seal of the Grove", "Sceau du Bosquet"));

        let w = seal_duel(Some("warden"), &p).expect("the title is at stake");
        assert!(w.grand);
        assert_eq!(
            w.heading(),
            T::new("The Champion's Duel", "Le Duel du champion")
        );
        beat(&mut p, &["warden"]);
        assert!(seal_duel(Some("warden"), &p).is_none());

        for id in [Some("t_pell"), Some("rook_1"), None] {
            assert!(seal_duel(id, &Progress::default()).is_none(), "{id:?}");
        }
        // Every keeper has its own heading, written out in both languages
        // (never the fallback).
        let heading = |id| seal_duel(Some(id), &Progress::default()).unwrap().heading();
        for r in RIVALS.iter().filter(|r| r.seal) {
            let h = heading(r.id);
            assert!(h.en.starts_with("Seal of the "), "{}", r.id);
            assert!(h.fr.starts_with("Sceau d"), "{}", r.id);
            assert!(RIVALS.iter().all(|o| o.id == r.id || heading(o.id) != h));
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
            for t in [l.intro, l.first_ko, l.ropes, l.defeat, l.refuse] {
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

    /// §8: Rook is a story duel, never a rival, a seal or the champion; his
    /// scenes move on whether the player wins or loses (the scene sets
    /// `flag:rook_<n>` before the duel, and the next scene waits only for
    /// that flag and the seals), and once his story is done the champion's
    /// title says so.
    #[test]
    fn rook_moves_the_story_on_and_never_counts_as_a_seal_or_the_champion() {
        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        for n in 1..=4 {
            let id = format!("rook_{n}");
            assert!(rival(&id).is_none() && !RIVALS.iter().any(|r| r.id == id));
            assert_eq!(stakes(Some(&id)), Stakes::Story);
            let o = opponent(&id).unwrap();
            assert!(!o.rival && o.name == "Rook" && o.lines.is_some());
        }
        // Rook's first scene, then a lost duel: the story still moves on.
        p.apply_all(&["flag:rook_1", "duel:rook_1"]).unwrap();
        let lost = record_opponent("rook_1", false, &mut p, &mut pr).unwrap();
        assert!(!lost.won && lost.facets == 0 && !lost.seal);
        assert!(p.check("lost:rook_1") && !p.check("beat:rook_1"));
        beat(&mut p, &["bramble", "marlo", "cinza"]);
        let next = "seals:3 & flag:rook_1 & !flag:rook_2";
        assert!(p.check(next), "a loss never blocks the next scene");
        // Won, the next one pays its purse and is beaten.
        p.apply_all(&["flag:rook_2", "duel:rook_2"]).unwrap();
        let won = record_opponent("rook_2", true, &mut p, &mut pr).unwrap();
        assert!(won.won && won.first && won.facets == 150 && !won.seal && won.packs.1 == 0);
        assert!(p.check("beat:rook_2") && !p.check(next));
        // Every Rook duel won: still three seals, no champion.
        for n in 3..=4 {
            record_opponent(&format!("rook_{n}"), true, &mut p, &mut pr).unwrap();
        }
        assert_eq!((seals(&p), champion(&p), title(&p)), (3, false, None));
        beat(&mut p, &["quill", "orla", "lumen", "warden"]);
        assert_eq!(title(&p).map(|t| t.en), Some("Champion"));
        p.apply("flag:rook_4").unwrap();
        assert_eq!(
            title(&p).map(|t| t.en),
            Some("Champion of the Seventh Face")
        );
        assert_eq!(
            title(&p).map(|t| t.fr),
            Some("Champion de la septième face")
        );
    }

    /// #150: a keeper accepts once enough of their region's duelists are
    /// bested; the Warden and a vouched-for save never wait.
    #[test]
    fn a_keeper_accepts_once_enough_of_their_duelists_are_bested() {
        let marlo = rival("marlo").unwrap();
        let mut p = Progress::default();
        assert_eq!(marlo.bested(&p), 0);
        assert!(marlo.open(&p) && !marlo.earned(&p) && !marlo.accepts(&p));
        // Another region's duelists and Rook don't count.
        p.apply_all(&["flag:beat_t_fenna", "flag:beat_t_tobin", "flag:beat_rook_1"])
            .unwrap();
        assert_eq!(marlo.bested(&p), 0);
        p.apply_all(&["flag:beat_t_pell", "flag:beat_t_nell"])
            .unwrap();
        assert_eq!(marlo.bested(&p), 2);
        assert!(!marlo.accepts(&p));
        p.apply("flag:beat_t_skye").unwrap();
        assert!(marlo.earned(&p) && marlo.accepts(&p));
        // The Warden is always earned (the six seals gate her).
        assert!(rival("warden").unwrap().earned(&Progress::default()));
        // A vouched-for save has earned every keeper.
        let mut v = Progress::default();
        v.apply(&format!("flag:{VOUCHED}")).unwrap();
        assert!(RIVALS.iter().all(|r| r.earned(&v)));
        // A keeper once beaten stays earned.
        let mut b = Progress::default();
        beat(&mut b, &["bramble"]);
        assert!(rival("bramble").unwrap().earned(&b));
        assert_eq!(marlo.region_de(), "de la Côte Salée");
        assert_eq!(rival("bramble").unwrap().region_de(), "du Bois-Profond");
        assert_eq!(rival("cinza").unwrap().region_de(), "des Braisières");
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
