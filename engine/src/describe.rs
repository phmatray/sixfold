//! Text for the UI, in English or French: button labels, prompt titles,
//! tooltips and the game log. Player-relative wording ("You" / "Opponent") is
//! from the viewer's side. Nothing here changes the game.

use std::fmt;

use crate::action::{Action, Event, PromptKind};
use crate::card::{card, CardId, Color, Stage};
use crate::condition::{Placed, Shield, Status};
use crate::effect::{Choice, Effect, Find};
use crate::lang::{fr, tr};
use crate::view::View;

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Color::Grass => tr("Grass", "Herbe"),
            Color::Fire => tr("Fire", "Feu"),
            Color::Water => tr("Water", "Eau"),
            Color::Lightning => tr("Lightning", "Foudre"),
            Color::Fighting => tr("Fighting", "Combat"),
            Color::Psychic => tr("Psychic", "Psychique"),
            Color::Colorless => tr("Colorless", "Incolore"),
        })
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Stage::Basic => tr("Basic", "Base"),
            Stage::Stage1 => tr("Stage 1", "Stade 1"),
            Stage::Stage2 => tr("Stage 2", "Stade 2"),
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Status::Confused => tr("Confused", "Confuse"),
            Status::Asleep => tr("Asleep", "Endormie"),
            Status::Paralyzed => tr("Paralyzed", "Paralysée"),
            Status::Poisoned => tr("Poisoned", "Empoisonnée"),
            Status::DoublePoisoned => tr("Double-poisoned", "Gravement empoisonnée"),
        })
    }
}

impl fmt::Display for Find {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Find::Any => tr("a card", "une carte"),
            Find::Basic => tr("a Basic creature", "une carte créature de Base"),
            Find::Creature => tr("a creature card", "une carte créature"),
            Find::BasicEnergy => tr("a basic energy", "une énergie de base"),
            Find::Trainer => tr("a trainer", "une carte Dresseur"),
        })
    }
}

/// "the defending creature can't attack", completing "During your opponent's
/// next turn, ...".
impl fmt::Display for Placed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if fr() {
            // "la créature en défense" is what the log replaces with the Active.
            return match self {
                Placed::CantRetreat => {
                    write!(f, "la créature en défense ne peut pas battre en retraite")
                }
                Placed::CantAttack => write!(f, "la créature en défense ne peut pas attaquer"),
                Placed::DisabledAttack(i) => write!(
                    f,
                    "la créature en défense ne peut pas utiliser l'attaque {}",
                    i + 1
                ),
                Placed::Accuracy => {
                    write!(
                        f,
                        "la créature en défense doit obtenir face à pile ou face pour attaquer"
                    )
                }
                Placed::Weaken(n) => write!(
                    f,
                    "les attaques de la créature en défense infligent {n} dégâts de moins"
                ),
            };
        }
        match self {
            Placed::CantRetreat => write!(f, "the defending creature can't retreat"),
            Placed::CantAttack => write!(f, "the defending creature can't attack"),
            Placed::DisabledAttack(i) => {
                write!(f, "the defending creature can't use attack {}", i + 1)
            }
            Placed::Accuracy => write!(f, "the defending creature must flip heads to attack"),
            Placed::Weaken(n) => write!(f, "the defending creature's attacks do {n} less damage"),
        }
    }
}

/// Completes "During your opponent's next turn, ...".
impl fmt::Display for Shield {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if fr() {
            // "cette créature" is what the log replaces with the Active.
            return match self {
                Shield::PreventAll => write!(
                    f,
                    "cette créature ne subit aucun effet des attaques, dégâts compris"
                ),
                Shield::PreventDamage => {
                    write!(f, "cette créature ne subit aucun dégât des attaques")
                }
                Shield::Reduce(n) => write!(
                    f,
                    "les attaques infligent {n} dégâts de moins à cette créature"
                ),
                Shield::PreventBelow(n) => write!(
                    f,
                    "cette créature ne subit pas les dégâts d'attaque inférieurs à {n}"
                ),
                Shield::Halve => write!(
                    f,
                    "les attaques n'infligent que la moitié de leurs dégâts à cette créature"
                ),
                Shield::CounterKo => write!(
                    f,
                    "si une attaque met cette créature K.O., la créature attaquante est aussi mise K.O."
                ),
            };
        }
        match self {
            Shield::PreventAll => write!(
                f,
                "prevent all effects of attacks, damage included, done to this creature"
            ),
            Shield::PreventDamage => {
                write!(f, "prevent all damage done to this creature by attacks")
            }
            Shield::Reduce(n) => write!(f, "this creature takes {n} less damage from attacks"),
            Shield::PreventBelow(n) => write!(
                f,
                "prevent attack damage of less than {n} done to this creature"
            ),
            Shield::Halve => write!(f, "this creature takes half damage from attacks"),
            Shield::CounterKo => write!(
                f,
                "if an attack knocks this creature out, the attacker is knocked out too"
            ),
        }
    }
}

fn plural(n: impl Into<u32>, word: &str) -> String {
    let n = n.into();
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// "2 cartes", "1 carte", "0 carte": French plural (0 and 1 singular) on the
/// first word ("2 cartes Énergie Feu").
fn pluriel(n: impl Into<u32>, word: &str) -> String {
    let n = n.into();
    if n <= 1 {
        format!("{n} {word}")
    } else {
        let (head, tail) = word.split_once(' ').map_or((word, ""), |(h, t)| (h, t));
        let sep = if tail.is_empty() { "" } else { " " };
        format!("{n} {head}s{sep}{tail}")
    }
}

/// The French agreement ending for `n` things: "" or "s".
fn s(n: impl Into<u32>) -> &'static str {
    if n.into() > 1 {
        "s"
    } else {
        ""
    }
}

fn coin_prefix(coin: bool) -> &'static str {
    if coin {
        tr(
            "Flip a coin. If heads, ",
            "Lancez une pièce. Si c'est face, ",
        )
    } else {
        ""
    }
}

