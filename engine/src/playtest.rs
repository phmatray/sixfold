//! Whole-game playtests run by the AI: the deck sweep over every deck the
//! game ships (prebuilts, rival and Gauntlet decks) and a full campaign
//! playthrough from a new player's collection.

use crate::ai::{Ai, Difficulty};
use crate::campaign::{champion, record, seals, RIVALS, SEALS};
use crate::card::{card, CardId, CardKind, Color, Stage};
use crate::decks::{Prebuilt, PREBUILT};
use crate::endgame::GAUNTLET;
use crate::overworld::Progress;
use crate::profile::Profile;
use crate::set::{self, basic_energy};
use crate::{card_id, validate_deck, Collection, Config, Game, DECK_SIZE, MAX_COPIES};

/// Play one game to the end; returns the winner's seat.
fn play(decks: [Vec<CardId>; 2], seed: u64, prizes: u8, levels: [Difficulty; 2]) -> usize {
    let mut g = Game::new(decks, seed, Config { prizes }).unwrap();
    let mut ais = [
        Ai::new(seed ^ 0xa1, levels[0]),
        Ai::new(seed ^ 0xb2, levels[1]),
    ];
    for _ in 0..50_000 {
        if let Some(w) = g.winner() {
            return w;
        }
        let me = g.view(0).waiting_on.unwrap();
        let a = ais[me].choose_action(&g, me);
        g.apply(a).unwrap();
    }
    panic!("seed {seed}: the game never ended");
}

/// Every distinct deck the game puts in front of the player: the prebuilts,
/// then the rival decks (those that aren't a prebuilt), then the Gauntlet.
fn field() -> Vec<&'static Prebuilt> {
    let mut decks: Vec<&Prebuilt> = PREBUILT.to_vec();
    for r in &RIVALS {
        if decks.iter().all(|d| d.name != r.deck.name) {
            decks.push(&r.deck);
        }
    }
    decks.extend(GAUNTLET.iter().map(|g| &g.deck));
    decks
}

/// Normal-vs-Normal round robin over [`field`], `games` per pair with seats
/// alternating and 6 prizes. Returns `beat[i][j]`: games deck i won against j.
#[allow(clippy::needless_range_loop)] // i, j index both the decks and the table
fn sweep(games: u64) -> (Vec<&'static Prebuilt>, Vec<Vec<u32>>) {
    let decks = field();
    let n = decks.len();
    let mut beat = vec![vec![0u32; n]; n];
    let normal = [Difficulty::Normal; 2];
    for i in 0..n {
        for j in i + 1..n {
            for g in 0..games {
                let seed = (i * n + j) as u64 * 1000 + g;
                let swap = g % 2 == 1;
                let mut pair = [decks[i].cards(), decks[j].cards()];
                if swap {
                    pair.swap(0, 1);
                }
                let first_won = play(pair, seed, 6, normal) == 0;
                let (w, l) = if first_won != swap { (i, j) } else { (j, i) };
                beat[w][l] += 1;
            }
        }
    }
    (decks, beat)
}

/// The deck sweep, 100 games per pair: prints the win table (percent of
/// each pairing, row deck's wins) and each deck's overall rate. Tuning aid:
/// `cargo test --release -p sixfold-engine deck_sweep -- --ignored --nocapture`.
/// Checks that each prebuilt wins 40–60% overall and that no pairing of two
/// prebuilts is beyond 75/25. (Rival and Gauntlet decks may stray further:
/// weakness ties some of them to one colour.)
#[test]
#[ignore]
#[allow(clippy::needless_range_loop)] // i, j index both the decks and the table
fn deck_sweep() {
    let games = 100;
    let (decks, beat) = sweep(games);
    let n = decks.len();
    let head: Vec<String> = (0..n).map(|j| format!("{:>3}", j + 1)).collect();
    println!("{:>2} {:18} {} | all", "#", "deck", head.join(" "));
    let mut worst = (0, 0, 0);
    for i in 0..n {
        let row: Vec<String> = (0..n)
            .map(|j| {
                if i == j {
                    "  -".into()
                } else {
                    format!("{:3}", beat[i][j] * 100 / games as u32)
                }
            })
            .collect();
        let wins: u32 = beat[i].iter().sum();
        let rate = wins * 100 / (games as u32 * (n as u32 - 1));
        println!(
            "{:>2} {:18} {} | {rate:>3}%",
            i + 1,
            decks[i].name,
            row.join(" ")
        );
        for j in 0..n {
            if beat[i][j] > worst.0 {
                worst = (beat[i][j], i, j);
            }
        }
    }
    println!(
        "widest pairing: {} beat {} {}/{games}",
        decks[worst.1].name, decks[worst.2].name, worst.0
    );
    for (i, d) in PREBUILT.iter().enumerate() {
        let wins: u32 = beat[i].iter().sum();
        let rate = wins as f32 / (games as f32 * (n as f32 - 1.0));
        assert!((0.4..=0.6).contains(&rate), "{} won {rate:.2}", d.name);
    }
    let k = PREBUILT.len();
    for i in 0..k {
        for j in 0..k {
            assert!(
                beat[i][j] <= games as u32 * 3 / 4,
                "{} beat {} {}/{games}",
                decks[i].name,
                decks[j].name,
                beat[i][j]
            );
        }
    }
}

