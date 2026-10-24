//! Damage (R-DMG-*): the attack calculation, effect damage, self-damage and
//! putting damage on a creature. Values run as `i32` and are clamped to 0
//! only at the end (R-DMG-01).

use crate::action::Event;
use crate::card::{Color, PowerKind};
use crate::condition::{Placed, Shield};
use crate::effect::Effect;
use crate::game::Game;

/// The outcome of a damage calculation, for the event and the UI (R-DMG-11).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Hit {
    pub amount: u16,
    pub weakness: bool,
    pub resistance: bool,
}

/// R-DMG-07: halve, rounding down to a multiple of 10.
pub(crate) fn halve(d: i32) -> i32 {
    if d <= 0 {
        d
    } else {
        d / 2 / 10 * 10
    }
}

impl Game {
    /// R-DMG-03: the colour weakness and resistance are compared with. A
    /// Power-made colour counts only while that Power works; a Trainer-as-Basic
    /// is colour index 6 (colorless).
    pub(crate) fn color_of(&self, p: usize, slot: usize) -> Color {
        let c = self.players[p].slot(slot).expect("in play");
        if c.is_trainer_basic() {
            return Color::Colorless;
        }
        match c.color_override {
            Some(color) if self.power_works(p, slot) => color,
            _ => c.creature().color,
        }
    }

    /// R-DMG-03: the Active's changed weakness/resistance wins over the printed one.
    fn weakness_of(&self, p: usize, slot: usize) -> (Option<Color>, Option<Color>) {
        let pl = &self.players[p];
        let printed = pl.slot(slot).expect("in play").creature();
        if slot == 0 {
            (
                pl.effects.weakness.or(printed.weakness),
                pl.effects.resistance.or(printed.resistance),
            )
        } else {
            (printed.weakness, printed.resistance)
        }
    }

    /// R-DMG-01 steps 3–4: weakness ×2, then resistance −30.
    fn apply_weakness(&self, color: Color, q: usize, slot: usize, d: &mut i32, hit: &mut Hit) {
        let (weak, resist) = self.weakness_of(q, slot);
        if weak == Some(color) {
            *d *= 2;
            hit.weakness = true;
        }
        if resist == Some(color) {
            *d -= 30;
            hit.resistance = true;
        }
    }

    fn plus10(&self, p: usize) -> i32 {
        self.players[p]
            .active
            .as_ref()
            .map_or(0, |a| 10 * a.attached(Effect::AttachPlus10) as i32)
    }

    fn minus20(&self, q: usize, slot: usize) -> i32 {
        self.players[q]
            .slot(slot)
            .map_or(0, |c| 20 * c.attached(Effect::AttachMinus20) as i32)
    }

    /// R-DMG-08: the "next attack does double damage" set on an earlier turn.
    fn doubled(&self, p: usize) -> bool {
        self.players[p]
            .effects
            .doubling
            .is_some_and(|set| self.turn > set)
    }

    /// R-DMG-01: the direct damage of player `p`'s attack on the opposing Active.
    pub(crate) fn attack_damage(&self, p: usize, base: i32, ignore_wr: bool, negated: bool) -> Hit {
        let q = 1 - p;
        let mut hit = Hit::default();
        // R-DMG-02: zero stays zero, with no modifiers at all.
        if base <= 0 {
            return hit;
        }
        let mut d = base;
        if self.doubled(p) {
            d *= 2;
        }
        // R-DMG-10
        if !ignore_wr {
            self.apply_weakness(self.color_of(p, 0), q, 0, &mut d, &mut hit);
        }
        d += self.plus10(p);
        d -= self.minus20(q, 0);
        d = self.reduce(p, q, 0, d, negated, false);
        hit.amount = d.max(0) as u16;
        hit
    }