/// One rules sentence per effect, from the user's side ("this creature" is
/// the one attacking or using the Power). Good for tooltips, and as a fallback
/// when a card's own `text` is empty.
impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if fr() {
            return effect_fr(self, f);
        }
        use Effect::*;
        match *self {
            AddPerHeads { coins, per } => write!(
                f,
                "Flip {}. This attack does {per} more damage for each heads.",
                plural(coins, "coin")
            ),
            AddPerDamage { per, own } => {
                let on = if own { "this creature" } else { "the defending creature" };
                write!(f, "This attack does {per} more damage for each 10 damage on {on}.")
            }
            IgnoreWeaknessResistance => {
                write!(f, "This attack's damage isn't affected by weakness or resistance.")
            }
            DoubleNextAttack => write!(
                f,
                "During your next turn, this creature's attack does double damage."
            ),
            Recoil(n) => write!(f, "This creature does {n} damage to itself."),
            BenchDamage {
                amount,
                own,
                opponent,
            } => {
                let whose = match (own, opponent) {
                    (true, true) => "each player's",
                    (true, false) => "your",
                    _ => "your opponent's",
                };
                write!(f, "Does {amount} damage to each of {whose} Benched creatures.")
            }
            Snipe(n) => write!(f, "Does {n} damage to 1 of your opponent's creatures."),
            Inflict { status, coin } => {
                let lead = coin_prefix(coin);
                let the = if coin { "the" } else { "The" };
                write!(f, "{lead}{the} defending creature is now {status}.")
            }
            InflictSelf(status) => write!(f, "This creature is now {status}."),
            Cure { both: false } => write!(f, "Removes all conditions from your Active creature."),
            Cure { both: true } => write!(f, "Removes all conditions from both Active creatures."),
            Protect { shield, coin } => {
                let during = if coin { "during" } else { "During" };
                write!(
                    f,
                    "{}{during} your opponent's next turn, {shield}.",
                    coin_prefix(coin)
                )
            }
            Place { placed, coin } => {
                let during = if coin { "during" } else { "During" };
                write!(
                    f,
                    "{}{during} your opponent's next turn, {placed}.",
                    coin_prefix(coin)
                )
            }
            DisableAttack => write!(
                f,
                "Choose 1 of the defending creature's attacks. It can't be used during your opponent's next turn."
            ),
            TrainerLock => write!(f, "Your opponent can't play trainers during their next turn."),
            DefenderWeakness(c) => write!(f, "The defending creature's weakness becomes {c}."),
            OwnResistance(c) => write!(f, "This creature's resistance becomes {c}."),
            DiscardEnergy {
                color: Color::Colorless,
                count,
            } => write!(f, "Discard {} attached to this creature.", plural(count, "energy card")),
            DiscardEnergy { color, count } => write!(
                f,
                "Discard {} attached to this creature.",
                plural(count, &format!("{color} energy card"))
            ),
            DiscardAllEnergy => write!(f, "Discard all energy attached to this creature."),
            DiscardDefenderEnergy => {
                write!(f, "Discard 1 energy card attached to the defending creature.")
            }
            SwitchDefender {
                chooser_is_attacker: true,
            } => write!(
                f,
                "Switch the defending creature with 1 of your opponent's Benched creatures, chosen by you."
            ),
            SwitchDefender {
                chooser_is_attacker: false,
            } => write!(
                f,
                "Your opponent switches the defending creature with 1 of their Benched creatures."
            ),
            SwitchSelf => write!(f, "Switch your Active creature with 1 of your Benched creatures."),
            Heal(n) => write!(f, "Removes {n} damage from your Active creature."),
            HealAll(n) => write!(f, "Removes {n} damage from each of your creatures."),
            AttachFromDeck(Color::Colorless) => write!(
                f,
                "Search your deck for a basic energy card and attach it to this creature, then shuffle your deck."
            ),
            AttachFromDeck(c) => write!(
                f,
                "Search your deck for a {c} energy card and attach it to this creature, then shuffle your deck."
            ),
            Devolve => write!(
                f,
                "The defending creature returns to its previous stage; the removed card goes to its owner's hand."
            ),
            StageSkip => write!(
                f,
                "Put a Stage 2 card from your hand onto the matching Basic creature in play."
            ),
            Draw(n) => write!(f, "Draw {}.", plural(n, "card")),
            DiscardHand => write!(f, "Discard your hand."),
            Search(find) => write!(
                f,
                "Search your deck for {find}, put it into your hand, then shuffle your deck."
            ),
            Recover(find) => write!(f, "Put {find} from your discard pile into your hand."),
            AttachPlus10 => write!(
                f,
                "Attach to your Active creature: its attacks do 10 more damage this turn."
            ),
            AttachMinus20 => write!(
                f,
                "Attach to 1 of your creatures: it takes 20 less damage until the end of your opponent's next turn."
            ),
            OnceWhileInPlay => write!(f, "Usable only once while this creature is in play."),
            CopyAttack => write!(f, "Use 1 of the defending creature's attacks as this attack."),
            SetOwnColor(c) => write!(f, "This creature's colour becomes {c}."),
        }
    }
}

