//! The shops: Facets in, packs and cards out (`docs/WORLD_DESIGN.md` §3).
//!
//! - Wynne's counter in the duel hall sells packs ([`buy_pack`]).
//! - Perrin's Swap Shop stocks six singles a day ([`daily_stock`],
//!   [`buy_single`]) and buys cards back for a tenth of their craft price
//!   ([`sell`], [`sell_extras`]); never the last copy, so a card once owned
//!   stays craftable.
//! - Gus at the Press makes one more copy of a card ([`craft`]), or any
//!   non-promo card for a Press voucher ([`craft_with_voucher`]).
//! - Each region's inn has a counter (1.3, §7.4): its themed pack, the Wild
//!   pack, and four fixed singles of its colours, one of them a Rare
//!   ([`inn_stock`], [`buy_inn_single`]).
//!
//! Promos and basic energy are never stocked, sold or crafted. Every function
//! changes nothing when it fails.

use std::fmt;

use crate::booster::{BOOSTERS, WILD};
use crate::campaign::RIVALS;
use crate::card::{card, CardId, Rarity};
use crate::deck::MAX_COPIES;
use crate::lang::tr;
use crate::overworld::Progress;
use crate::profile::{Profile, STARTING_PACKS};
use crate::rng::Rng;
use crate::set::{self, is_promo};

pub const THEMED_PACK_PRICE: u32 = 150;
pub const WILD_PACK_PRICE: u32 = 200;
/// By tier: Common, Uncommon, Rare.
pub const SINGLE_PRICE: [u32; 3] = [20, 60, 240];
/// Crafting a card the collection holds a copy of.
pub const CRAFT_OWNED_PRICE: [u32; 3] = [40, 120, 400];
/// Crafting a card never owned: Rares must be pulled (or use a voucher).
pub const CRAFT_NEW_PRICE: [Option<u32>; 3] = [Some(80), Some(240), None];
pub const SELL_PRICE: [u32; 3] = [3, 10, 40];
/// The daily stock, per tier, in this order.
pub const STOCK: [usize; 3] = [2, 3, 1];
pub const STOCK_SIZE: usize = 6;
/// The item that pays for one craft of any non-promo card.
pub const VOUCHER: &str = "press_voucher";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopError {
    /// This many Facets short.
    Short(u32),
    /// Not on sale (yet), or never: a promo, basic energy, an unknown slot.
    NotForSale,
    /// That stock slot was bought today.
    SoldOut,
    /// The last copy, or one a deck uses.
    Keep,
    NoVoucher,
}

impl fmt::Display for ShopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ShopError::Short(n) => f.write_str(&tr(
                format!("Not enough Facets: ⬡ {n} short."),
                format!("Pas assez de Facettes\u{a0}: il en manque ⬡ {n}."),
            )),
            ShopError::NotForSale => f.write_str(tr("Not for sale.", "Pas à vendre.")),
            ShopError::SoldOut => {
                f.write_str(tr("Sold out for today.", "Épuisé pour aujourd'hui."))
            }
            ShopError::Keep => f.write_str(tr(
                "You keep your last copy, and the copies your decks use.",
                "Vous gardez votre dernier exemplaire, et ceux qu'utilisent vos decks.",
            )),
            ShopError::NoVoucher => f.write_str(tr(
                "You have no Press voucher.",
                "Vous n'avez pas de bon de l'Imprimerie.",
            )),
        }
    }
}

/// A card's price tier (0 Common, 1 Uncommon, 2 Rare); None for basic
/// energy and promos.
pub fn tier(id: CardId) -> Option<usize> {
    if is_promo(id) {
        return None;
    }
    match card(id).rarity {
        Rarity::Common => Some(0),
        Rarity::Uncommon => Some(1),
        Rarity::Rare => Some(2),
        Rarity::Energy => None,
    }
}

fn pay(p: &mut Progress, n: u32) -> Result<(), ShopError> {
    let held = p.coins();
    p.spend(n)
        .then_some(())
        .ok_or(ShopError::Short(n.saturating_sub(held)))
}