    /// R-DMG-09 steps 1–4 for damage from player `p` on the creature at (q, slot).
    fn reduce(
        &self,
        p: usize,
        q: usize,
        slot: usize,
        mut d: i32,
        negated: bool,
        power: bool,
    ) -> i32 {
        if negated {
            return 0;
        }
        let shield = if slot == 0 {
            self.players[q].effects.shield
        } else {
            None
        };
        match shield {
            Some(Shield::PreventDamage) => d = 0,
            Some(Shield::Reduce(n)) => d -= n as i32,
            Some(Shield::PreventBelow(n)) => {
                if d < n as i32 {
                    d = 0
                }
            }
            Some(Shield::Halve) => d = halve(d),
            // Only without one of those does the reduction Power apply; never
            // to Power damage.
            // RULES: R-DMG-09 — the whole-attack shield isn't one of the step-2
            // values, so the reduction Power may still apply under it.
            Some(Shield::PreventAll | Shield::CounterKo) | None => {
                if !power {
                    d = self.reduction_power(q, slot, d)
                }
            }
        }
        if let Some(Placed::Weaken(n)) = self.players[p].effects.placed {
            d -= n as i32;
        }
        d
    }

    /// R-DMG-09 (3): the holder's own reduction Power, while it works.
    fn reduction_power(&self, q: usize, slot: usize, d: i32) -> i32 {
        match self.passive(q, slot) {
            Some(PowerKind::PreventAtLeast(n)) if d >= n as i32 => 0,
            Some(PowerKind::HalveDamage) => halve(d),
            _ => d,
        }
    }

    /// R-DMG-04: effect damage from player `p` to (q, slot): never weakness or
    /// resistance; +10 trainers on an Active target; −20 trainers except for
    /// Power damage; the Active's reductions; the Bench's reduction Power after
    /// the clamp.
    pub(crate) fn effect_damage(
        &self,
        p: usize,
        q: usize,
        slot: usize,
        amount: u16,
        power: bool,
    ) -> Hit {
        let mut d = amount as i32;
        if slot == 0 {
            d += self.plus10(p);
        }
        if !power {
            d -= self.minus20(q, slot);
        }
        if slot == 0 {
            d = self.reduce(p, q, 0, d, false, power);
        }
        d = d.max(0);
        if slot != 0 && !power {
            d = self.reduction_power(q, slot, d);
        }
        Hit {
            amount: d as u16,
            ..Hit::default()
        }
    }

    /// R-DMG-05: damage to its own Active (confusion, recoil): own weakness and
    /// resistance against its own colour, own trainers, clamp. Never prevented.
    pub(crate) fn self_damage(&self, p: usize, amount: u16) -> Hit {
        let mut hit = Hit::default();
        let mut d = amount as i32;
        self.apply_weakness(self.color_of(p, 0), p, 0, &mut d, &mut hit);
        d += self.plus10(p);
        d -= self.minus20(p, 0);
        hit.amount = d.max(0) as u16;
        hit
    }

    /// R-DMG-06: subtract, flooring HP at 0. `retaliate`: attack or effect
    /// damage, which sets off a retaliation Power (R-DMG-14).
    pub(crate) fn hurt(
        &mut self,
        q: usize,
        slot: usize,
        hit: Hit,
        retaliate: bool,
        ev: &mut Vec<Event>,
    ) {
        let Some(c) = self.players[q].slot_mut(slot) else {
            return;
        };
        c.hp = c.hp.saturating_sub(hit.amount);
        ev.push(Event::Damage {
            player: q,
            slot,
            amount: hit.amount,
            weakness: hit.weakness,
            resistance: hit.resistance,
        });
        // RULES: R-DMG-14 — one hook, at the damage itself, for any slot; the
        // second after-phase-4 hook is not modelled separately. Self-damage,
        // poison and Power damage never retaliate.
        if retaliate && hit.amount > 0 && self.passive(q, slot) == Some(PowerKind::Retaliate) {
            self.raw_damage(1 - q, 10, ev);
        }
    }