/// [`Effect`]'s rules sentence in French.
fn effect_fr(e: &Effect, f: &mut fmt::Formatter) -> fmt::Result {
    use Effect::*;
    match *e {
        AddPerHeads { coins, per } => {
            let coins = if coins == 1 {
                "une pièce".to_string()
            } else {
                pluriel(coins, "pièce")
            };
            write!(
                f,
                "Lancez {coins}. Cette attaque inflige {per} dégâts de plus pour chaque face."
            )
        }
        AddPerDamage { per, own } => {
            let on = if own {
                "cette créature"
            } else {
                "la créature en défense"
            };
            write!(
                f,
                "Cette attaque inflige {per} dégâts de plus pour chaque tranche de 10 dégâts sur {on}."
            )
        }
        IgnoreWeaknessResistance => write!(
            f,
            "Les dégâts de cette attaque ne tiennent compte ni de la Faiblesse ni de la Résistance."
        ),
        DoubleNextAttack => write!(
            f,
            "Pendant votre prochain tour, l'attaque de cette créature inflige le double de dégâts."
        ),
        Recoil(n) => write!(f, "Cette créature s'inflige {n} dégâts."),
        BenchDamage {
            amount,
            own,
            opponent,
        } => {
            let whose = match (own, opponent) {
                (true, true) => "de Banc des deux joueurs",
                (true, false) => "de votre Banc",
                _ => "du Banc adverse",
            };
            write!(f, "Inflige {amount} dégâts à chaque créature {whose}.")
        }
        Snipe(n) => write!(f, "Inflige {n} dégâts à 1 créature de votre adversaire."),
        Inflict { status, coin } => {
            let the = if coin { "la" } else { "La" };
            write!(
                f,
                "{}{the} créature en défense est maintenant {status}.",
                coin_prefix(coin)
            )
        }
        InflictSelf(status) => write!(f, "Cette créature est maintenant {status}."),
        Cure { both: false } => {
            write!(f, "Soigne toutes les altérations de votre créature Active.")
        }
        Cure { both: true } => {
            write!(f, "Soigne toutes les altérations des deux créatures Actives.")
        }
        Protect { shield, coin } => {
            let during = if coin { "pendant" } else { "Pendant" };
            write!(
                f,
                "{}{during} le prochain tour de votre adversaire, {shield}.",
                coin_prefix(coin)
            )
        }
        Place { placed, coin } => {
            let during = if coin { "pendant" } else { "Pendant" };
            write!(
                f,
                "{}{during} le prochain tour de votre adversaire, {placed}.",
                coin_prefix(coin)
            )
        }
        DisableAttack => write!(
            f,
            "Choisissez 1 des attaques de la créature en défense. Elle ne peut pas être utilisée pendant le prochain tour de votre adversaire."
        ),
        TrainerLock => write!(
            f,
            "Votre adversaire ne peut pas jouer de cartes Dresseur pendant son prochain tour."
        ),
        DefenderWeakness(c) => write!(f, "La Faiblesse de la créature en défense devient {c}."),
        OwnResistance(c) => write!(f, "La Résistance de cette créature devient {c}."),
        DiscardEnergy {
            color: Color::Colorless,
            count,
        } => write!(
            f,
            "Défaussez {} attachée{} à cette créature.",
            pluriel(count, "carte Énergie"),
            s(count)
        ),
        DiscardEnergy { color, count } => write!(
            f,
            "Défaussez {} attachée{} à cette créature.",
            pluriel(count, &format!("carte Énergie {color}")),
            s(count)
        ),
        DiscardAllEnergy => write!(f, "Défaussez toutes les énergies attachées à cette créature."),
        DiscardDefenderEnergy => write!(
            f,
            "Défaussez 1 carte Énergie attachée à la créature en défense."
        ),
        SwitchDefender {
            chooser_is_attacker: true,
        } => write!(
            f,
            "Échangez la créature en défense contre 1 des créatures du Banc adverse, de votre choix."
        ),
        SwitchDefender {
            chooser_is_attacker: false,
        } => write!(
            f,
            "Votre adversaire échange la créature en défense contre 1 des créatures de son Banc."
        ),
        SwitchSelf => write!(
            f,
            "Échangez votre créature Active contre 1 des créatures de votre Banc."
        ),
        Heal(n) => write!(f, "Soigne {n} dégâts de votre créature Active."),
        HealAll(n) => write!(f, "Soigne {n} dégâts de chacune de vos créatures."),
        AttachFromDeck(Color::Colorless) => write!(
            f,
            "Cherchez une carte Énergie de base dans votre deck et attachez-la à cette créature, puis mélangez votre deck."
        ),
        AttachFromDeck(c) => write!(
            f,
            "Cherchez une carte Énergie {c} dans votre deck et attachez-la à cette créature, puis mélangez votre deck."
        ),
        Devolve => write!(
            f,
            "La créature en défense régresse à son stade précédent ; la carte retirée retourne dans la main de son propriétaire."
        ),
        StageSkip => write!(
            f,
            "Posez une carte Stade 2 de votre main sur la créature de Base correspondante en jeu."
        ),
        Draw(n) => write!(f, "Piochez {}.", pluriel(n, "carte")),
        DiscardHand => write!(f, "Défaussez votre main."),
        Search(find) => write!(
            f,
            "Cherchez {find} dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."
        ),
        Recover(find) => write!(
            f,
            "Prenez {find} dans votre défausse et ajoutez-la à votre main."
        ),
        AttachPlus10 => write!(
            f,
            "À attacher à votre créature Active : ses attaques infligent 10 dégâts de plus pendant ce tour."
        ),
        AttachMinus20 => write!(
            f,
            "À attacher à 1 de vos créatures : elle subit 20 dégâts de moins jusqu'à la fin du prochain tour de votre adversaire."
        ),
        OnceWhileInPlay => write!(
            f,
            "Utilisable une seule fois tant que cette créature est en jeu."
        ),
        CopyAttack => write!(
            f,
            "Cette attaque copie 1 des attaques de la créature en défense."
        ),
        SetOwnColor(c) => write!(f, "Cette créature devient une créature {c}."),
    }
}

/// [`PromptKind`]'s heading in French.
fn prompt_fr(k: &PromptKind, f: &mut fmt::Formatter) -> fmt::Result {
    use Effect::*;
    match k {
        PromptKind::ChooseActive => write!(f, "Choisissez votre créature Active"),
        PromptKind::ChooseBench => {
            write!(
                f,
                "Placez des créatures de Base sur votre Banc, puis Terminé"
            )
        }
        PromptKind::MainPhase => write!(f, "Votre tour"),
        PromptKind::ChooseRetreatEnergy { cost, .. } => write!(
            f,
            "Choisissez l'énergie à défausser pour battre en retraite (coût {cost})"
        ),
        PromptKind::ChooseRetreatTarget { .. } => {
            write!(f, "Choisissez la créature de Banc à faire entrer")
        }
        PromptKind::ChoosePrize { count } => {
            write!(f, "Prenez une Récompense ({count} à prendre)")
        }
        PromptKind::ChooseReplacement => {
            write!(f, "Votre Active est K.O. : choisissez sa remplaçante")
        }
        PromptKind::Choose { effect, .. } => match *effect {
            DisableAttack => write!(f, "Choisissez l'attaque à bloquer"),
            CopyAttack => write!(f, "Choisissez l'attaque à copier"),
            DiscardDefenderEnergy => write!(f, "Choisissez l'énergie à défausser"),
            Snipe(n) => write!(f, "Choisissez la créature qui subira {n} dégâts"),
            SwitchDefender {
                chooser_is_attacker: true,
            } => write!(f, "Choisissez la créature adverse à faire entrer"),
            SwitchDefender {
                chooser_is_attacker: false,
            } => write!(f, "Votre Active est échangée : choisissez sa remplaçante"),
            SwitchSelf => write!(f, "Choisissez la créature de Banc à faire entrer"),
            Search(find) => write!(f, "Choisissez {find} à prendre dans votre deck"),
            Recover(find) => write!(f, "Choisissez {find} à prendre dans votre défausse"),
            AttachMinus20 => write!(f, "Choisissez la créature qui recevra cette carte"),
            AttachFromDeck(_) => write!(f, "Choisissez l'énergie de votre deck à attacher"),
            StageSkip => write!(
                f,
                "Choisissez la carte Stade 2 et la créature de Base qui la reçoit"
            ),
            other => write!(f, "Choisissez : {other}"),
        },
    }
}

