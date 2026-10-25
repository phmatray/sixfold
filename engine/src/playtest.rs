//! Whole-game playtests run by the AI: the deck sweep over every deck the
//! game ships (prebuilts, rival and Gauntlet decks), the route duelists' kits
//! and the rivals' AI styles, and a full campaign playthrough from a new
//! player's collection.

use crate::ai::{Ai, Difficulty, Style};
use crate::booster::BOOSTERS;
use crate::campaign::{champion, record, rival, seals, RIVALS, SEALS};
use crate::card::{card, CardId, CardKind, Color, Stage};
use crate::decks::{Prebuilt, PREBUILT, STARTER};
use crate::endgame::GAUNTLET;
use crate::overworld::Progress;
use crate::profile::Profile;
use crate::set::{self, basic_energy};
use crate::shop::{buy_pack, pack_price};
use crate::stats::{award, Stats};
use crate::trainers::{self, Kit, KITS, SPROUTS, TRAINERS};
use crate::{card_id, validate_deck, Collection, Config, Game, DECK_SIZE, MAX_COPIES};

/// Play one game to the end; returns the winner's seat.
fn play(decks: [Vec<CardId>; 2], seed: u64, prizes: u8, levels: [Difficulty; 2]) -> usize {
    play_styled(decks, seed, prizes, levels.map(|l| (l, Style::Balanced)))
}