    /// Damage with no modifiers at all on an Active (poison, retaliation).
    pub(crate) fn raw_damage(&mut self, q: usize, amount: u16, ev: &mut Vec<Event>) {
        let hit = Hit {
            amount,
            ..Hit::default()
        };
        self.hurt(q, 0, hit, false, ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Event;
    use crate::condition::Special;
    use crate::effect::Effect::*;
    use crate::fixtures::*;
    use crate::testkit::*;
    use Color::*;

    /// Player 0's Mossling (Grass) attacks player 1's Pebblet (weak to Grass, 60 HP).
    fn duel() -> Game {
        main_phase(0)
    }

    fn attach_trainer(g: &mut Game, p: usize, slot: usize, card: crate::CardId) {
        g.players[p].slot_mut(slot).unwrap().trainers.push(card);
    }

    /// R-DMG-01
    #[test]
    fn damage_order_weakness_then_resistance_then_trainers() {
        let mut g = duel();
        g.players[1].effects.resistance = Some(Grass);
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        attach_trainer(&mut g, 1, 0, BUCKLER);
        let ev = attack_with(&mut g, 30, &[]);
        // 30 ×2 = 60, −30 = 30, +10 = 40, −20 = 20.
        assert_eq!(damage_to(&ev, 1, 0), vec![20]);
        assert_eq!(hp(&g, 1, 0), 40);
    }

    /// R-DMG-01: only the end result is clamped.
    #[test]
    fn resisted_10_with_plus10_is_zero() {
        let mut g = duel();
        g.players[1].effects.weakness = Some(Water);
        g.players[1].effects.resistance = Some(Grass);
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        let ev = attack_with(&mut g, 10, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![0]);
        assert_eq!(hp(&g, 1, 0), 60);
    }

    /// R-DMG-02
    #[test]
    fn zero_base_damage_ignores_weakness_and_plus10() {
        let mut g = duel();
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        let ev = attack_with(&mut g, 0, &[]);
        assert!(damage_to(&ev, 1, 0).is_empty(), "no direct damage at all");
        // The attacker step leaving it at 0 (all tails) also skips everything.
        let mut g = duel();
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        g.force_coins(&[false, false]);
        let ev = attack_with(&mut g, 0, &[AddPerHeads { coins: 2, per: 10 }]);
        assert_eq!(damage_to(&ev, 1, 0), vec![0]);
    }

    /// R-DMG-03
    #[test]
    fn changed_weakness_overrides_printed() {
        let mut g = duel();
        g.players[1].effects.weakness = Some(Fire);
        let ev = attack_with(&mut g, 20, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![20]);
        assert!(!ev
            .iter()
            .any(|e| matches!(e, Event::Damage { weakness: true, .. })));
    }

    /// R-DMG-03
    #[test]
    fn weak_and_resistant_same_type_x2_then_minus30() {
        let mut g = duel();
        g.players[1].effects.resistance = Some(Grass);
        let ev = attack_with(&mut g, 20, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![10]);
    }

    /// R-DMG-03: a Power-made colour counts only while that Power works, and a
    /// Trainer-as-Basic is colorless.
    #[test]
    fn changed_attacker_colour_needs_working_power() {
        let mut g = duel();
        g.players[1].active = Some(creature(MOSSLING, &[])); // weak to Fire
        g.players[0].active = Some(creature(SHIFTER, &[]));
        g.players[0].active.as_mut().unwrap().color_override = Some(Fire);
        assert_eq!(g.color_of(0, 0), Fire);
        let ev = attack_with(&mut g, 10, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![20]);
        let mut g = duel();
        g.players[0].active = Some(creature(SHIFTER, &[]));
        g.players[0].active.as_mut().unwrap().color_override = Some(Fire);
        g.players[0].condition.apply_special(Special::Confused);
        assert_eq!(g.color_of(0, 0), Colorless, "a condition stops the Power");
        g.players[0].condition = Default::default();
        g.players[1].bench.push(creature(HUSHMOTH, &[]));
        assert_eq!(g.color_of(0, 0), Colorless, "so does a blocker");
        g.players[0].active = Some(creature(STRAW_DECOY, &[]));
        assert_eq!(g.color_of(0, 0), Colorless);
    }

    /// R-DMG-04, R-EFFECT-04
    #[test]
    fn bench_effect_damage_ignores_weakness() {
        let mut g = duel();
        g.players[1].bench.push(creature(PEBBLET, &[])); // weak to Grass
        g.players[0].bench.push(creature(EMBERKIT, &[]));
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        let ev = attack_with(
            &mut g,
            0,
            &[BenchDamage {
                amount: 20,
                own: true,
                opponent: true,
            }],
        );
        assert_eq!(
            damage_to(&ev, 1, 1),
            vec![20],
            "no weakness, no +10 on the Bench"
        );
        assert_eq!(damage_to(&ev, 0, 1), vec![20]);
        assert!(damage_to(&ev, 1, 0).is_empty(), "Actives are not hit");
    }

    /// R-DMG-04
    #[test]
    fn bench_minus20_trainer_reduces_effect_damage() {
        let mut g = duel();
        g.players[1].bench.push(creature(PEBBLET, &[]));
        attach_trainer(&mut g, 1, 1, BUCKLER);
        let ev = attack_with(
            &mut g,
            0,
            &[BenchDamage {
                amount: 30,
                own: false,
                opponent: true,
            }],
        );
        assert_eq!(damage_to(&ev, 1, 1), vec![10]);
    }

    /// R-DMG-04: effect damage to an Active adds +10 and goes through its
    /// reductions; a Benched creature's reduction Power applies after the
    /// clamp, but never to Power damage.
    #[test]
    fn effect_damage_active_and_bench_rules() {
        let mut g = duel();
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        g.players[1].effects.shield = Some(Shield::Reduce(10));
        assert_eq!(
            g.effect_damage(0, 1, 0, 20, false).amount,
            20,
            "20 +10 −10, no weakness"
        );
        g.players[1].bench.push(creature(BULWARK, &[]));
        attach_trainer(&mut g, 1, 1, BUCKLER);
        assert_eq!(
            g.effect_damage(0, 1, 1, 50, false).amount,
            0,
            "50 −20 = 30: prevented"
        );
        assert_eq!(
            g.effect_damage(0, 1, 1, 50, true).amount,
            50,
            "Power damage: no −20, no Power"
        );
    }

    /// R-DMG-05
    #[test]
    fn confusion_self_hit_weak_to_own_type_takes_40() {
        let mut g = duel();
        g.players[0].condition.apply_special(Special::Confused);
        g.players[0].effects.weakness = Some(Grass);
        g.force_coins(&[false]);
        let ev = attack_with(&mut g, 30, &[]);
        assert_eq!(damage_to(&ev, 0, 0), vec![40]);
        assert!(damage_to(&ev, 1, 0).is_empty());
        assert_eq!(hp(&g, 0, 0), 10);
    }

    /// R-DMG-05
    #[test]
    fn confusion_self_hit_resists_own_type_takes_0() {
        let mut g = duel();
        g.players[0].condition.apply_special(Special::Confused);
        g.players[0].effects.resistance = Some(Grass);
        g.players[0].effects.shield = None;
        g.force_coins(&[false]);
        let ev = attack_with(&mut g, 30, &[]);
        assert_eq!(damage_to(&ev, 0, 0), vec![0]);
    }

    /// R-DMG-05: self-damage can't be prevented and uses its own trainers.
    #[test]
    fn recoil_uses_own_trainers_and_ignores_shields() {
        let mut g = duel();
        g.players[0].effects.shield = Some(Shield::PreventDamage);
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        let ev = attack_with(&mut g, 10, &[Recoil(20)]);
        assert_eq!(damage_to(&ev, 0, 0), vec![30]);
    }

    /// R-DMG-06
    #[test]
    fn damage_equal_to_hp_knocks_out() {
        let mut g = duel();
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        g.players[1].active.as_mut().unwrap().hp = 20;
        let ev = attack_with(&mut g, 10, &[]);
        assert!(ev.contains(&Event::KnockedOut {
            player: 1,
            slot: 0,
            card: PEBBLET
        }));
        let mut g = duel();
        g.players[1].active.as_mut().unwrap().hp = 30;
        attack_with(&mut g, 10, &[]);
        assert_eq!(hp(&g, 1, 0), 10, "one point short is not a knock-out");
    }

    /// R-DMG-06
    #[test]
    fn overkill_clamps_hp_to_zero() {
        let mut g = duel();
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        let hit = Hit {
            amount: 200,
            ..Hit::default()
        };
        g.hurt(1, 1, hit, false, &mut Vec::new());
        assert_eq!(hp(&g, 1, 1), 0);
    }

    /// R-DMG-07
    #[test]
    fn halve_damage_rounds_down_to_ten() {
        assert_eq!([30, 50, 60, 10, 0].map(halve), [10, 20, 30, 0, 0]);
        let mut g = duel();
        g.players[1].effects.shield = Some(Shield::Halve);
        let ev = attack_with(&mut g, 25, &[]); // 50 after weakness
        assert_eq!(damage_to(&ev, 1, 0), vec![20]);
    }

    /// R-DMG-08
    #[test]
    fn double_damage_then_weakness_then_resistance() {
        let mut g = main_phase(2);
        g.players[0].effects.doubling = Some(0);
        g.players[1].effects.resistance = Some(Grass);
        let ev = attack_with(&mut g, 20, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![50], "20 ×2 ×2 −30");
    }

    /// R-DMG-09: the own-turn shield comes after weakness.
    #[test]
    fn defender_minus20_applies_after_weakness() {
        let mut g = duel();
        g.players[1].effects.shield = Some(Shield::Reduce(20));
        let ev = attack_with(&mut g, 20, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![20], "20 ×2 −20");
    }

    /// R-DMG-09: one shield, else the reduction Power, then the attacker's weaken.
    #[test]
    fn defender_reductions_exclude_each_other() {
        let hit = |shield: Option<Shield>, defender, base| {
            let mut g = duel();
            g.players[1].active = Some(creature(defender, &[]));
            g.players[1].effects.shield = shield;
            g.attack_damage(0, base, true, false).amount
        };
        assert_eq!(hit(Some(Shield::PreventDamage), PEBBLET, 50), 0);
        assert_eq!(hit(Some(Shield::PreventBelow(40)), PEBBLET, 30), 0);
        assert_eq!(hit(Some(Shield::PreventBelow(40)), PEBBLET, 40), 40);
        assert_eq!(hit(None, BULWARK, 30), 0);
        assert_eq!(hit(None, BULWARK, 20), 20);
        assert_eq!(hit(None, HALVER, 50), 20);
        assert_eq!(
            hit(Some(Shield::Reduce(10)), BULWARK, 40),
            30,
            "the shield wins over the Power"
        );
        let mut g = duel();
        g.players[0].effects.placed = Some(Placed::Weaken(20));
        assert_eq!(g.attack_damage(0, 30, true, false).amount, 10);
    }

    /// R-DMG-10
    #[test]
    fn ignore_wr_flag_skips_weakness_keeps_plus10() {
        let mut g = duel();
        attach_trainer(&mut g, 0, 0, WHETSTONE);
        let ev = attack_with(&mut g, 20, &[IgnoreWeaknessResistance]);
        assert_eq!(damage_to(&ev, 1, 0), vec![30]);
    }

    /// R-DMG-11
    #[test]
    fn attack_result_reports_weakness_and_resistance_flags() {
        let mut g = duel();
        g.players[1].effects.resistance = Some(Grass);
        let ev = attack_with(&mut g, 30, &[]);
        assert!(ev.contains(&Event::Damage {
            player: 1,
            slot: 0,
            amount: 30,
            weakness: true,
            resistance: true,
        }));
    }

    /// R-DMG-12
    #[test]
    fn prevention_substatus_negates_damage_and_effects() {
        let mut g = duel();
        g.players[1].effects.shield = Some(Shield::PreventAll);
        g.players[1].bench.push(creature(EMBERKIT, &[]));
        attack_with(
            &mut g,
            30,
            &[
                Inflict {
                    status: crate::Status::Poisoned,
                    coin: false,
                },
                Snipe(10),
            ],
        );
        let ev = ok(&mut g, crate::Action::Choose(0)); // Snipe the Active
        assert!(ev.contains(&Event::AttackNegated { player: 1 }));
        assert_eq!(damage_to(&ev, 1, 0), vec![0], "the Snipe is cancelled too");
        assert!(g.players[1].condition.is_empty());
        // A protected creature from an evolved attacker (Power), but not from a Basic.
        let mut g = main_phase(2);
        g.players[1].active = Some(creature(MIRAGE, &[]));
        assert_eq!(damage_to(&attack_with(&mut g, 20, &[]), 1, 0), vec![20]);
        let mut g = main_phase(2);
        g.players[1].active = Some(creature(MIRAGE, &[]));
        g.players[0].active.as_mut().unwrap().stack.push(MOSSBACK);
        assert_eq!(damage_to(&attack_with(&mut g, 20, &[]), 1, 0), vec![0]);
    }

    /// R-DMG-12: skipped for attacks that do no direct damage.
    #[test]
    fn prevention_skipped_without_direct_damage() {
        let mut g = duel();
        g.players[1].effects.shield = Some(Shield::PreventAll);
        let ev = attack_with(
            &mut g,
            0,
            &[Inflict {
                status: crate::Status::Asleep,
                coin: false,
            }],
        );
        assert!(ev.contains(&Event::ConditionApplied {
            player: 1,
            status: crate::Status::Asleep
        }));
    }

    /// R-DMG-13
    #[test]
    fn coin_prevention_power_heads_zeroes_damage() {
        let mut g = duel();
        g.players[1].active = Some(creature(LUCKSHELL, &[]));
        g.force_coins(&[true]);
        let poison = &[Inflict {
            status: crate::Status::Poisoned,
            coin: false,
        }];
        let ev = attack_with(&mut g, 30, poison);
        assert_eq!(damage_to(&ev, 1, 0), vec![0]);
        assert!(
            g.players[1].condition.is_empty(),
            "heads also negates the effects"
        );
        let mut g = duel();
        g.players[1].active = Some(creature(LUCKSHELL, &[]));
        g.force_coins(&[false]);
        let ev = attack_with(&mut g, 30, poison);
        assert_eq!(
            damage_to(&ev, 1, 0),
            vec![30, 10],
            "the hit, then poison between turns"
        );
    }

    /// R-DMG-14
    #[test]
    fn retaliation_power_deals_flat_10() {
        let mut g = duel();
        g.players[1].active = Some(creature(THORNBACK, &[]));
        attach_trainer(&mut g, 1, 0, BUCKLER);
        let ev = attack_with(&mut g, 30, &[]);
        assert_eq!(damage_to(&ev, 1, 0), vec![10]);
        assert_eq!(damage_to(&ev, 0, 0), vec![10]);
        // No retaliation for zero damage.
        let mut g = duel();
        g.players[1].active = Some(creature(THORNBACK, &[]));
        let ev = attack_with(&mut g, 0, &[AddPerHeads { coins: 0, per: 0 }]);
        assert!(damage_to(&ev, 0, 0).is_empty());
    }
}