/// A pack's price, while it is on sale: the starting packs (Ember, Tide,
/// Wild) always, another themed pack once the rival who gives it is beaten.
pub fn pack_price(b: usize, p: &Progress) -> Option<u32> {
    let open = STARTING_PACKS.get(b).is_some_and(|&n| n > 0)
        || RIVALS.iter().any(|r| r.booster == b && r.beaten(p));
    let price = if b == WILD {
        WILD_PACK_PRICE
    } else {
        THEMED_PACK_PRICE
    };
    (b < BOOSTERS.len() && open).then_some(price)
}

/// Buy one pack of booster `b` into the wallet.
pub fn buy_pack(p: &mut Progress, profile: &mut Profile, b: usize) -> Result<(), ShopError> {
    pay(p, pack_price(b, p).ok_or(ShopError::NotForSale)?)?;
    profile.wallet.grant(b, 1);
    Ok(())
}

/// A region's inn counter (§7.4): the inn's interior map, its region's
/// booster (the first word of the booster's name) and four singles, Commons
/// first, one Rare last.
#[derive(Debug)]
pub struct InnStock {
    pub map: &'static str,
    pub booster: &'static str,
    pub singles: [CardId; 4],
}

impl InnStock {
    /// The region's booster, by index in [`BOOSTERS`].
    pub fn booster(&self) -> usize {
        BOOSTERS
            .iter()
            .position(|b| b.name.en.split(' ').next() == Some(self.booster))
            .expect("an inn names a booster")
    }
}

/// The six regional inns' counters (§10, "Inn stock").
pub static INNS: [InnStock; 6] = [
    InnStock {
        map: "inn_ember",
        booster: "Ember",
        singles: [set::CINDREL, set::ASHPAW, set::BLAZELYNX, set::SOLARLYNX],
    },
    InnStock {
        map: "inn_storm",
        booster: "Storm",
        singles: [set::SPARKIT, set::ZIPPET, set::ARCFANG, set::FULGUREX],
    },
    InnStock {
        map: "inn_tide",
        booster: "Tide",
        singles: [set::PUDDOCK, set::KELPUP, set::KELPWARDEN, set::GULFWYRM],
    },
    InnStock {
        map: "inn_dream",
        booster: "Dream",
        singles: [set::WISPUN, set::HEXLET, set::SOMNITAR, set::HEXARCH],
    },
    InnStock {
        map: "inn_grove",
        booster: "Grove",
        singles: [
            set::SPRIGGET,
            set::POLLENPUP,
            set::THORNBUCK,
            set::ELDERHART,
        ],
    },
    InnStock {
        map: "inn_stone",
        booster: "Stone",
        singles: [
            set::KNOLLOP,
            set::BRAWLET,
            set::SHALEHOUND,
            set::CRAGMASTIFF,
        ],
    },
];

/// The counter of the inn whose interior is `map`, if it has one (the hub's
/// Hearth Rooms only rest).
pub fn inn_stock(map: &str) -> Option<&'static InnStock> {
    INNS.iter().find(|i| i.map == map)
}

/// Buy single `slot` of an inn's counter, at the single price; no daily limit.
pub fn buy_inn_single(
    p: &mut Progress,
    profile: &mut Profile,
    inn: &InnStock,
    slot: usize,
) -> Result<CardId, ShopError> {
    let &id = inn.singles.get(slot).ok_or(ShopError::NotForSale)?;
    pay(p, SINGLE_PRICE[tier(id).ok_or(ShopError::NotForSale)?])?;
    profile.collection.add(id, 1);
    Ok(id)
}

/// Perrin's six singles for day `day` (days since 1970-01-01): two
/// Commons, three Uncommons and a Rare, distinct, the same all day.
pub fn daily_stock(day: u32) -> [CardId; STOCK_SIZE] {
    let mut rng = Rng::new(u64::from(day) ^ 0x5709_c0de);
    let mut stock = Vec::with_capacity(STOCK_SIZE);
    for (t, &n) in STOCK.iter().enumerate() {
        let mut pool: Vec<CardId> = set::ids().filter(|&id| tier(id) == Some(t)).collect();
        rng.shuffle(&mut pool);
        stock.extend(pool.into_iter().take(n));
    }
    stock.try_into().expect("every tier holds enough cards")
}