/// [`play`] with an AI style per seat.
fn play_styled(
    decks: [Vec<CardId>; 2],
    seed: u64,
    prizes: u8,
    seats: [(Difficulty, Style); 2],
) -> usize {
    let mut g = Game::new(decks, seed, Config { prizes }).unwrap();
    let mut ais = [
        Ai::new(seed ^ 0xa1, seats[0].0).with_style(seats[0].1),
        Ai::new(seed ^ 0xb2, seats[1].0).with_style(seats[1].1),
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

/// The style a rival plays this deck with (its campaign or Gauntlet deck);
/// Balanced for a deck no rival uses.
fn style_of(d: &Prebuilt) -> Style {
    RIVALS
        .iter()
        .find(|r| r.deck.name == d.name)
        .or_else(|| {
            GAUNTLET
                .iter()
                .find(|g| g.deck.name == d.name)
                .map(|g| g.rival())
        })
        .map_or(Style::Balanced, |r| r.style)
}

/// Normal-vs-Normal round robin over [`field`], `games` per pair with seats
/// alternating and 6 prizes, each deck played with its rival's style when
/// `styled`. Returns `beat[i][j]`: games deck i won against j.
#[allow(clippy::needless_range_loop)] // i, j index both the decks and the table
fn sweep(games: u64, styled: bool) -> (Vec<&'static Prebuilt>, Vec<Vec<u32>>) {
    let decks = field();
    let n = decks.len();
    let mut beat = vec![vec![0u32; n]; n];
    let seat = |d: &Prebuilt| {
        let style = if styled { style_of(d) } else { Style::Balanced };
        (Difficulty::Normal, style)
    };
    for i in 0..n {
        for j in i + 1..n {
            for g in 0..games {
                let seed = (i * n + j) as u64 * 1000 + g;
                let swap = g % 2 == 1;
                let mut pair = [decks[i].cards(), decks[j].cards()];
                let mut seats = [seat(decks[i]), seat(decks[j])];
                if swap {
                    pair.swap(0, 1);
                    seats.swap(0, 1);
                }
                let first_won = play_styled(pair, seed, 6, seats) == 0;
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
/// Checks that each prebuilt wins 42–58% overall and that no pairing of two
/// prebuilts is beyond 75/25. (Rival and Gauntlet decks may stray further:
/// weakness ties some of them to one colour.)
#[test]
#[ignore]
#[allow(clippy::needless_range_loop)] // i, j index both the decks and the table
fn deck_sweep() {
    let games = 100;
    let (decks, beat) = sweep(games, false);
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
        assert!((0.42..=0.58).contains(&rate), "{} won {rate:.2}", d.name);
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

/// Each deck's overall win rate in a sweep.
fn rates(beat: &[Vec<u32>], games: u64) -> Vec<f32> {
    let n = beat.len() as f32;
    beat.iter()
        .map(|row| row.iter().sum::<u32>() as f32 / (games as f32 * (n - 1.0)))
        .collect()
}

/// Risk G4: the deck sweep with and without the rivals' AI styles, 60 games
/// per pair. A styled rival deck must stay inside 40–60% overall, or no
/// further outside it than the same deck played Balanced. Tuning aid:
/// `cargo test --release -p sixfold-engine style_sweep -- --ignored --nocapture`.
#[test]
#[ignore]
fn style_sweep() {
    let games = 60;
    let (decks, plain) = sweep(games, false);
    let (_, styled) = sweep(games, true);
    let (plain, styled) = (rates(&plain, games), rates(&styled, games));
    let off = |r: f32| (0.4 - r).max(r - 0.6).max(0.0);
    for (i, d) in decks.iter().enumerate() {
        let style = style_of(d);
        println!(
            "{:20} {:12} balanced {:>3.0}%  styled {:>3.0}%",
            d.name.en,
            format!("{style:?}"),
            plain[i] * 100.0,
            styled[i] * 100.0
        );
        if style != Style::Balanced {
            assert!(
                off(styled[i]) <= off(plain[i]) + 0.02,
                "{} drifts with its style: {:.2} -> {:.2}",
                d.name.en,
                plain[i],
                styled[i]
            );
        }
    }
}

/// §4.3: each kit against the prebuilt-level field (the six prebuilts and
/// the starter) at 2 prizes, Normal against Normal, 40 games per opponent;
/// no kit may win more than 60% overall. Then each route duelist as it plays
/// (its deck, prizes and AI) against the starter a new player holds, for the
/// record. Tuning aid:
/// `cargo test --release -p sixfold-engine kit_sweep -- --ignored --nocapture`.
#[test]
#[ignore]
fn kit_sweep() {
    let rate =
        |deck: &[CardId], against: &[Vec<CardId>], prizes: u8, level: Difficulty, games: u64| {
            let mut wins = 0;
            for (k, other) in against.iter().enumerate() {
                for seed in 0..games {
                    let mut pair = [other.clone(), deck.to_vec()];
                    let mut levels = [Difficulty::Normal, level];
                    let swap = seed % 2 == 1;
                    if swap {
                        pair.swap(0, 1);
                        levels.swap(0, 1);
                    }
                    let w = play(pair, 9_000 + 100 * k as u64 + seed, prizes, levels);
                    wins += usize::from((w == 1) != swap);
                }
            }
            wins as f32 / (games * against.len() as u64) as f32
        };
    let field: Vec<Vec<CardId>> = PREBUILT
        .iter()
        .chain([&&STARTER])
        .map(|d| d.cards())
        .collect();
    for r in KITS {
        let w = rate(&Kit::One(r).deck(None), &field, 2, Difficulty::Normal, 40);
        let tier = if SPROUTS.iter().any(|s| s.name == r.name) {
            "sprout"
        } else {
            ""
        };
        println!(
            "kit {:10} {:>3.0}% vs the field at 2 prizes {tier}",
            r.name,
            w * 100.0
        );
        assert!(w <= 0.6, "{} wins {w:.2}", r.name);
    }
    let starter = [STARTER.cards()];
    for t in &TRAINERS {
        let w = rate(&t.deck(), &starter, t.prizes, t.difficulty, 60);
        println!(
            "{:9} {:>3.0}% vs the starter ({} prizes, {:?})",
            t.name,
            w * 100.0,
            t.prizes,
            t.difficulty
        );
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

/// Spend the Facets on packs: the themed pack on sale this player has
/// bought fewest of, while the Facets last. Returns the packs bought.
fn shop_packs(progress: &mut Progress, profile: &mut Profile, bought: &mut [u32]) -> u32 {
    let mut n = 0;
    loop {
        let pick = (0..BOOSTERS.len())
            .filter(|&b| b != crate::booster::WILD && pack_price(b, progress).is_some())
            .min_by_key(|&b| bought[b]);
        let Some(b) = pick else { return n };
        if buy_pack(progress, profile, b).is_err() {
            return n;
        }
        bought[b] += 1;
        n += 1;
    }
}

/// Rival duels the campaign playthrough may take on top of one duel per
/// route duelist. Pinned for 1.2 (risk E1) at the 40 rival duels the
/// pack-reward campaign had (54 duels with 1.2's 14 route duelists); 45
/// since 1.3, whose rules make rivals take longer to beat (a median of 27
/// rival duels over 16 pack seeds, against 16 before; pack seed 1 took 42).
const RIVAL_DUELS: u64 = 45;

/// Count a duel and what it changed, and pay the achievements it earned
/// (their Facets buy packs at the next shop visit, like any others).
fn achieve(
    stats: &mut Stats,
    before: (Progress, Profile),
    won: bool,
    progress: &mut Progress,
    profile: &Profile,
) {
    stats.add("wins", u64::from(won));
    stats.observe((&before.0, &before.1), (progress, profile));
    award(stats, progress, profile);
}

/// A full playthrough: a new player, played by the Normal AI, first meets
/// every route duelist once (win or lose, they walk on), then takes on the
/// first open rival they haven't beaten, rebuilding their deck from the
/// collection before every duel. Every Facet earned buys themed packs at
/// the shop and every pack is opened, from `pack_seed` on, and achievements pay out as earned. Rivals play with
/// their AI style. Stops when the Warden falls or after one duel per route
/// duelist plus `rival_duels`; returns the duels played, the packs
/// bought, the log and the final state.
fn playthrough(
    mut pack_seed: u64,
    rival_duels: u64,
) -> (u64, u32, Vec<(&'static str, bool)>, Progress, Profile) {
    let (mut progress, mut profile) = (Progress::default(), Profile::new_player());
    let mut bought = vec![0; BOOSTERS.len()];
    let mut packs = shop_packs(&mut progress, &mut profile, &mut bought);
    open_all(&mut profile, &mut pack_seed);
    let mut duels = 0;
    let mut log = vec![];
    let mut stats = Stats::default();
    for t in &TRAINERS {
        let deck = build_deck(&profile.collection);
        let won = play(
            [deck, t.deck()],
            6_000 + duels,
            t.prizes,
            [Difficulty::Normal, t.difficulty],
        ) == 0;
        let before = (progress.clone(), profile.clone());
        trainers::record(t, won, &mut progress);
        progress.settle();
        packs += shop_packs(&mut progress, &mut profile, &mut bought);
        open_all(&mut profile, &mut pack_seed);
        achieve(&mut stats, before, won, &mut progress, &profile);
        duels += 1;
        log.push((t.id, won));
    }
    while !champion(&progress) && duels < TRAINERS.len() as u64 + rival_duels {
        let r = RIVALS
            .iter()
            .find(|r| r.open(&progress) && !r.beaten(&progress))
            .expect("an open rival while the Warden stands");
        let deck = build_deck(&profile.collection);
        let won = play_styled(
            [deck, r.deck.cards()],
            7_000 + duels,
            r.prizes,
            [
                (Difficulty::Normal, Style::Balanced),
                (r.difficulty, r.style),
            ],
        ) == 0;
        let before = (progress.clone(), profile.clone());
        record(r, won, &mut progress, &mut profile);
        progress.settle();
        packs += shop_packs(&mut progress, &mut profile, &mut bought);
        open_all(&mut profile, &mut pack_seed);
        achieve(&mut stats, before, won, &mut progress, &profile);
        duels += 1;
        log.push((r.id, won));
    }
    (duels, packs, log, progress, profile)
}

/// The campaign must be finished within one duel per route duelist plus
/// [`RIVAL_DUELS`], with packs bought along the way, in at least two of
/// three playthroughs: one seeded run swings by 30 duels on the packs it
/// opens alone (`docs/BALANCE.md`), so a single one tests the seed.
#[test]
fn the_campaign_can_be_finished_from_a_new_profile() {
    let mut finished = 0;
    for seed in [1, 2, 3] {
        let (duels, packs, log, progress, profile) = playthrough(seed, RIVAL_DUELS);
        if !champion(&progress) {
            eprintln!("pack seed {seed}: not finished after {duels} duels: {log:?}");
            continue;
        }
        eprintln!(
            "pack seed {seed}: campaign finished in {duels} duels, {packs} packs bought, ⬡ {} left: {log:?}",
            progress.coins()
        );
        assert_eq!(seals(&progress), SEALS);
        assert!(packs >= 10, "Facets bought only {packs} packs");
        // Every promo was earned along the way.
        assert!(set::PROMOS
            .iter()
            .all(|&id| profile.collection.count(id) == 1));
        assert!(rival("warden").unwrap().beaten(&progress));
        finished += 1;
    }
    assert!(finished >= 2, "only {finished} of 3 playthroughs finished");
}

#[test]
fn the_deck_builder_makes_legal_decks() {
    let p = Profile::new_player();
    let d = build_deck(&p.collection);
    assert_eq!(validate_deck(&d), Ok(()));
    assert!(p.collection.can_build(&d));
}

/// English left in a French text: words French never has, as words.
fn english_in(line: &str) -> Option<&'static str> {
    const MARKERS: [&str; 15] = [
        " the ",
        "The ",
        "You ",
        " your ",
        "Your ",
        "Opponent",
        " card",
        " damage",
        " turn",
        "Flip ",
        "Active Hexon",
        "Bench ",
        " heads",
        " tails",
        "Choose ",
    ];
    let padded = format!(" {line} ");
    MARKERS.into_iter().find(|m| padded.contains(m))
}

/// A French game reads in French: AI-vs-AI games over the prebuilts, every
/// log line from both seats, every legal action's button and every prompt,
/// plus each set card's effects as rules text. Prints one game's log for
/// review with `-- --nocapture`.
#[test]
fn a_french_game_has_no_english_left() {
    use crate::lang::{set_lang, Lang};
    set_lang(Lang::Fr);
    let mut lines: Vec<String> = vec![];
    let mut sample: Vec<String> = vec![];
    let n = PREBUILT.len();
    for k in 0..4 {
        let seed = 4_200 + k as u64;
        let decks = [PREBUILT[k % n].cards(), PREBUILT[(k + 1) % n].cards()];
        let mut g = Game::new(decks, seed, Config { prizes: 6 }).unwrap();
        let mut ais = [
            Ai::new(seed ^ 0xa1, Difficulty::Normal),
            Ai::new(seed ^ 0xb2, Difficulty::Normal),
        ];
        for _ in 0..50_000 {
            if g.winner().is_some() {
                break;
            }
            let me = g.view(0).waiting_on.unwrap();
            let v = g.view(me);
            lines.extend(v.prompt.map(|p| p.kind.to_string()));
            lines.extend(g.legal_actions().into_iter().map(|a| v.describe_action(a)));
            let a = ais[me].choose_action(&g, me);
            g.apply(a).unwrap();
        }
        assert!(g.winner().is_some(), "seed {seed}: the game never ended");
        for viewer in 0..2 {
            for e in g.log() {
                let l = e.describe(viewer);
                if k == 0 && viewer == 0 {
                    sample.push(l.clone());
                }
                lines.push(l);
            }
        }
    }
    for c in set::CARDS {
        let effects = match &c.kind {
            CardKind::Creature(cr) | CardKind::TrainerBasic(cr) => {
                cr.attacks.iter().flat_map(|a| a.effects).collect()
            }
            CardKind::Trainer(t) => t.effects.iter().collect(),
            CardKind::Energy(_) => vec![],
        };
        lines.extend(effects.into_iter().map(|e| e.to_string()));
    }
    set_lang(Lang::En);
    for l in sample.iter().take(80) {
        eprintln!("{l}");
    }
    for l in &lines {
        assert!(!l.trim().is_empty(), "an empty French text");
        assert!(english_in(l).is_none(), "English in French: {l:?}");
    }
}

/// Balance indicators over the deck sweep's field (Normal vs Normal, 6
/// prizes, seats alternating): first-player win rate, game length, each
/// card's play rate and win rate when played, and deck diversity.
/// `cargo test --release -p sixfold-engine balance_indicators -- --ignored --nocapture`
#[test]
#[ignore]
#[allow(clippy::needless_range_loop)]
fn balance_indicators() {
    use crate::Event;
    use std::collections::{BTreeMap, BTreeSet};
    let games = 100u64;
    let decks = field();
    let n = decks.len();
    let (mut first_wins, mut played_games) = (0u32, 0u32);
    let mut turns: Vec<u32> = vec![];
    let mut wins = vec![0u32; n];
    // card -> (games in a deck that holds it, games it was played, wins when played)
    let mut cards: BTreeMap<CardId, (u32, u32, u32, u32)> = BTreeMap::new();
    for i in 0..n {
        for j in i + 1..n {
            for k in 0..games {
                let seed = (i * n + j) as u64 * 1000 + k;
                let seats = if k % 2 == 1 { [j, i] } else { [i, j] };
                let mut g =
                    Game::new(seats.map(|d| decks[d].cards()), seed, Config { prizes: 6 }).unwrap();
                let mut ais = [
                    Ai::new(seed ^ 0xa1, Difficulty::Normal),
                    Ai::new(seed ^ 0xb2, Difficulty::Normal),
                ];
                while g.winner().is_none() {
                    let me = g.view(0).waiting_on.unwrap();
                    let a = ais[me].choose_action(&g, me);
                    g.apply(a).unwrap();
                }
                let w = g.winner().unwrap();
                let mut first = 0;
                let mut seen = [BTreeSet::new(), BTreeSet::new()];
                for e in g.log() {
                    match e {
                        Event::FirstPlayer { player, .. } => first = *player,
                        Event::Placed { player, card, .. }
                        | Event::Evolved { player, card, .. }
                        | Event::TrainerPlayed { player, card, .. }
                        | Event::EnergyAttached { player, card, .. } => {
                            seen[*player].insert(*card);
                        }
                        _ => {}
                    }
                }
                played_games += 1;
                first_wins += u32::from(w == first);
                turns.push(g.turn());
                wins[seats[w]] += 1;
                for p in 0..2 {
                    let deck: BTreeSet<CardId> = decks[seats[p]].cards().into_iter().collect();
                    for id in deck {
                        let e = cards.entry(id).or_default();
                        e.0 += 1;
                        e.3 += u32::from(w == p);
                        if seen[p].contains(&id) {
                            e.1 += 1;
                            e.2 += u32::from(w == p);
                        }
                    }
                }
            }
        }
    }
    turns.sort_unstable();
    let mean = turns.iter().sum::<u32>() as f32 / turns.len() as f32;
    println!(
        "first player wins {:.1}% of {played_games}; turns mean {mean:.1}, median {}, p10 {}, p90 {}",
        100.0 * first_wins as f32 / played_games as f32,
        turns[turns.len() / 2],
        turns[turns.len() / 10],
        turns[turns.len() * 9 / 10]
    );
    let per = games as f32 * (n as f32 - 1.0);
    let rates: Vec<f32> = wins.iter().map(|&w| w as f32 / per).collect();
    let k = PREBUILT.len();
    println!(
        "prebuilts within 45-55%: {}/{k}; decks above 55%: {:?}",
        rates[..k]
            .iter()
            .filter(|r| (0.45..=0.55).contains(*r))
            .count(),
        (0..n)
            .filter(|&i| rates[i] > 0.55)
            .map(|i| format!("{} {:.0}%", decks[i].name, rates[i] * 100.0))
            .collect::<Vec<_>>()
    );
    println!("card\tin_deck_games\tplay_rate\twin_when_played\twin_when_not_played");
    for (id, (inn, pl, w, wd)) in cards {
        if !card(id).is_basic_energy() {
            let pct = |a: u32, b: u32| {
                if b > 0 {
                    100.0 * a as f32 / b as f32
                } else {
                    0.0
                }
            };
            println!(
                "{}\t{inn}\t{:.0}\t{:.0}\t{:.0}",
                card(id).name,
                pct(pl, inn),
                pct(w, pl),
                pct(wd - w, inn - pl)
            );
        }
    }
}

/// The first player's edge in the post-game: every Gauntlet rung and every
/// daily duel (a rival's campaign deck) as it plays (prizes, AI level and
/// style) against each prebuilt played by the Normal AI, 40 games each with
/// seats alternating. Prints the first player's win rate for each.
/// `cargo test --release -p sixfold-engine postgame_first_player -- --ignored --nocapture`
#[test]
#[ignore]
fn postgame_first_player_edge() {
    use crate::Event;
    let edge = |deck: &Prebuilt, prizes: u8, seat: (Difficulty, Style)| {
        let (mut first, mut all) = (0u32, 0u32);
        for (i, mine) in PREBUILT.iter().enumerate() {
            for k in 0..40u64 {
                let seed = 50_000 + i as u64 * 100 + k;
                let mut pair = [mine.cards(), deck.cards()];
                let mut seats = [(Difficulty::Normal, Style::Balanced), seat];
                if k % 2 == 1 {
                    pair.swap(0, 1);
                    seats.swap(0, 1);
                }
                let mut g = Game::new(pair, seed, Config { prizes }).unwrap();
                let mut ais = seats.map(|(l, s)| Ai::new(seed ^ 0xa1, l).with_style(s));
                while g.winner().is_none() {
                    let me = g.view(0).waiting_on.unwrap();
                    let a = ais[me].choose_action(&g, me);
                    g.apply(a).unwrap();
                }
                let starter = g.log().iter().find_map(|e| match e {
                    Event::FirstPlayer { player, .. } => Some(*player),
                    _ => None,
                });
                first += u32::from(g.winner() == starter);
                all += 1;
            }
        }
        (first, all)
    };
    let mut total = (0, 0);
    for (label, list) in [
        (
            "Gauntlet",
            GAUNTLET
                .iter()
                .map(|r| (&r.deck, r.prizes, (r.difficulty, r.rival().style)))
                .collect::<Vec<_>>(),
        ),
        (
            "daily",
            RIVALS
                .iter()
                .map(|r| (&r.deck, r.prizes, (r.difficulty, r.style)))
                .collect(),
        ),
    ] {
        let mut sum = (0, 0);
        for (deck, prizes, seat) in list {
            let (f, a) = edge(deck, prizes, seat);
            println!(
                "{label:8} {:18} first player wins {:.1}%",
                deck.name,
                100.0 * f as f32 / a as f32
            );
            sum = (sum.0 + f, sum.1 + a);
        }
        println!(
            "{label:8} overall: first player wins {:.1}% of {}",
            100.0 * sum.0 as f32 / sum.1 as f32,
            sum.1
        );
        total = (total.0 + sum.0, total.1 + sum.1);
    }
    println!(
        "post-game overall: first player wins {:.1}% of {}",
        100.0 * total.0 as f32 / total.1 as f32,
        total.1
    );
}

/// How long the campaign takes with no duel budget, over 16 pack seeds (one
/// seeded run swings widely on its packs alone): prints each and the median.
/// `cargo test --release -p sixfold-engine campaign_length -- --ignored --nocapture`
#[test]
#[ignore]
fn campaign_length() {
    let mut all: Vec<u64> = (1..=16)
        .map(|seed| {
            let (duels, packs, _, progress, _) = playthrough(seed, 1_000);
            assert!(champion(&progress), "pack seed {seed} never finished");
            println!("pack seed {seed}: {duels} duels, {packs} packs");
            duels
        })
        .collect();
    all.sort_unstable();
    println!(
        "campaign: {}-{} duels, median {}",
        all[0],
        all[15],
        (all[7] + all[8]) / 2
    );
}