/// The colours a deck builder may pick: every colour but Colorless.
const COLOURS: [Color; 6] = [
    Color::Grass,
    Color::Fire,
    Color::Water,
    Color::Lightning,
    Color::Fighting,
    Color::Psychic,
];

/// A simple deck builder: for one or two colours, every owned creature of
/// those colours (and Colorless) whose line starts with an owned Basic, up to
/// 24; owned trainers, two of each, up to 12; the rest basic energy split by
/// the creatures' colours.
// ponytail: a greedy builder, no synergy; good enough to prove the campaign
// can be finished with what the packs give.
fn deck_for(owned: &[(CardId, usize)], colors: [Color; 2]) -> Vec<CardId> {
    let mut deck: Vec<CardId> = vec![];
    for stage in [Stage::Basic, Stage::Stage1, Stage::Stage2] {
        for &(id, n) in owned {
            let Some(cr) = card(id).creature() else {
                continue;
            };
            let fits = cr.color == Color::Colorless || colors.contains(&cr.color);
            let line = cr
                .evolves_from
                .is_none_or(|from| deck.contains(&card_id(from).unwrap()));
            if fits && cr.stage == stage && line && deck.len() < 24 {
                deck.extend(std::iter::repeat_n(id, n.min(24 - deck.len())));
            }
        }
    }
    let trainers = owned
        .iter()
        .filter(|(id, _)| matches!(card(*id).kind, CardKind::Trainer(_)))
        .flat_map(|&(id, n)| std::iter::repeat_n(id, n.min(2)))
        .take(12);
    deck.extend(trainers);
    let per = |c: Color| {
        deck.iter()
            .filter(|&&id| card(id).creature().is_some_and(|cr| cr.color == c))
            .count()
    };
    let (a, b) = (per(colors[0]), per(colors[1]));
    let room = DECK_SIZE - deck.len();
    let first = if colors[0] == colors[1] || a + b == 0 {
        room
    } else {
        room * a / (a + b)
    };
    deck.extend(std::iter::repeat_n(basic_energy(colors[0]).unwrap(), first));
    deck.extend(std::iter::repeat_n(
        basic_energy(colors[1]).unwrap(),
        room - first,
    ));
    deck
}

/// The best [`deck_for`] the collection allows: the legal one with the most
/// creature HP in it.
fn build_deck(c: &Collection) -> Vec<CardId> {
    let owned: Vec<(CardId, usize)> = set::ids()
        .filter(|&id| !card(id).is_basic_energy())
        .map(|id| (id, c.count(id).min(MAX_COPIES)))
        .filter(|&(_, n)| n > 0)
        .collect();
    let hp = |d: &[CardId]| -> u32 {
        d.iter()
            .filter_map(|&id| card(id).creature())
            .map(|cr| u32::from(cr.hp))
            .sum()
    };
    let mut best: Option<Vec<CardId>> = None;
    for (i, &a) in COLOURS.iter().enumerate() {
        for &b in &COLOURS[i..] {
            let d = deck_for(&owned, [a, b]);
            if validate_deck(&d).is_ok()
                && c.can_build(&d)
                && best.as_ref().is_none_or(|x| hp(&d) > hp(x))
            {
                best = Some(d);
            }
        }
    }
    best.expect("the starter collection always builds a deck")
}

fn open_all(p: &mut Profile, seed: &mut u64) {
    for b in 0..crate::booster::BOOSTERS.len() {
        while p.open_pack(b, *seed).is_some() {
            *seed += 1;
        }
    }
}

/// A full playthrough: a new player, played by the Normal AI, takes on the
/// first open rival they haven't beaten, rebuilding their deck from the
/// collection before every duel and opening every pack they win, until the
/// Warden falls. The campaign must be finished in at most 40 duels.
#[test]
fn the_campaign_can_be_finished_from_a_new_profile() {
    let (mut progress, mut profile) = (Progress::default(), Profile::new_player());
    let mut pack_seed = 1;
    open_all(&mut profile, &mut pack_seed);
    let mut duels = 0;
    let mut log = vec![];
    while !champion(&progress) {
        assert!(duels < 40, "not finished after {duels} duels: {log:?}");
        let r = RIVALS
            .iter()
            .find(|r| r.open(&progress) && !r.beaten(&progress))
            .expect("an open rival while the Warden stands");
        let deck = build_deck(&profile.collection);
        let won = play(
            [deck, r.deck.cards()],
            7_000 + duels,
            r.prizes,
            [Difficulty::Normal, r.difficulty],
        ) == 0;
        record(r, won, &mut progress, &mut profile);
        progress.settle();
        open_all(&mut profile, &mut pack_seed);
        duels += 1;
        log.push((r.id, won));
    }
    eprintln!("campaign finished in {duels} duels: {log:?}");
    assert_eq!(seals(&progress), SEALS);
    // Every promo was earned along the way.
    assert!(set::PROMOS
        .iter()
        .all(|&id| profile.collection.count(id) == 1));
}

#[test]
fn the_deck_builder_makes_legal_decks() {
    let p = Profile::new_player();
    let d = build_deck(&p.collection);
    assert_eq!(validate_deck(&d), Ok(()));
    assert!(p.collection.can_build(&d));
}
