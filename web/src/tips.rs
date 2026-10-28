//! First-time rule tips: the first time a rule comes up, a short note points at
//! the Hexon it concerns. Each is shown once per browser; the duel menu can
//! bring them all back.

use sixfold_engine::{tr, Status};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tip {
    FirstTurn,
    Asleep,
    Confused,
    Paralyzed,
    Poisoned,
    Retreat,
    Power,
    SuddenDeath,
}

impl Tip {
    fn bit(self) -> u32 {
        1 << self as u32
    }

    pub fn for_status(s: Status) -> Tip {
        match s {
            Status::Asleep => Tip::Asleep,
            Status::Confused => Tip::Confused,
            Status::Paralyzed => Tip::Paralyzed,
            Status::Poisoned | Status::DoublePoisoned => Tip::Poisoned,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Tip::FirstTurn => tr("Going first", "Premier à jouer"),
            Tip::Asleep => tr("Asleep", "Endormi"),
            Tip::Confused => tr("Confused", "Confus"),
            Tip::Paralyzed => tr("Paralyzed", "Paralysé"),
            Tip::Poisoned => tr("Poisoned", "Empoisonné"),
            Tip::Retreat => tr("Retreat", "Retraite"),
            Tip::Power => tr("Powers", "Pouvoirs"),
            Tip::SuddenDeath => tr("Sudden death", "Mort subite"),
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Tip::FirstTurn => tr(
                "The player who goes first can't attack on their first turn. Set up instead: Basics, energy, Gear cards.",
                "Le premier joueur ne peut pas attaquer à son premier tour. Préparez-vous\u{a0}: Hexons de Base, énergie, Objets.",
            ),
            Tip::Asleep => tr(
                "An Asleep Hexon can't attack or retreat. Between turns its owner flips a coin: heads, it wakes up.",
                "Un Hexon Endormi ne peut ni attaquer ni battre en retraite. Entre les tours, son joueur lance une pièce\u{a0}: face, il se réveille.",
            ),
            Tip::Confused => tr(
                "A Confused Hexon flips a coin to attack: tails, the attack fails and it takes 20 damage. It stays Confused until it leaves the Active spot.",
                "Un Hexon Confus lance une pièce pour attaquer\u{a0}: pile, l'attaque échoue et il subit 20 dégâts. Il reste Confus tant qu'il est Actif.",
            ),
            Tip::Paralyzed => tr(
                "A Paralyzed Hexon can't attack or retreat. It recovers at the end of its owner's next turn.",
                "Un Hexon Paralysé ne peut ni attaquer ni battre en retraite. Il se remet à la fin du prochain tour de son joueur.",
            ),
            Tip::Poisoned => tr(
                "A Poisoned Hexon takes 10 damage between every turn. It stays Poisoned until it leaves the Active spot or is cured.",
                "Un Hexon Empoisonné subit 10 dégâts entre chaque tour. Il le reste tant qu'il est Actif ou jusqu'à un soin.",
            ),
            Tip::Retreat => tr(
                "Retreat swaps your Active for a Benched Hexon. Pay its retreat cost in energy, once a turn. Special conditions wear off when it goes to the Bench.",
                "La retraite échange votre Actif contre un Hexon du Banc. Payez son coût en énergie, une fois par tour. Ses états spéciaux disparaissent sur le Banc.",
            ),
            Tip::Power => tr(
                "A Power is a Hexon's ability, used without attacking: it doesn't end your turn. An Asleep, Confused or Paralyzed Active can't use it.",
                "Un Pouvoir est une capacité du Hexon, utilisée sans attaquer\u{a0}: elle ne finit pas le tour. Un Actif Endormi, Confus ou Paralysé ne peut pas s'en servir.",
            ),
            Tip::SuddenDeath => tr(
                "Both players won at once. A new setup starts with 1 prize each: the first to take it wins.",
                "Les deux joueurs gagnent en même temps. Une nouvelle partie commence avec 1 Récompense chacun\u{a0}: le premier qui la prend gagne.",
            ),
        }
    }

    pub fn seen(self) -> bool {
        crate::store::tips_seen() & self.bit() != 0
    }

    pub fn mark_seen(self) {
        crate::store::set_tips_seen(crate::store::tips_seen() | self.bit());
    }
}

/// Show every tip again from the next time each rule comes up.
pub fn reset() {
    crate::store::set_tips_seen(0);
}