/// Buy stock slot `slot` of day `day` (once per day); returns the card.
pub fn buy_single(
    p: &mut Progress,
    profile: &mut Profile,
    day: u32,
    slot: usize,
) -> Result<CardId, ShopError> {
    let &id = daily_stock(day).get(slot).ok_or(ShopError::NotForSale)?;
    let bought = p.stock_bought(day);
    if bought & 1 << slot != 0 {
        return Err(ShopError::SoldOut);
    }
    pay(p, SINGLE_PRICE[tier(id).expect("the stock is priced")])?;
    p.set_stock_bought(day, bought | 1 << slot);
    profile.collection.add(id, 1);
    Ok(id)
}

/// What Perrin pays for one copy; None for promos and basic energy.
pub fn sell_price(id: CardId) -> Option<u32> {
    tier(id).map(|t| SELL_PRICE[t])
}

/// Copies of `id` that may be sold: all but the last, and all but those
/// one of the saved decks uses.
pub fn spare(profile: &Profile, id: CardId) -> usize {
    let used = profile
        .decks
        .iter()
        .map(|d| d.cards.iter().filter(|&&c| c == id).count())
        .max()
        .unwrap_or(0);
    profile.collection.count(id).saturating_sub(used.max(1))
}

/// Sell one copy; returns the Facets earned.
pub fn sell(p: &mut Progress, profile: &mut Profile, id: CardId) -> Result<u32, ShopError> {
    let price = sell_price(id).ok_or(ShopError::NotForSale)?;
    if spare(profile, id) == 0 {
        return Err(ShopError::Keep);
    }
    profile.collection.remove(id, 1);
    p.earn(price);
    Ok(price)
}

/// "Sell extras": every copy beyond four of each card (a deck never needs
/// more); returns the Facets earned.
pub fn sell_extras(p: &mut Progress, profile: &mut Profile) -> u32 {
    let extra: Vec<(CardId, usize)> = profile
        .collection
        .iter()
        .filter(|&(id, n)| n > MAX_COPIES && sell_price(id).is_some())
        .map(|(id, n)| (id, n - MAX_COPIES))
        .collect();
    let mut earned = 0;
    for (id, n) in extra {
        profile.collection.remove(id, n);
        earned += sell_price(id).unwrap_or(0) * n as u32;
    }
    p.earn(earned);
    earned
}

/// What the Press charges for one more copy of `id`; None when it can't be
/// made (a promo, basic energy, or a Rare never owned).
pub fn craft_price(id: CardId, profile: &Profile) -> Option<u32> {
    let t = tier(id)?;
    if profile.collection.count(id) > 0 {
        Some(CRAFT_OWNED_PRICE[t])
    } else {
        CRAFT_NEW_PRICE[t]
    }
}

/// Craft one copy of `id` for Facets.
pub fn craft(p: &mut Progress, profile: &mut Profile, id: CardId) -> Result<(), ShopError> {
    pay(p, craft_price(id, profile).ok_or(ShopError::NotForSale)?)?;
    profile.collection.add(id, 1);
    Ok(())
}