/// The instruction shown to the player being asked.
impl fmt::Display for PromptKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if fr() {
            return prompt_fr(self, f);
        }
        use Effect::*;
        match self {
            PromptKind::ChooseActive => write!(f, "Choose your Active creature"),
            PromptKind::ChooseBench => write!(f, "Put Basic creatures on your Bench, then Done"),
            PromptKind::MainPhase => write!(f, "Your turn"),
            PromptKind::ChooseRetreatEnergy { cost, .. } => {
                write!(f, "Choose energy to discard to retreat (cost {cost})")
            }
            PromptKind::ChooseRetreatTarget { .. } => {
                write!(f, "Choose the Benched creature to switch in")
            }
            PromptKind::ChoosePrize { count } => {
                write!(f, "Take a prize card ({count} to take)")
            }
            PromptKind::ChooseReplacement => {
                write!(f, "Your Active was knocked out: choose its replacement")
            }
            PromptKind::Choose { effect, .. } => match *effect {
                DisableAttack => write!(f, "Choose the attack to disable"),
                CopyAttack => write!(f, "Choose the attack to copy"),
                DiscardDefenderEnergy => write!(f, "Choose the energy to discard"),
                Snipe(n) => write!(f, "Choose the creature to take {n} damage"),
                SwitchDefender {
                    chooser_is_attacker: true,
                } => write!(f, "Choose the opposing creature to switch in"),
                SwitchDefender {
                    chooser_is_attacker: false,
                } => write!(f, "Your Active is being switched: choose its replacement"),
                SwitchSelf => write!(f, "Choose the Benched creature to switch in"),
                Search(find) => write!(f, "Choose {find} to take from your deck"),
                Recover(find) => write!(f, "Choose {find} to take from your discard pile"),
                AttachMinus20 => write!(f, "Choose the creature to attach this card to"),
                AttachFromDeck(_) => write!(f, "Choose the energy to attach from your deck"),
                StageSkip => write!(f, "Choose the Stage 2 card and the Basic it goes on"),
                other => write!(f, "Choose: {other}"),
            },
        }
    }
}

fn name(id: CardId) -> &'static str {
    card(id).label()
}

fn slot_label(slot: usize) -> String {
    if slot == 0 {
        "Active".to_string()
    } else {
        tr(format!("Bench {slot}"), format!("Banc {slot}"))
    }
}

impl View<'_> {
    fn hand_name(&self, h: usize) -> &'static str {
        self.hand.get(h).map_or("?", |&id| name(id))
    }

    fn creature_name(&self, player: usize, slot: usize) -> &'static str {
        let side = &self.sides[player];
        let c = if slot == 0 {
            side.active
        } else {
            side.bench.get(slot - 1)
        };
        c.map_or("?", |c| name(c.top()))
    }

    /// A button label for one of `Game::legal_actions()`, from this view's
    /// player (the one being asked).
    pub fn describe_action(&self, action: Action) -> String {
        let me = self.me;
        let kind = self.prompt.map(|p| &p.kind);
        let active = self.sides[me].active;
        match action {
            Action::PlayBasic(h) if kind == Some(&PromptKind::ChooseActive) => tr(
                format!("Make {} your Active", self.hand_name(h)),
                format!("Jouer {} en Active", self.hand_name(h)),
            ),
            Action::PlayBasic(h) => tr(
                format!("Put {} on your Bench", self.hand_name(h)),
                format!("Placer {} sur votre Banc", self.hand_name(h)),
            ),
            Action::Evolve { hand, slot } => format!(
                "{} {} {} {}",
                tr("Evolve", "Faire évoluer"),
                self.creature_name(me, slot),
                tr("into", "en"),
                self.hand_name(hand)
            ),
            Action::AttachEnergy { hand, slot } => format!(
                "{} {} {} {} ({})",
                tr("Attach", "Attacher"),
                self.hand_name(hand),
                tr("to", "à"),
                self.creature_name(me, slot),
                slot_label(slot)
            ),
            Action::PlayTrainer(h) => format!("{} {}", tr("Play", "Jouer"), self.hand_name(h)),
            Action::Retreat => {
                let cost = self.sides[me].retreat_cost;
                tr(
                    format!("Retreat {} (cost {cost})", self.creature_name(me, 0)),
                    format!("Battre en retraite (coût {cost})"),
                )
            }
            Action::DiscardEnergy(i) => {
                let e = active.and_then(|a| a.energy.get(i));
                format!(
                    "{} {}",
                    tr("Discard", "Défausser"),
                    e.map_or("?", |&id| name(id))
                )
            }
            Action::Promote(slot) => format!(
                "{} {}",
                tr("Switch in", "Faire entrer"),
                self.creature_name(me, slot)
            ),
            Action::Cancel => tr("Cancel", "Annuler").to_string(),
            Action::Attack(i) => {
                let a = active.and_then(|a| a.creature().attacks.get(i));
                match a {
                    Some(a) if a.damage > 0 => format!("{} ({})", a.label(), a.damage),
                    Some(a) => a.label().to_string(),
                    None => "?".to_string(),
                }
            }
            Action::UsePower(slot) => {
                let side = &self.sides[me];
                let c = if slot == 0 {
                    side.active
                } else {
                    side.bench.get(slot - 1)
                };
                let power = c.and_then(|c| c.creature().power.as_ref());
                format!(
                    "{} {} ({})",
                    tr("Use", "Utiliser"),
                    power.map_or("?", |p| p.label()),
                    self.creature_name(me, slot)
                )
            }
            Action::Choose(i) => match kind {
                Some(PromptKind::Choose { options, .. }) => options
                    .get(i)
                    .map_or("?".to_string(), |&c| self.describe_choice(c)),
                _ => "?".to_string(),
            },
            Action::TakePrize(i) => {
                format!("{} {}", tr("Take prize", "Prendre la Récompense"), i + 1)
            }
            Action::Done if kind == Some(&PromptKind::MainPhase) => {
                tr("End turn", "Terminer le tour").to_string()
            }
            Action::Done => tr("Done", "Terminé").to_string(),
        }
    }

    /// A label for one option of a `PromptKind::Choose`, from this view's player.
    pub fn describe_choice(&self, choice: Choice) -> String {
        let me = self.me;
        let defender = self.sides[1 - me].active;
        match choice {
            Choice::Slot { player, slot } => {
                let at = match (player == me, fr()) {
                    (true, false) => format!("your {}", slot_label(slot)),
                    (false, false) => format!("opponent's {}", slot_label(slot)),
                    (true, true) => format!("votre {}", slot_label(slot)),
                    (false, true) if slot == 0 => "Active adverse".to_string(),
                    (false, true) => format!("Banc adverse {slot}"),
                };
                format!("{} ({at})", self.creature_name(player, slot))
            }
            Choice::Energy(i) => defender
                .and_then(|a| a.energy.get(i))
                .map_or("?".to_string(), |&id| name(id).to_string()),
            Choice::Attack(i) => defender
                .and_then(|a| a.creature().attacks.get(i))
                .map_or("?".to_string(), |a| a.label().to_string()),
            Choice::Card(id) => name(id).to_string(),
            Choice::Evolve { hand, slot } => format!(
                "{} {} {}",
                self.hand_name(hand),
                tr("onto", "sur"),
                self.creature_name(me, slot)
            ),
        }
    }
}