/// Craft one copy of any non-promo card, Rares included, for a voucher.
pub fn craft_with_voucher(
    p: &mut Progress,
    profile: &mut Profile,
    id: CardId,
) -> Result<(), ShopError> {
    tier(id).ok_or(ShopError::NotForSale)?;
    if !p.take(VOUCHER) {
        return Err(ShopError::NoVoucher);
    }
    profile.collection.add(id, 1);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::{record, rival, CONSOLATION_PER_PRIZE, RIVAL_PURSE, WARDEN_PURSE};
    use crate::set::*;
    use crate::trainers::{PURSE_BASE, PURSE_PER_PRIZE, TRAINERS};

    fn rich() -> (Progress, Profile) {
        let mut p = Progress::default();
        p.earn(10_000);
        (p, Profile::new_player())
    }

    /// §3.2, as a table: change a price here and in the design doc.
    #[test]
    fn prices_match_the_design() {
        assert_eq!((THEMED_PACK_PRICE, WILD_PACK_PRICE), (150, 200));
        assert_eq!(SINGLE_PRICE, [20, 60, 240]);
        assert_eq!(CRAFT_OWNED_PRICE, [40, 120, 400]);
        assert_eq!(CRAFT_NEW_PRICE, [Some(80), Some(240), None]);
        assert_eq!(SELL_PRICE, [3, 10, 40]);
        assert_eq!(
            (STOCK, STOCK.iter().sum::<usize>()),
            ([2, 3, 1], STOCK_SIZE)
        );
        // Risk E2: selling pays at most a tenth of crafting, so no loop pays.
        for t in 0..3 {
            assert!(SELL_PRICE[t] * 10 <= CRAFT_OWNED_PRICE[t]);
            assert!(SELL_PRICE[t] < SINGLE_PRICE[t]);
        }
    }

    #[test]
    fn packs_go_on_sale_with_their_rival() {
        let (mut p, mut pr) = rich();
        // Ember, Tide and Wild from the start; Grove once Bramble falls.
        let on: Vec<usize> = (0..BOOSTERS.len())
            .filter(|&b| pack_price(b, &p).is_some())
            .collect();
        assert_eq!(on, [0, 1, WILD]);
        assert_eq!(pack_price(WILD, &p), Some(WILD_PACK_PRICE));
        assert_eq!(buy_pack(&mut p, &mut pr, 3), Err(ShopError::NotForSale));
        assert_eq!(buy_pack(&mut p, &mut pr, 99), Err(ShopError::NotForSale));
        record(rival("bramble").unwrap(), true, &mut p, &mut pr);
        let (coins, packs) = (p.coins(), pr.wallet.packs(3));
        assert_eq!(buy_pack(&mut p, &mut pr, 3), Ok(()));
        assert_eq!((p.coins(), pr.wallet.packs(3)), (coins - 150, packs + 1));
        // Short: nothing changes, and the error says by how much.
        let mut poor = Progress::default();
        assert_eq!(
            buy_pack(&mut poor, &mut pr, WILD),
            Err(ShopError::Short(100))
        );
        assert_eq!(poor.coins(), 100);
    }

    #[test]
    fn the_daily_stock_is_fixed_per_day_and_sells_once() {
        for day in 0..200 {
            let s = daily_stock(day);
            let tiers: Vec<Option<usize>> = s.iter().map(|&id| tier(id)).collect();
            assert_eq!(tiers, [0, 0, 1, 1, 1, 2].map(Some), "day {day}");
            assert!((0..6).all(|i| !s[..i].contains(&s[i])), "distinct");
            assert_eq!(s, daily_stock(day));
        }
        assert_ne!(daily_stock(1), daily_stock(2));
        let (mut p, mut pr) = rich();
        let coins = p.coins();
        let rare = daily_stock(9)[5];
        let had = pr.collection.count(rare);
        assert_eq!(buy_single(&mut p, &mut pr, 9, 5), Ok(rare));
        assert_eq!(
            (p.coins(), pr.collection.count(rare)),
            (coins - 240, had + 1)
        );
        assert_eq!(buy_single(&mut p, &mut pr, 9, 5), Err(ShopError::SoldOut));
        assert_eq!(
            buy_single(&mut p, &mut pr, 9, 6),
            Err(ShopError::NotForSale)
        );
        assert_eq!(buy_single(&mut p, &mut pr, 9, 0).map(tier), Ok(Some(0)));
        assert_eq!(p.stock_bought(9), 0b100001);
        // A new day restocks, and the mask rides in the save string.
        assert_eq!(p.stock_bought(10), 0);
        assert_eq!(Progress::decode(&p.encode()).stock_bought(9), 0b100001);
        assert!(buy_single(&mut p, &mut pr, 10, 5).is_ok());
        assert_eq!((p.stock_bought(9), p.stock_bought(10)), (0, 0b100000));
    }

    #[test]
    fn selling_keeps_the_last_copy_and_what_decks_use() {
        let (mut p, mut pr) = rich();
        let coins = p.coins();
        // The starter deck uses 3 Cindrel (a Common) and the collection holds 3.
        assert_eq!(pr.collection.count(CINDREL), 3);
        assert_eq!(sell(&mut p, &mut pr, CINDREL), Err(ShopError::Keep));
        pr.collection.add(CINDREL, 2);
        assert_eq!(sell(&mut p, &mut pr, CINDREL), Ok(3));
        assert_eq!(spare(&pr, CINDREL), 1);
        // No deck: only the last copy is kept.
        pr.decks.clear();
        pr.collection.add(SNARE_HOOK, 2);
        assert_eq!(sell(&mut p, &mut pr, SNARE_HOOK), Ok(40));
        assert_eq!(sell(&mut p, &mut pr, SNARE_HOOK), Err(ShopError::Keep));
        assert_eq!(
            sell(&mut p, &mut pr, FIRE_ENERGY),
            Err(ShopError::NotForSale)
        );
        pr.collection.add(THISTLEWICK, 3);
        assert_eq!(
            sell(&mut p, &mut pr, THISTLEWICK),
            Err(ShopError::NotForSale)
        );
        assert_eq!(p.coins(), coins + 43);
    }

    #[test]
    fn sell_extras_sells_beyond_four() {
        let (mut p, mut pr) = rich();
        pr.collection.add(SPRIGGET, 7);
        pr.collection.add(HIVEVOLT, 4);
        pr.collection.add(HEXCREST, 9);
        let coins = p.coins();
        let earned = sell_extras(&mut p, &mut pr);
        let sprig = pr.collection.count(SPRIGGET);
        assert_eq!(sprig, 4);
        assert_eq!(pr.collection.count(HIVEVOLT), 4);
        assert_eq!(pr.collection.count(HEXCREST), 9, "promos stay");
        assert_eq!(earned, 3 * 3);
        assert_eq!(p.coins(), coins + earned);
        assert_eq!(sell_extras(&mut p, &mut pr), 0);
    }

    #[test]
    fn crafting_prices_by_ownership_and_the_voucher() {
        let (mut p, mut pr) = rich();
        assert_eq!(pr.collection.count(SPRIGGET), 0);
        assert_eq!(craft_price(SPRIGGET, &pr), Some(80));
        assert_eq!(craft_price(PYREQUILL, &pr), None, "a Rare never owned");
        assert_eq!(craft_price(THISTLEWICK, &pr), None, "a promo");
        assert_eq!(craft_price(FIRE_ENERGY, &pr), None);
        let coins = p.coins();
        assert_eq!(craft(&mut p, &mut pr, SPRIGGET), Ok(()));
        assert_eq!(craft_price(SPRIGGET, &pr), Some(40));
        assert_eq!(
            craft(&mut p, &mut pr, PYREQUILL),
            Err(ShopError::NotForSale)
        );
        assert_eq!(p.coins(), coins - 80);
        assert_eq!(
            craft_with_voucher(&mut p, &mut pr, PYREQUILL),
            Err(ShopError::NoVoucher)
        );
        p.apply(&format!("give:{VOUCHER}")).unwrap();
        assert_eq!(
            craft_with_voucher(&mut p, &mut pr, THISTLEWICK),
            Err(ShopError::NotForSale)
        );
        assert!(p.has_item(VOUCHER), "a refused craft keeps the voucher");
        assert_eq!(craft_with_voucher(&mut p, &mut pr, PYREQUILL), Ok(()));
        assert!(!p.has_item(VOUCHER) && pr.collection.count(PYREQUILL) == 1);
        assert_eq!(craft_price(PYREQUILL, &pr), Some(400));
    }

    #[test]
    fn inn_counters_sell_their_region() {
        use crate::card::Color;
        for inn in &INNS {
            let b = &BOOSTERS[inn.booster()];
            let tiers: Vec<_> = inn.singles.iter().map(|&id| tier(id)).collect();
            assert_eq!(tiers, [Some(0), Some(0), Some(1), Some(2)], "{}", inn.map);
            for &id in &inn.singles {
                let c = card(id)
                    .creature()
                    .expect("inn singles are creatures")
                    .color;
                assert!(
                    b.colors.contains(&c) || c == Color::Colorless,
                    "{}: {}",
                    inn.map,
                    card(id).name
                );
            }
        }
        assert!(inn_stock("inn_hub").is_none() && inn_stock("inn_storm").is_some());
        let (mut p, mut pr) = (Progress::default(), Profile::new_player());
        let ember = inn_stock("inn_ember").unwrap();
        assert_eq!(
            buy_inn_single(&mut p, &mut pr, ember, 3),
            Err(ShopError::Short(140))
        );
        p.earn(400);
        assert_eq!(
            buy_inn_single(&mut p, &mut pr, ember, 3),
            Ok(set::SOLARLYNX)
        );
        assert_eq!(buy_inn_single(&mut p, &mut pr, ember, 0), Ok(set::CINDREL));
        assert_eq!((p.coins(), pr.collection.count(set::SOLARLYNX)), (240, 1));
        assert_eq!(
            buy_inn_single(&mut p, &mut pr, ember, 4),
            Err(ShopError::NotForSale)
        );
    }

    #[test]
    fn shop_errors_read_in_both_languages() {
        use crate::lang::{set_lang, Lang};
        assert_eq!(
            ShopError::Short(5).to_string(),
            "Not enough Facets: ⬡ 5 short."
        );
        set_lang(Lang::Fr);
        let fr = ShopError::Short(5).to_string();
        set_lang(Lang::En);
        assert_eq!(fr, "Pas assez de Facettes\u{a0}: il en manque ⬡ 5.");
    }

    /// One scripted source: minutes it takes, Facets and packs it pays.
    struct Source {
        minutes: u32,
        facets: u32,
        packs: u32,
    }

    /// Packs an hour: every Facet turned into themed packs, plus packs found.
    fn packs_per_hour(sources: &[Source]) -> f32 {
        let minutes: u32 = sources.iter().map(|s| s.minutes).sum();
        let facets: u32 = sources.iter().map(|s| s.facets).sum();
        let packs: u32 = sources.iter().map(|s| s.packs).sum();
        (facets as f32 / THEMED_PACK_PRICE as f32 + packs as f32) * 60.0 / minutes as f32
    }

    /// §3.4: a scripted first playthrough of the 1.2 world, from the real
    /// tables: every route duelist once (5 min each), every rival and the
    /// Warden (two 12-min duels each: a first playthrough loses about half),
    /// the finds of §5.3 met while exploring (10 min per outdoor map, 3 per
    /// house) and the quests of §5.4 with their errands. Rival packs and
    /// promos come on top. Change a price or a purse and this says what it did.
    #[test]
    fn economy_sim() {
        let mut first = vec![];
        first.extend(TRAINERS.iter().filter(|t| !t.story()).map(|t| Source {
            minutes: 5,
            facets: t.purse(),
            packs: 0,
        }));
        // 1.7 Rook's four scenes (§8.2), each played once whatever the result:
        // a minute's scene and two minutes a prize; a first playthrough wins
        // about half of them (the purse), and a loss pays the consolation.
        first.extend(TRAINERS.iter().filter(|t| t.story()).map(|t| Source {
            minutes: 1 + 2 * u32::from(t.prizes),
            facets: (t.purse() + 4 * CONSOLATION_PER_PRIZE) / 2,
            packs: 0,
        }));
        first.extend(RIVALS.iter().map(|r| Source {
            minutes: 24,
            facets: if r.seal { RIVAL_PURSE } else { WARDEN_PURSE },
            packs: 0,
        }));
        // §5.3: 18 Facet finds (⬡ 675), 3 pack finds; the rest are cards and
        // curios. Five outdoor maps and seven houses.
        first.push(Source {
            minutes: 5 * 10 + 7 * 3,
            facets: 675,
            packs: 3,
        });
        // §5.4, (minutes, Facets, packs) per quest: the letter (a Tide pack),
        // the lens (Grove), Moss's seeds over three maps, Dace's kite (Storm),
        // Hob's rye from the quarry, Mags's consult; Odile's two Wild packs
        // for three curios come with the finds.
        for (minutes, facets, packs) in [
            (8, 0, 1),
            (10, 0, 1),
            (15, 150, 0),
            (8, 0, 1),
            (12, 120, 0),
            (6, 100, 0),
            (0, 0, 2),
        ] {
            first.push(Source {
                minutes,
                facets,
                packs,
            });
        }
        // 1.3 Ember and Storm (§10.1-10.2): their duelists come with TRAINERS above;
        // their Facet finds (e_pond 60, e_vent_w 100, e_quarry_nook 80, st_wall 50,
        // st_cliff 40, st_dock 60) and two pack finds (e_rim, st_islet), over five
        // outdoor maps and four houses; the Lantern quest's errand (three shards on
        // three maps, a lantern rather than Facets).
        first.push(Source {
            minutes: 5 * 10 + 4 * 3 + 15,
            facets: 390,
            packs: 2,
        });
        // 1.3 Tide and Dream (§10.3-10.4): their Facet finds (t_quay 50, t_pool_2 80,
        // t_lamp 100, t_seawall 40, d_tree 40, d_ford 60, d_raft 100) and two pack
        // finds (t_nook, d_islet), over five outdoor maps and four houses; the Ferry
        // Pass (⬡ 250) is a sink, left out like the other prices.
        first.push(Source {
            minutes: 5 * 10 + 4 * 3,
            facets: 470,
            packs: 2,
        });
        // The re-lay to the world rules (docs/WORLD_RULES.md): ten small Facet finds
        // that reward the enlarged maps' corners (sq_moving 30, w_camp 40, q_cart 40,
        // e_coalcart 40, st_struck 40, st_wreck 45, t_lookout 40, d_nook_lane 30,
        // d_inn_stump 40, d_hedge_nook 40), two minutes' detour each.
        first.push(Source {
            minutes: 10 * 2,
            facets: 385,
            packs: 0,
        });
        // 1.3 Grove and Stone (§10.5-10.6): their Facet finds (g_walkway 60, g_ring 50,
        // g_glade 100, w_hollow 40, sn_crystal 80, sn_scree 60, sn_menhir 50) and two pack
        // finds (g_cliff, sn_side_3), over six new outdoor maps, the reworked trail and five
        // houses; the errands that pay (the six cairns 60, Agnes's honey 50) with the boots,
        // the crew's three favours and the gloves, which pay in keys. The boots' ⬡ 100 is a
        // sink, left out like the other prices.
        first.push(Source {
            minutes: 7 * 10 + 5 * 3 + 55,
            facets: 440 + 110,
            packs: 2,
        });
        // 1.7 achievements (§12.3) a first playthrough earns on the way: the
        // first win and Awaken 2, the seals and the champion, ten duelists and
        // finds, 50 cards, 25 packs, the field items and an empty-bench win.
        // The rest (⬡ 3,050 of the 3,975) are the long tail: the Gauntlet, the
        // daily duel, every find, card and duelist, streaks and records.
        let on_the_way = [
            "first_win",
            "full_bloom",
            "first_seal",
            "three_seals",
            "all_seals",
            "champion",
            "trainers_10",
            "finds_10",
            "cards_50",
            "packs_25",
            "items_all",
            "empty_bench",
        ];
        first.push(Source {
            minutes: 0,
            facets: crate::stats::ACHIEVEMENTS
                .iter()
                .filter(|a| on_the_way.contains(&a.id))
                .map(|a| a.facets)
                .sum(),
            packs: 0,
        });
        // A lost duel pays CONSOLATION_PER_PRIZE, plus as much per prize
        // taken: about one loss per rival on a first playthrough (its time is
        // counted above), taking 3 prizes on average.
        first.push(Source {
            minutes: 0,
            facets: RIVALS.len() as u32 * 4 * CONSOLATION_PER_PRIZE,
            packs: 0,
        });
        let rate = packs_per_hour(&first);
        // The "exploring" row of §3.4, per hour: 5 two-prize duelists, 6 finds
        // of ⬡ 40, half a quest of ⬡ 160 and 0.8 of a rival.
        let explore = packs_per_hour(&[Source {
            minutes: 60,
            facets: 5 * (PURSE_BASE + 2 * PURSE_PER_PRIZE) + 6 * 40 + 80 + RIVAL_PURSE * 4 / 5,
            packs: 0,
        }]);
        eprintln!("packs per hour: first playthrough {rate:.2}, exploring {explore:.2}");
        assert!(
            (3.5..=6.0).contains(&rate),
            "first playthrough: {rate:.2} packs/h"
        );
        assert!(
            (3.5..=6.0).contains(&explore),
            "exploring: {explore:.2} packs/h"
        );
    }
}