impl Event {
    /// This event shows a card `viewer` isn't allowed to see: another player's
    /// draw, deck search or prize. Show it without the card (as
    /// [`Event::describe`] does); an AI must not read those cards.
    pub fn is_private(&self, viewer: usize) -> bool {
        match *self {
            Event::CardDrawn { player, .. } | Event::PrizeTaken { player, .. } => player != viewer,
            Event::CardToHand {
                player, from_deck, ..
            } => from_deck && player != viewer,
            _ => false,
        }
    }

    /// One log line, worded for `viewer` and hiding what they can't see.
    pub fn describe(&self, viewer: usize) -> String {
        if fr() {
            return self.describe_fr(viewer);
        }
        let who = |p: usize| if p == viewer { "You" } else { "Opponent" };
        let whose = |p: usize| if p == viewer { "Your" } else { "Opponent's" };
        let their = |p: usize| if p == viewer { "your" } else { "their" };
        let hidden = self.is_private(viewer);
        match self {
            Event::Shuffled { player } => {
                format!("{} shuffled {} deck", who(*player), their(*player))
            }
            Event::CardDrawn { player, .. } if hidden => format!("{} drew a card", who(*player)),
            Event::CardDrawn { player, card } => format!("{} drew {}", who(*player), name(*card)),
            Event::Mulligan { player, hand } => {
                let cards: Vec<_> = hand.iter().map(|&id| name(id)).collect();
                format!(
                    "{} had no Basic creature and redrew (revealed: {})",
                    who(*player),
                    cards.join(", ")
                )
            }
            Event::SetupRestarted => {
                "Neither player had a Basic creature: setup starts over".to_string()
            }
            Event::Placed {
                player, slot, card, ..
            } => {
                let place = if *slot == 0 {
                    "as Active"
                } else {
                    "on the Bench"
                };
                format!("{} put {} into play {place}", who(*player), name(*card))
            }
            Event::PrizesSet { player, count } => format!(
                "{} set aside {}",
                who(*player),
                plural(*count, "prize card")
            ),
            Event::CoinFlip { player, heads } => format!(
                "{} flipped a coin: {}",
                who(*player),
                if *heads { "heads" } else { "tails" }
            ),
            Event::FirstPlayer { player } => format!("{} will go first", who(*player)),
            Event::TurnStarted { player, turn } => {
                format!("Turn {}: {} turn", turn + 1, whose(*player))
            }
            Event::EnergyAttached {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} attached {} to {}",
                who(*player),
                name(*card),
                name(*creature)
            ),
            Event::Evolved {
                player, slot, card, ..
            } => format!(
                "{} {} evolved into {}",
                whose(*player),
                slot_label(*slot),
                name(*card)
            ),
            Event::TrainerPlayed { player, card } => {
                format!("{} played {}", who(*player), name(*card))
            }
            Event::EnergyDiscarded {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} {} discarded {}",
                whose(*player),
                name(*creature),
                name(*card)
            ),
            Event::Retreated {
                player, creature, ..
            } => format!(
                "{} retreated: {} is now Active",
                who(*player),
                name(*creature)
            ),
            Event::RetreatFailed { player } => {
                format!("{} failed to retreat (confused)", who(*player))
            }
            Event::AttackUsed {
                player,
                attack,
                creature,
                ..
            } => {
                let attack = card(*creature)
                    .creature()
                    .and_then(|c| c.attacks.get(*attack))
                    .map_or("an attack", |a| a.label());
                format!("{} {} used {attack}", whose(*player), name(*creature))
            }
            Event::AttackFailed { player } => format!("{} attack failed", whose(*player)),
            Event::AttackNegated { player } => {
                format!("{} Active is protected from the attack", whose(*player))
            }
            Event::PowerUsed {
                player, creature, ..
            } => {
                let power = card(*creature)
                    .creature()
                    .and_then(|c| c.power.as_ref())
                    .map_or("its Power", |p| p.label());
                format!("{} {} used {power}", whose(*player), name(*creature))
            }
            Event::ConditionApplied { player, status } => {
                format!("{} Active is now {status}", whose(*player))
            }
            Event::ConditionRemoved { player, status } => {
                format!("{} Active is no longer {status}", whose(*player))
            }
            Event::EffectApplied { player, effect } => {
                let p = *player;
                let active = if p == viewer {
                    "your Active"
                } else {
                    "the opponent's Active"
                };
                let next = |q: usize| format!("During {} next turn", their(q));
                match *effect {
                    Effect::Protect { shield, .. } => format!(
                        "{}, {}",
                        next(1 - p),
                        shield.to_string().replace("this creature", active)
                    ),
                    Effect::Place { placed, .. } => format!(
                        "{}, {}",
                        next(p),
                        placed.to_string().replace("the defending creature", active)
                    ),
                    Effect::DisableAttack => {
                        format!("{}, one of {active}'s attacks can't be used", next(p))
                    }
                    Effect::TrainerLock => format!(
                        "{} can't play trainer cards during {} next turn",
                        who(p),
                        their(p)
                    ),
                    Effect::DefenderWeakness(c) => {
                        format!("{} Active's weakness is now {c}", whose(p))
                    }
                    Effect::OwnResistance(c) => {
                        format!("{} Active's resistance is now {c}", whose(p))
                    }
                    Effect::DoubleNextAttack => {
                        format!("{} Active's next attack does double damage", whose(p))
                    }
                    Effect::SetOwnColor(c) => format!("{} creature is now {c}", whose(p)),
                    _ => format!("On {} side: {effect}", their(p)),
                }
            }
            Event::Healed {
                player,
                creature,
                amount,
                ..
            } => format!(
                "{} {} healed {amount} damage",
                whose(*player),
                name(*creature)
            ),
            Event::Switched {
                player, creature, ..
            } => format!(
                "{} {} was switched in as Active",
                whose(*player),
                name(*creature)
            ),
            Event::Devolved {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} {} devolved into {}",
                whose(*player),
                name(*card),
                name(*creature)
            ),
            Event::TrainerAttached {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} attached {} to {}",
                who(*player),
                name(*card),
                name(*creature)
            ),
            Event::TrainerDiscarded {
                player,
                creature,
                card,
                ..
            } => format!(
                "{} {} was discarded from {}",
                whose(*player),
                name(*card),
                name(*creature)
            ),
            Event::HandDiscarded { player, cards } => format!(
                "{} discarded {} hand ({})",
                who(*player),
                their(*player),
                plural(cards.len() as u32, "card")
            ),
            Event::CardToHand { player, .. } if hidden => {
                format!("{} took a card from {} deck", who(*player), their(*player))
            }
            Event::CardToHand {
                player,
                card,
                from_deck,
            } => format!(
                "{} took {} from {} {}",
                who(*player),
                name(*card),
                their(*player),
                if *from_deck { "deck" } else { "discard pile" }
            ),
            Event::Damage {
                player,
                creature,
                amount,
                weakness,
                resistance,
                ..
            } => {
                let mut s = format!(
                    "{} {} took {amount} damage",
                    whose(*player),
                    name(*creature)
                );
                if *weakness {
                    s.push_str(" (weakness)");
                }
                if *resistance {
                    s.push_str(" (resistance)");
                }
                s
            }
            Event::KnockedOut { player, card, .. } => {
                format!("{} {} was knocked out", whose(*player), name(*card))
            }
            Event::PrizeTaken { player, .. } if hidden => {
                format!("{} took a prize card", who(*player))
            }
            Event::PrizeTaken { player, card, .. } => {
                format!("{} took a prize card: {}", who(*player), name(*card))
            }
            Event::Promoted {
                player, creature, ..
            } => format!("{} {} is the new Active", whose(*player), name(*creature)),
            Event::TurnEnded { player } => format!("{} turn ended", whose(*player)),
            Event::BetweenTurns => "Between turns".to_string(),
            Event::DeckOut { player } => format!("{} couldn't draw a card", who(*player)),
            Event::SuddenDeath => {
                "Sudden death: the duel restarts with 1 prize card each".to_string()
            }
            Event::GameOver { winner } if *winner == viewer => "You win!".to_string(),
            Event::GameOver { .. } => "Opponent wins".to_string(),
        }
    }

    /// [`Event::describe`] in French: whole sentences per side, since French
    /// conjugates ("Vous piochez" / "L'adversaire pioche").
    fn describe_fr(&self, viewer: usize) -> String {
        // Your thing or theirs: `pick(p, mine, theirs)`.
        let pick = |p: usize, a: String, b: String| if p == viewer { a } else { b };
        // "Votre Braisillon" / "Braisillon adverse", starting a sentence.
        let crea = |p: usize, id: CardId| {
            if p == viewer {
                format!("Votre {}", name(id))
            } else {
                format!("{} adverse", name(id))
            }
        };
        // The Active, mid-sentence.
        let active = |p: usize| {
            if p == viewer {
                "votre Active"
            } else {
                "l'Active adverse"
            }
        };
        let hidden = self.is_private(viewer);
        match self {
            Event::Shuffled { player } => pick(
                *player,
                "Vous mélangez votre deck".into(),
                "L'adversaire mélange son deck".into(),
            ),
            Event::CardDrawn { player, .. } if hidden => pick(
                *player,
                "Vous piochez une carte".into(),
                "L'adversaire pioche une carte".into(),
            ),
            Event::CardDrawn { player, card } => pick(
                *player,
                format!("Vous piochez {}", name(*card)),
                format!("L'adversaire pioche {}", name(*card)),
            ),
            Event::Mulligan { player, hand } => {
                let cards: Vec<_> = hand.iter().map(|&id| name(id)).collect();
                let who = pick(*player, "Vous n'avez".into(), "L'adversaire n'a".into());
                format!(
                    "{who} aucune créature de Base : nouvelle main (révélée : {})",
                    cards.join(", ")
                )
            }
            Event::SetupRestarted => {
                "Aucun joueur n'a de créature de Base : la mise en place recommence".to_string()
            }
            Event::Placed {
                player, slot, card, ..
            } => {
                let c = name(*card);
                match (*player == viewer, *slot == 0) {
                    (true, true) => format!("Vous placez {c} en Active"),
                    (true, false) => format!("Vous placez {c} sur votre Banc"),
                    (false, true) => format!("L'adversaire place {c} en Active"),
                    (false, false) => format!("L'adversaire place {c} sur son Banc"),
                }
            }
            Event::PrizesSet { player, count } => {
                let n = pluriel(*count, "Récompense");
                pick(
                    *player,
                    format!("Vous mettez de côté {n}"),
                    format!("L'adversaire met de côté {n}"),
                )
            }
            Event::CoinFlip { player, heads } => {
                let side = if *heads { "face" } else { "pile" };
                pick(
                    *player,
                    format!("Vous lancez une pièce : {side}"),
                    format!("L'adversaire lance une pièce : {side}"),
                )
            }
            Event::FirstPlayer { player } => pick(
                *player,
                "Vous commencez".into(),
                "L'adversaire commence".into(),
            ),
            Event::TurnStarted { player, turn } => pick(
                *player,
                format!("Tour {} : à vous", turn + 1),
                format!("Tour {} : à l'adversaire", turn + 1),
            ),
            Event::EnergyAttached {
                player,
                creature,
                card,
                ..
            }
            | Event::TrainerAttached {
                player,
                creature,
                card,
                ..
            } => {
                let (c, to) = (name(*card), name(*creature));
                pick(
                    *player,
                    format!("Vous attachez {c} à {to}"),
                    format!("L'adversaire attache {c} à {to}"),
                )
            }
            Event::Evolved {
                player, slot, card, ..
            } => {
                let who = match (*player == viewer, *slot) {
                    (true, 0) => "Votre Active".to_string(),
                    (false, 0) => "L'Active adverse".to_string(),
                    (true, s) => format!("Votre créature en Banc {s}"),
                    (false, s) => format!("La créature adverse en Banc {s}"),
                };
                format!("{who} évolue en {}", name(*card))
            }
            Event::TrainerPlayed { player, card } => pick(
                *player,
                format!("Vous jouez {}", name(*card)),
                format!("L'adversaire joue {}", name(*card)),
            ),
            Event::EnergyDiscarded {
                player,
                creature,
                card,
                ..
            } => format!("{} défausse {}", crea(*player, *creature), name(*card)),
            Event::Retreated {
                player, creature, ..
            } => {
                let c = name(*creature);
                pick(
                    *player,
                    format!("Vous battez en retraite : {c} passe en Active"),
                    format!("L'adversaire bat en retraite : {c} passe en Active"),
                )
            }
            Event::RetreatFailed { player } => pick(
                *player,
                "Votre retraite échoue (confusion)".into(),
                "La retraite adverse échoue (confusion)".into(),
            ),
            Event::AttackUsed {
                player,
                attack,
                creature,
                ..
            } => {
                let attack = card(*creature)
                    .creature()
                    .and_then(|c| c.attacks.get(*attack))
                    .map_or("une attaque", |a| a.label());
                format!("{} utilise {attack}", crea(*player, *creature))
            }
            Event::AttackFailed { player } => pick(
                *player,
                "Votre attaque échoue".into(),
                "L'attaque adverse échoue".into(),
            ),
            Event::AttackNegated { player } => pick(
                *player,
                "Votre Active est protégée de l'attaque".into(),
                "L'Active adverse est protégée de l'attaque".into(),
            ),
            Event::PowerUsed {
                player, creature, ..
            } => {
                let power = card(*creature)
                    .creature()
                    .and_then(|c| c.power.as_ref())
                    .map_or("son Pouvoir", |p| p.label());
                format!("{} utilise {power}", crea(*player, *creature))
            }
            Event::ConditionApplied { player, status } => pick(
                *player,
                format!("Votre Active est maintenant {status}"),
                format!("L'Active adverse est maintenant {status}"),
            ),
            Event::ConditionRemoved { player, status } => pick(
                *player,
                format!("Votre Active n'est plus {status}"),
                format!("L'Active adverse n'est plus {status}"),
            ),
            Event::EffectApplied { player, effect } => {
                let p = *player;
                let next = |q: usize| {
                    pick(
                        q,
                        "Pendant votre prochain tour".into(),
                        "Pendant son prochain tour".into(),
                    )
                };
                match *effect {
                    Effect::Protect { shield, .. } => format!(
                        "{}, {}",
                        pick(
                            1 - p,
                            "Pendant votre prochain tour".into(),
                            "Pendant le prochain tour adverse".into(),
                        ),
                        shield.to_string().replace("cette créature", active(p))
                    ),
                    Effect::Place { placed, .. } => format!(
                        "{}, {}",
                        next(p),
                        placed
                            .to_string()
                            .replace("la créature en défense", active(p))
                    ),
                    Effect::DisableAttack => format!(
                        "{}, une des attaques de {} est bloquée",
                        next(p),
                        active(p)
                    ),
                    Effect::TrainerLock => pick(
                        p,
                        "Vous ne pouvez pas jouer de cartes Dresseur pendant votre prochain tour"
                            .into(),
                        "L'adversaire ne peut pas jouer de cartes Dresseur pendant son prochain tour"
                            .into(),
                    ),
                    Effect::DefenderWeakness(c) => {
                        format!("La Faiblesse de {} est maintenant {c}", active(p))
                    }
                    Effect::OwnResistance(c) => {
                        format!("La Résistance de {} est maintenant {c}", active(p))
                    }
                    Effect::DoubleNextAttack => format!(
                        "La prochaine attaque de {} inflige le double de dégâts",
                        active(p)
                    ),
                    Effect::SetOwnColor(c) => pick(
                        p,
                        format!("Votre Active devient une créature {c}"),
                        format!("L'Active adverse devient une créature {c}"),
                    ),
                    _ => pick(
                        p,
                        format!("De votre côté : {effect}"),
                        format!("Du côté adverse : {effect}"),
                    ),
                }
            }
            Event::Healed {
                player,
                creature,
                amount,
                ..
            } => format!("{} récupère {amount} PV", crea(*player, *creature)),
            Event::Switched {
                player, creature, ..
            }
            | Event::Promoted {
                player, creature, ..
            } => format!("{} passe en Active", crea(*player, *creature)),
            Event::Devolved {
                player,
                creature,
                card,
                ..
            } => format!("{} régresse en {}", crea(*player, *card), name(*creature)),
            Event::TrainerDiscarded {
                player,
                creature,
                card,
                ..
            } => {
                let on = if *player == viewer {
                    format!("votre {}", name(*creature))
                } else {
                    format!("{} adverse", name(*creature))
                };
                format!("{} part à la défausse ({on})", name(*card))
            }
            Event::HandDiscarded { player, cards } => {
                let n = pluriel(cards.len() as u32, "carte");
                pick(
                    *player,
                    format!("Vous défaussez votre main ({n})"),
                    format!("L'adversaire défausse sa main ({n})"),
                )
            }
            Event::CardToHand { player, .. } if hidden => pick(
                *player,
                "Vous prenez une carte dans votre deck".into(),
                "L'adversaire prend une carte dans son deck".into(),
            ),
            Event::CardToHand {
                player,
                card,
                from_deck,
            } => {
                let c = name(*card);
                match (*player == viewer, *from_deck) {
                    (true, true) => format!("Vous prenez {c} dans votre deck"),
                    (true, false) => format!("Vous prenez {c} dans votre défausse"),
                    (false, true) => format!("L'adversaire prend {c} dans son deck"),
                    (false, false) => format!("L'adversaire prend {c} dans sa défausse"),
                }
            }
            Event::Damage {
                player,
                creature,
                amount,
                weakness,
                resistance,
                ..
            } => {
                let who = crea(*player, *creature);
                let mut s = if *amount == 0 {
                    format!("{who} ne subit aucun dégât")
                } else {
                    format!("{who} subit {amount} dégâts")
                };
                if *weakness {
                    s.push_str(" (Faiblesse)");
                }
                if *resistance {
                    s.push_str(" (Résistance)");
                }
                s
            }
            Event::KnockedOut { player, card, .. } => {
                format!("{} est K.O.", crea(*player, *card))
            }
            Event::PrizeTaken { player, .. } if hidden => pick(
                *player,
                "Vous prenez une Récompense".into(),
                "L'adversaire prend une Récompense".into(),
            ),
            Event::PrizeTaken { player, card, .. } => {
                let c = name(*card);
                pick(
                    *player,
                    format!("Vous prenez une Récompense : {c}"),
                    format!("L'adversaire prend une Récompense : {c}"),
                )
            }
            Event::TurnEnded { player } => pick(
                *player,
                "Fin de votre tour".into(),
                "Fin du tour adverse".into(),
            ),
            Event::BetweenTurns => "Entre deux tours".to_string(),
            Event::DeckOut { player } => pick(
                *player,
                "Vous ne pouvez pas piocher".into(),
                "L'adversaire ne peut pas piocher".into(),
            ),
            Event::SuddenDeath => {
                "Mort subite : le duel reprend avec 1 Récompense chacun".to_string()
            }
            Event::GameOver { winner } => {
                pick(*winner, "Vous gagnez !".into(), "L'adversaire gagne".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;
    use crate::testkit::*;

    #[test]
    fn effect_text_reads_as_rules() {
        assert_eq!(
            Effect::AddPerHeads { coins: 2, per: 10 }.to_string(),
            "Flip 2 coins. This attack does 10 more damage for each heads."
        );
        assert_eq!(
            Effect::Inflict {
                status: Status::Paralyzed,
                coin: true
            }
            .to_string(),
            "Flip a coin. If heads, the defending creature is now Paralyzed."
        );
        assert_eq!(Effect::Draw(1).to_string(), "Draw 1 card.");
    }

    #[test]
    fn actions_are_labelled_from_the_view() {
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E, FIELD_NOTES]);
        let v = g.view(0);
        assert_eq!(
            v.describe_action(Action::AttachEnergy { hand: 0, slot: 0 }),
            "Attach Grass Energy to Mossling (Active)"
        );
        assert_eq!(
            v.describe_action(Action::PlayTrainer(1)),
            "Play Field Notes"
        );
        assert_eq!(v.describe_action(Action::Attack(0)), "Vine Poke (10)");
        assert_eq!(v.describe_action(Action::Done), "End turn");
        assert_eq!(v.prompt.unwrap().kind.to_string(), "Your turn");
    }

    /// A lasting effect is logged as what now holds, not as the card's rules
    /// text (no "Flip a coin" after the coin has landed).
    #[test]
    fn effect_events_read_as_what_happened() {
        let protect = Event::EffectApplied {
            player: 1,
            effect: Effect::Protect {
                shield: Shield::PreventDamage,
                coin: true,
            },
        };
        assert_eq!(
            protect.describe(0),
            "During your next turn, prevent all damage done to the opponent's Active by attacks"
        );
        let jaw = Event::EffectApplied {
            player: 0,
            effect: Effect::Place {
                placed: Placed::CantRetreat,
                coin: false,
            },
        };
        assert_eq!(
            jaw.describe(0),
            "During your next turn, your Active can't retreat"
        );
        assert_eq!(
            jaw.describe(1),
            "During their next turn, the opponent's Active can't retreat"
        );
        let lock = Event::EffectApplied {
            player: 0,
            effect: Effect::TrainerLock,
        };
        assert_eq!(
            lock.describe(0),
            "You can't play trainer cards during your next turn"
        );
    }

    /// French reads as French sentences per side, with French plurals and
    /// the log's Active put in place of the rules text's creature.
    #[test]
    fn french_text_reads_as_french() {
        use crate::lang::{set_lang, Lang};
        set_lang(Lang::Fr);
        let mut g = main_phase(0);
        set_hand(&mut g, 0, &[GRASS_E]);
        let v = g.view(0);
        let attach = v.describe_action(Action::AttachEnergy { hand: 0, slot: 0 });
        let done = v.describe_action(Action::Done);
        let title = v.prompt.unwrap().kind.to_string();
        let draw = Event::CardDrawn {
            player: 1,
            card: MOSSLING,
        };
        let jaw = Event::EffectApplied {
            player: 0,
            effect: Effect::Place {
                placed: Placed::CantRetreat,
                coin: false,
            },
        };
        let protect = Event::EffectApplied {
            player: 1,
            effect: Effect::Protect {
                shield: Shield::PreventDamage,
                coin: true,
            },
        };
        let got = [
            Effect::Draw(1).to_string(),
            Effect::Draw(2).to_string(),
            Effect::Inflict {
                status: Status::Paralyzed,
                coin: true,
            }
            .to_string(),
            draw.describe(0),
            jaw.describe(0),
            jaw.describe(1),
            protect.describe(0),
            attach,
            done,
            title,
        ];
        set_lang(Lang::En);
        assert_eq!(
            got,
            [
                "Piochez 1 carte.",
                "Piochez 2 cartes.",
                "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée.",
                "L'adversaire pioche une carte",
                "Pendant votre prochain tour, votre Active ne peut pas battre en retraite",
                "Pendant son prochain tour, l'Active adverse ne peut pas battre en retraite",
                "Pendant votre prochain tour, l'Active adverse ne subit aucun dégât des attaques",
                "Attacher Grass Energy à Mossling (Active)",
                "Terminer le tour",
                "Votre tour",
            ]
        );
    }

    #[test]
    fn opponent_draws_and_prizes_are_hidden_in_the_log() {
        let draw = Event::CardDrawn {
            player: 1,
            card: MOSSLING,
        };
        assert!(draw.is_private(0) && !draw.is_private(1));
        assert_eq!(draw.describe(0), "Opponent drew a card");
        assert_eq!(draw.describe(1), "You drew Mossling");
        let prize = Event::PrizeTaken {
            player: 0,
            prize: 2,
            card: FIRE_E,
        };
        assert_eq!(prize.describe(1), "Opponent took a prize card");
    }
}
