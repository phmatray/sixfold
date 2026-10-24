# Rules Ledger

This ledger holds the mechanics the clean-room engine (`engine/`) must implement. Every rule here was
reconstructed from the reference duel engine and then checked a second time by a separate verifier.
Each rule gives its exact parameters, its status, the evidence behind it, and the name of the engine
unit test that should cover it.

- **Source.** A local reference ROM, identified only by its SHA-256:
  `30f2ec540c48dcddef205cecb4d14c3941064d8454c2dcf498dff77c32f99aae` (the European GBC release). The
  ROM is git-ignored. It is not distributed and is never committed.
- **Method.** Static, read-only disassembly using custom SM83 tooling kept in the session scratchpad.
  The REA tool could not open the Game Boy format, because it has no SM83 architecture. Each claim
  went through two passes. An investigator located code and bytes. A verifier then re-disassembled
  the same code on its own and marked the claim confirmed or corrected.
- **Legal scope.** This file describes game **rules and mechanics only**, in our own words. It
  contains no card names, attack names, flavor or UI text, art, or card data tables. Internal IDs
  (`$CB`, text `$3E`, and so on) appear only as evidence pointers. The engine must model every
  card-specific hook as a generic, original effect.

## Conventions

- **Status.** `CONFIRMED` means the verifier reproduced the claim. `CORRECTED` means the verifier
  changed the claim, and the corrected version is the one written here. `UNCERTAIN` means the rule is
  only partly traced. A rule tagged *(verifier-found)* was discovered during verification, with the
  evidence cited, and was not part of the original claims.
- **Addresses.** `BB:AAAA` means ROM bank and CPU address. Per-player duel variables appear as
  `+XX`, an offset into that player's duel page.
- **Glossary**
  - *T* is the turn player and *N* is the non-turn player (the opponent).
  - *Creature* is a character card (type index < 8). Its *stage* is 0 for Basic, 1 for Stage 1 and
    2 for Stage 2.
  - *Energy* covers basic energy (one per colour) and one *special 2-unit colorless energy*.
  - *Trainer* is a one-shot card (type `$10`). Trainers have no subtypes.
  - *Active* is slot 0. *Bench* is slots 1 to 5.
  - *Condition* means a special condition: Confused, Asleep, Paralyzed, Poisoned or Double-Poisoned.
  - *Power* is a creature ability stored in attack slot 1 with category 4. A Power is either
    *activated* (used from a menu) or *passive* (always on).
  - *Power blocker* is a passive Power that shuts down all other Powers.
  - *+10 trainer* and *−20 trainer* are trainers that stay attached to a creature and change damage.
  - *Trainer-as-Basic* is a trainer card that may be placed in play as a Basic creature.

---

## 1. Setup (R-SETUP)

### R-SETUP-01: Opening hand of seven
- **Rule.** At setup each player's state is reset, with every card back in the deck. The deck is
  shuffled and the player draws 7 cards.
- **Parameters.** 7 draws. 2 shuffle passes. The shuffle is skipped only in a scripted practice
  duel. A card counts as Basic when it is a creature (type < 8) with stage 0.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:4D96-4DB0`: reset (`01:71B4`), `call 00:1021` twice, then `ld b,7` loop of draw + add-to-hand.
  - `01:71B4`: zeroes the duel page, sets all 60 locations to deck, sets the deck order to 0..59.
  - `01:4DB1-4DED`: scans the 7 hand cards with the Basic test.
- **Test.** `setup_draws_seven_cards_each_player`

### R-SETUP-02: Forced mulligan, no bonus cards
- **Rule.** A player whose 7-card hand has no Basic must reveal it, shuffle it back and draw 7 again.
  This repeats until the hand has a Basic. The mulligan cannot be declined. The other player gets no
  extra cards. If **neither** player has a Basic, both hands are revealed and the whole setup restarts
  for both players.
- **Parameters.** 0 bonus cards. In single player the human side is checked first.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:4B7C-4B89`: ANDs and ORs the two has-Basic results. Both → placement. Neither → `4BB1`.
    Otherwise one side mulligans.
  - `01:4B8B-4B99` / `01:4B9E-4BAF`: reveal, reset, redraw loop. Then `jr 4BCF`, which goes to
    placement without drawing for the other side.
  - `01:4BB1-4BCC`: both sides lack a Basic. Both are reset and the code jumps to `4B5F` to restart.
- **Test.** `mulligan_redraws_until_basic_without_bonus_cards`, `both_players_without_basic_restart_setup`

### R-SETUP-03: Mulligan check ignores Trainer-as-Basic cards
- **Rule.** The opening-hand Basic check does not count Trainer-as-Basic cards, even though setup
  placement accepts them. Deck validation does not count them either (R-DECK-02).
- **Parameters.** The mulligan check enters at `01:4DDB` (type and stage only). The placement filter
  enters at `01:4DD0`, which also accepts the two trainer IDs.
- **Status.** CONFIRMED. The verifier raised it to high confidence: in hand these cards stay type
  `$10`, and `01:6E31` turns them into creature data only while they are in play.
- **Evidence.**
  - `01:4DC0`: `call 4DDB` from the hand scan.
  - `01:552A`: `call 4DD0`. This is the only caller of the ID-accepting entry.
  - `00:12E9 → 01:6E31`: creature data is substituted only while the card's location is in play.
- **Test.** `mulligan_ignores_trainer_as_basic`

### R-SETUP-04: Placing Active and Bench
- **Rule.** After the hands are settled, each player places exactly one Basic as Active. The human
  goes first, then the opponent. The Active choice cannot be cancelled. The player may then place 0
  to 5 more Basics on the Bench and stop at any time. Only Basics (or Trainer-as-Basic cards) are
  accepted. No confirmation prompt is shown in a normal duel.
- **Parameters.** At most 6 in play (`+EF`, `cp 6`). The placement primitive is `00:13EA`.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:4D22-4D46`: the Active pick loops until a card is chosen. With `cbfc=0`, B (cancel) loops back.
  - `01:4D5E-4D8B`: the Bench loop. Cancel ends it, and 6 in play shows the bench-full message.
  - `01:5525-5538`, `01:4DD0`: the selection filter.
  - `01:4BCF-4BE2`: player first, then opponent. `01:51ED` is a practice-duel hook only (`cc13`).
- **Test.** `setup_requires_exactly_one_basic_active`, `setup_rejects_evolution_card`, `setup_bench_limit_five`

### R-SETUP-05: Prizes are set after placement; count is 1 to 6
- **Rule.** After both players have placed their creatures, each player sets aside N prize cards from
  the top of the deck, player first. N is a per-duel setting from 1 to 6. A sudden-death duel uses 1.
  **Clean-room default: 6**, kept configurable.
- **Parameters.** `cc08 ← cc18`. The remaining-prize bitmask is `+EC = (1<<N)-1`, from table
  `00 01 03 07 0F 1F 3F`. The values observed in the source duels are {2,3,4,6}. A linked duel lets
  the players pick 2 to 6 (default 4). The placement loop is do-while, so N = 0 is unsupported.
- **Status.** CORRECTED. The range is 1 to 6, not 0 to 6, and there are more writers of `cc18`.
- **Evidence.**
  - `01:71E0-7206`: draws `cc08` cards into the prize list at `+3C` and marks them location 8.
  - `01:40DF`: `cc08 ← cc18`. `01:41E0`: `cc08 = 1` (sudden death).
  - `01:4C0C/4C12`: prizes are set after placement (`4BD6/4BDC`).
  - `cc18` writers: `03:4D67`, `04:5DFE`, `04:7236`, `06:65D4`, `06:6652`, `06:6692` (2..6 selector).
- **Test.** `prizes_set_after_placement_default_six`, `prize_count_must_be_one_to_six`

### R-SETUP-06: Coin flip for the first player
- **Rule.** One coin decides who goes first, flipped **after** placement and prizes. On heads the side
  that flipped (the human) plays first. On tails the opponent does. The winner gets no choice of going
  first or second.
- **Parameters.** One coin (`00:3034`). Carry means heads. Tails calls `SwapTurn`.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:4C20-4C50`: the coin flip. Tails calls `1BD2`.
  - `01:73A5-73AB`: carry means at least one heads.
- **Test.** `coin_flip_heads_flipper_goes_first`, `coin_flip_tails_opponent_goes_first`

---

## 2. Turn structure (R-TURN)

### R-TURN-01: Draw at the start of every turn
- **Rule.** The turn player draws 1 card at the start of **every** turn, including the first player's
  first turn.
- **Parameters.** 1 card. The turn counter is not tested around the draw.
- **Status.** CONFIRMED
- **Evidence.** `01:423E-424E`: an unconditional draw. `01:4231` is the only test of `cc06`, and it
  guards evolution, not the draw.
- **Test.** `first_player_draws_on_turn_zero`

### R-TURN-02: Deck-out happens only at the turn draw
- **Rule.** If the deck is empty at the turn draw, the turn player loses immediately, before taking
  any action. Draws caused by effects are capped at the cards left in the deck. Search effects (the
  callers of `0B:42E0`) are refused on an empty deck. Plain draw effects can still be played and draw
  nothing (`01:4939` clamps, `01:4973` message); a discard-hand-then-draw trainer still discards the
  hand. Running out because of an effect is **not** a loss. Only the turn player's own deck at their
  own draw is checked.
- **Parameters.** A draw fails when cards drawn `[BA] ≥ 60`. The result becomes `cc07 = 2`
  (turn player loses).
- **Status.** CORRECTED. The empty-deck refusal covers search effects only, not plain draws.
- **Evidence.**
  - `01:4241-424B`: `call 1034; jr nc; ld a,2; ld (cc07),a; ret`.
  - `00:1034`: `cp $3C → scf`.
  - `01:4945-4973`: the draw-N screen clamps N.
  - `0B:6B46`, `0B:73B9`: effect draw loops stop on carry and set no result.
  - `0B:42E0` (CheckDeckNotEmpty): the empty-deck precondition. Its only 6 callers (`0B:484A`, `49DB`,
    `4C40`, `4F5D`, `6100`, `6877`) are search-type effects. The draw-2 trainer (list `06:4FAC`), the
    discard-hand-draw-7 trainer (`06:4F0F`) and the draw-1 attack (`06:4DC7`) have no phase-1 entry.
  - Runtime: with `C2BA = $3C`, both draw trainers were played: `01:4973` was hit, the card (and for
    the draw-7 trainer the whole hand) went to the discard, 0 cards were drawn, `cc07` stayed 0.
  - `cc07` is written only at `4092`, `4248`, `6F79` and `7158`.
- **Test.** `empty_deck_at_turn_draw_loses`, `effect_draw_clamped_to_remaining_deck_no_loss`,
  `search_refused_on_empty_deck`

### R-TURN-03: Turn sequence
- **Rule.** Each turn runs in this order:
  1. Clear the turn player's own-turn substatus (`+E7`).
  2. Show the turn banner.
  3. If the turn counter is ≥ 2, mark every creature the turn player has in play as *settled*
     (bit 7, which allows evolution) and clear its once-per-turn Power flag (bit 5).
  4. Reset the per-turn flags (energy attached, retreat failed, accuracy coin) and record whose turn
     it is.
  5. Draw 1 card (deck-out ends the duel).
  6. Run the main phase.
  7. Check the result.
  8. Clear end-of-turn effects: the turn player's trainer lock, `+E8`, and conditionally `+EB` bit 0.
  9. Run between-turns processing.
  10. Check the result.
  11. Increment the turn counter.
  12. Swap the turn.
- **Parameters.** The turn counter `cc06` starts at 0 and is shared by both players. Step 3 is gated
  by `cp 2; jr c`. The per-turn reset clears `cc0b`, `cc0c` and `cc0a`.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:40F2-4128`: the main loop, checked call by call.
  - `01:4231-423B`: the evolution gate and `7193`.
  - `00:359D`: clears `+E7`.
  - `00:35B1`: clears the trainer lock and `+E8`.
  - `01:7157`: `cc06 = 0` at duel start.
- **Test.** `turn_sequence_resets_per_turn_flags`, `turn_counter_increments_after_between_turns`

### R-TURN-04: Main phase actions in any order
- **Rule.** The main phase is a loop. The turn player may repeat any of these, in any order and as
  often as allowed:
  - Play a card from hand. A Basic goes to the Bench if fewer than 6 creatures are in play. An
    evolution card evolves a creature. Energy is attached, once per turn. A Trainer is played.
  - Use a Power.
  - Retreat.
  - Look at cards.

  Each of these returns to the loop. Only an attack that resolves, or *Done*, ends the turn. Using a
  Power does not end the turn.
- **Parameters.** Jump table `01:42F5`: Hand `4429`, Attack `4700`, Check `4589`, Power `4392`,
  Retreat `43AF`, Done `439E`. An empty hand returns to the menu.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:445F-4478`: dispatch by card type. Every branch returns to `4271`.
  - `01:44DF-4527`: Basic → Bench, otherwise evolve.
  - `01:4392-439B`: Power → menu.
- **Test.** `main_phase_multiple_actions_same_turn`

### R-TURN-05: Attacking ends the turn; no first-turn restriction
- **Rule.** Once an attack is committed, the turn ends, even if the attack then fails a coin (a
  confusion self-hit, or a miss caused by an accuracy effect). Only cancelling, or the attack's own
  precondition failing, returns to the menu. The first player **may** attack on turn 0.

  Attacking is blocked when any of these is true (checked in this order):
  - the Active has an attack-lock substatus;
  - the Active is Asleep or Paralyzed;
  - the Active has no attacks;
  - the attached energy is not enough;
  - the chosen attack was disabled by the opponent's last attack. The other attacks stay usable.
- **Parameters.** Blocks:
  - `00:3378`: attacker `+E8 ∈ {5, 6, $0B}`.
  - `01:491E`: condition nibble ∈ {2, 3}.
  - `01:4829`: 0 attacks.
  - `01:4895`: not enough energy.
  - `00:3398`: `+E8 == 4` and the chosen index equals the stored disabled index `+F2`.

  Failures that still end the turn: `00:1783` (accuracy coin) and `00:1788` (confusion).
- **Status.** CORRECTED. The original claim missed the disabled-attack check and the failures that
  still end the turn.
- **Evidence.**
  - `01:4700-470D`: the pre-checks, attack lock (`01:4700 → 00:3378`) before Asleep/Paralyzed
    (`01:4705 → 01:491E`). The turn counter is never read on this path.
  - `01:4772-4780`: `call 1695; jp c,4271; ret`, so no carry means the turn ends.
  - `00:1695`: carry returns to the menu only at `177E/1781`.
- **Test.** `first_player_may_attack_turn_zero`, `attack_ends_turn`, `disabled_attack_only_blocks_that_attack`,
  `attack_blocked_without_energy_or_attacks`

### R-TURN-06: Done ends the turn without confirmation
- **Rule.** Choosing Done ends the turn immediately. There is no confirmation prompt; the only
  "decline" path belongs to practice-duel scripting. Done clears the non-turn player's last-attack
  bookkeeping (`+F2..F9`), which includes the disabled-attack memory.
- **Parameters.** `01:51ED` is a practice-only hook (`cc13 ≠ 0`). `01:7227` zeroes the non-turn
  player's `+F2..F9`.
- **Status.** CORRECTED. The original claim said Done asks for confirmation.
- **Evidence.** `01:439E-43AE`. `01:51ED` returns immediately when `cc13 == 0`. `cc13` is set only at
  `00:2AFB`.
- **Test.** `end_turn_passes_turn_without_prompt`

### R-TURN-07: Result codes and when a duel ends
- **Rule.** The duel result is stored relative to the turn player: 1 means T wins, 2 means T loses,
  3 means a draw, and a draw always leads to sudden death. The result is checked after each menu
  action (the turn ends early), after the turn, and after between-turns processing. A result set
  during between-turns ends the duel before the turn counter goes up. The AI's turn loop stops the
  same way.
- **Parameters.** `cc07`. Checks at `01:42A5`, `01:4102` and `01:4114`. The AI loop checks at
  `01:68A0` and `01:6A17`.
- **Status.** CONFIRMED
- **Evidence.** `01:4161-41B9` (the result screens), `01:42A2-42A9`, `01:40FC-4118`.
- **Test.** `ko_mid_turn_ends_duel_immediately`, `poison_ko_between_turns_ends_before_next_draw`

### R-TURN-08: A draw becomes sudden death
- **Rule.** A draw (`cc07 = 3`) always restarts the duel as sudden death:
  - 1 prize each;
  - turn counter back to 0, so the first-turn restrictions apply again;
  - full setup again (reset, shuffle, draw 7, mulligans, placement, prizes);
  - a **new** coin flip for the first player.

  Sudden death can repeat. In a linked duel, link data only decides which side runs the setup and
  flips the coin.
- **Parameters.** `cc08 = 1`. `7157` re-initialises. `ff98 = $C2`, then `4B5F`.
- **Status.** CORRECTED. The coin flip still decides the first player in linked duels too.
- **Evidence.** `01:41B4-41F4`. `01:7157`. `4B5F` includes the coin at `4C20/4C51`.
- **Test.** `draw_result_starts_sudden_death_one_prize`

---

## 3. Energy (R-ENERGY)

### R-ENERGY-01: One energy attachment per turn
- **Rule.** A player may attach at most one energy card from hand per turn, to the Active or to any
  Benched creature. The limit is checked before the target is chosen. The flag is set only when the
  attachment is committed, so cancelling the target choice keeps the attachment available. This
  applies on any turn, including both players' first turns. The flag resets every turn.
- **Parameters.** Flag `cc0b`: 0 means an attachment is still allowed, 1 means it has been used. The
  attach primitive `00:1437` sets the card's location to `$10|slot`.
- **Status.** CONFIRMED
- **Evidence.**
  - `01:4485`: flag check (text `$3E`).
  - `01:448B-4496`: picker, cancel → menu, otherwise set the flag.
  - `01:7193`: per-turn reset.
  - `01:6A65`: the AI/link executor sets the same flag.
- **Test.** `second_energy_attach_same_turn_rejected`, `cancel_attach_keeps_allowance`, `energy_attach_allowed_turn_zero`

### R-ENERGY-02: Free-attach Power exception
- **Rule.** A generic Power can lift the one-per-turn limit for one energy colour going onto creatures
  of that same colour. While a holder of this Power is in play **on the attaching side**, a matching
  energy attached to a creature whose *effective* colour matches does not use up the attachment.
  The exception does not apply while a power blocker is in play.

  Counting rule (R-POWER-03): a Benched holder always counts. An Active holder counts only if it is
  not Confused, Asleep or Paralyzed (Poison does not matter). The blocker is counted the same way:
  a Confused, Asleep or Paralyzed Active blocker does not block. If the exception does not apply and
  the normal attachment is already used, the play is refused. The AI/link executor always sets the
  flag.
- **Parameters.** Energy type `$0B`. Holder ID `$43` (own side only). Blocker ID `$27` (both sides).
  Target colour from `00:36AE` must equal 3. The free branch goes to `01:4499` and skips the flag write.
- **Status.** CORRECTED. The verifier refined the status test, the blocker counting, the effective
  colour and the AI case.
- **Evidence.** `01:447B-4483`, `00:35CC-35D8`, `00:34DC-34FF`, `00:35D9-35EE`, `01:44B3-44C7`, `01:6A65`.
- **Test.** `free_attach_power_allows_extra_matching_energy`, `free_attach_power_off_when_holder_active_asleep`

### R-ENERGY-03: Energy counting
- **Rule.** Attached energy is counted per colour. A basic energy gives 1 unit of its colour. The
  special 2-unit colorless energy gives 2 units, both in the colorless bucket. The total over all
  buckets is used to pay retreat costs and the colorless part of attack costs.
- **Parameters.** Buckets `cc1b..cc22` are indexed by `type & 7`. Index 6 (special) is incremented
  twice. The total is stored in `cc23`.
- **Status.** CONFIRMED
- **Evidence.** `00:1504-1553`.
- **Test.** `special_colorless_energy_counts_two_units`

### R-ENERGY-04: Paying an attack cost
- **Rule.** An attack is usable when two checks pass:
  1. For each of the 6 typed colours, the attached units of that colour are at least the
     requirement for that colour.
  2. The total units minus the sum of the typed requirements are at least the colorless requirement.

  So surplus typed energy pays colorless. The 2-unit special energy pays only colorless. The check
  only tests sufficiency and does not assign cards to costs. An empty attack slot, or a Power entry
  (category 4), can never be used as an attack.
- **Parameters.** The cost is 4 bytes of nibbles: 6 typed requirements, then the colorless
  requirement in the high nibble of the 4th byte. The typed sum is kept in `cbcd`.
- **Status.** CONFIRMED
- **Evidence.** `01:48B2-491D`: per-colour compare at `4906`, colorless check at `48EF-4902`,
  empty/Power rejection at `48C3-48D2`.
- **Test.** `attack_cost_colorless_paid_by_surplus`, `special_energy_cannot_pay_typed_cost`

---

## 4. Retreat (R-RETREAT)

### R-RETREAT-01: Retreat preconditions
- **Rule.** A retreat is refused, and nothing is discarded, when any of these holds (checked in
  this order):
  1. The opponent's effect has set "can't retreat" on the Active.
  2. The Active is Asleep or Paralyzed.
  3. There is no Benched creature with HP above 0.
  4. The Active is a Trainer-as-Basic card.
  5. The attached units are fewer than the retreat cost.

  Poison and Confusion do not block the attempt.
- **Parameters.** (1) `+E8 == $09`. (2) Condition nibble is 2 or 3. (3) `01:5FF6` counts Bench slots
  with nonzero HP. (4) Card type `$10`. (5) Cost `> cc23`. On the confused path the
  retreat-failed flag is checked first (R-RETREAT-03).
- **Status.** CONFIRMED
- **Evidence.** `01:45BF-45F7` (the chain), `00:354E-355F`, `01:491E-4938`, `01:5FF6`, `01:45F8-4614`.
- **Test.** `retreat_rejected_when_asleep_or_paralyzed`, `retreat_rejected_without_live_bench`, `poisoned_active_can_retreat`

### R-RETREAT-02: Retreat cost and payment
- **Rule.** The retreat cost is the printed cost, minus 1 for each copy of a generic retreat-reducer
  Power on the **retreating player's own Bench**. The reduction is ignored while a power blocker is in
  play, and the cost cannot go below 0. The player chooses which attached energies to discard.
  Choosing stops as soon as the paid units reach or exceed the cost. The 2-unit energy counts as 2
  and is discarded whole, with no refund. A cost of 0 discards nothing.
- **Parameters.** Base cost at card `+$32`. Reducer ID `$B6`, counted on the own Bench only (an
  Active copy or an opposing copy has no effect). The payment loop adds 1, or 2 for the special
  energy.
- **Status.** CONFIRMED
- **Evidence.** `00:1B65 → 00:3521-354D`, `01:4615-461D`, `01:4646-466C`.
- **Test.** `retreat_cost_two_paid_by_single_special_energy`, `retreat_reducer_counts_own_bench_only`, `retreat_cost_zero_discards_nothing`

### R-RETREAT-03: Confused retreat flip
- **Rule.** A Confused Active may try to retreat. The chosen energies are discarded first, then a
  coin is flipped, even when the cost is 0.
  - Heads: the switch happens.
  - Tails: the energy stays discarded, the creature stays Active, and a *retreat failed* flag is set.
    While that flag is set and the Active is Confused, further retreat attempts that turn are refused.

  The flag is cleared on a successful retreat and at the start of every turn. The flag is read only
  on the confused path, so if confusion is cured later that turn, a normal retreat is allowed again.
- **Parameters.** Flag `cc0c`. Carry means heads, which means success.
- **Status.** CONFIRMED
- **Evidence.** `01:43AF-43C0`, `01:65D0-65F4` (discard at `65AE` before the coin at `3034`). `cc0c`
  is written only at `65E5/65F1/7197`.
- **Test.** `confused_retreat_tails_discards_energy_and_stays`, `confused_retreat_failure_blocks_retry_same_turn`

### R-RETREAT-04: Switching clears the outgoing Active's conditions
- **Rule.** Any switch between Active and Bench (retreat or an effect) clears every condition and
  every attack-applied effect on the outgoing Active. That covers conditions, substatuses, changed
  weakness and resistance, the doubling flag, and last-attack data. Conditions are stored only for the
  Active, so a Benched creature never has one. These move with the creature: damage, attached cards,
  stage, evolve and Power flags, the per-slot colour override and the attached-trainer counters. The
  player-wide trainer lock is not cleared.
- **Parameters.** `00:13C6` zeroes `+F0`, `+E7`, `+E8`, `+E9`, `+EA`, `+EB` bit 0 and `+F2..F9`.
  `00:14A8` then swaps `+BB`, `+C8`, `+C2`, `+CE`, `+D4`, `+E0` and `+DA`, and rewrites the
  location bytes.
- **Status.** CONFIRMED
- **Evidence.** `00:14A8-14EC`, `00:13C6-13E9`.
- **Test.** `retreat_clears_status_keeps_damage`

### R-RETREAT-05: No per-turn retreat limit
- **Rule.** A creature that is not Confused may retreat any number of times per turn, paying each
  time. Retreating does not end the turn, and the new Active may attack. Retreat is available on every
  turn, including first turns.
- **Parameters.** The only per-turn retreat state is `cc0c` (R-RETREAT-03).
- **Status.** CONFIRMED
- **Evidence.** `01:43F5-4420` (no flag read, `jp 4271`). `05:4078-408E`: the AI retreats twice in
  one turn.
- **Test.** `retreat_twice_same_turn_then_attack`

### R-RETREAT-06: Cancelling a retreat costs nothing
- **Rule.** Cancelling during energy or Bench selection abandons the retreat at no cost. Energy that
  was discarded for display is re-attached.
- **Parameters.** `01:65BA` re-attaches the cards to slot 0. Cancel goes to `jp 4271`.
- **Status.** CONFIRMED
- **Evidence.** `01:43FF-4415`, `01:65BA-65CE`.
- **Test.** `cancel_retreat_keeps_energy`

---

## 5. Special conditions (R-STATUS)

### R-STATUS-01: Condition layout
- **Rule.** Each player has one condition byte, for the Active only. Confused, Asleep and Paralyzed
  exclude each other. Poison (single or double) is stored separately and can combine with any one
  of them.
- **Parameters.** `+F0` low nibble: 1 Confused, 2 Asleep, 3 Paralyzed. Bits 7-6: `$80` Poisoned,
  `$C0` Double-Poisoned.
- **Status.** CONFIRMED. The verifier swept all 47 access sites.
- **Evidence.** `01:491E`, `00:1837`, `01:6DEC`.
- **Test.** `asleep_and_poisoned_coexist`, `confused_replaces_asleep`

### R-STATUS-02: Applying a condition
- **Rule.** Applying Confused, Asleep or Paralyzed replaces whichever of those three is present and
  keeps Poison. Applying Poison or Double-Poison overwrites the poison level and keeps the other
  condition. So Poison on a Double-Poisoned creature makes it single-Poisoned. The coin variants flip
  first and apply only on heads.
- **Parameters.** `new = (old & mask) | value`:

  | Condition | mask | value |
  |---|---|---|
  | Poison | `$0F` | `$80` |
  | Double-Poison | `$0F` | `$C0` |
  | Paralyzed | `$F0` | `3` |
  | Confused | `$F0` | `1` |
  | Asleep | `$F0` | `2` |

- **Status.** CONFIRMED
- **Evidence.** `0B:4000-4033` (the constructors), `01:6EE5` (apply).
- **Test.** `poison_on_double_poisoned_becomes_single`, `paralysis_keeps_poison`

### R-STATUS-03: When conditions take effect, and immunity
- **Rule.** An attack's condition is decided, including its coin, before damage. It is queued and
  written only after damage and the after-damage effects. If the defender was protected from the
  attack's effects, conditions aimed at the defender are dropped, and conditions the attacker puts on
  itself still apply. A Trainer-as-Basic creature can never receive a condition. A generic
  "condition immunity" Power protects its holder while that Power is usable.
- **Parameters.** The queue is at `ccd3/ccd4` as entries (target, mask, value). It is applied at
  `01:6E9E`. Effects count as negated when `cccd ≠ 0`. Immune IDs are `$CB` and `$CC`, plus `$BE`
  gated by `34A7`.
- **Status.** CONFIRMED. The verifier raised it to high confidence.
- **Evidence.** `06:4765` (phase-3 entries), `00:1715`, `00:1770`, `0B:4035-4060`, `01:6EB4-6EDF`.
- **Test.** `protected_defender_drops_status_but_attacker_self_status_applies`, `trainer_as_basic_immune_to_status`

### R-STATUS-04: Asleep and Paralyzed block attacking and retreating
- **Rule.** An Asleep or Paralyzed Active cannot attack or retreat. A Confused Active may try both,
  with a coin. Poison blocks neither. When an attack lock or a can't-retreat effect also applies, that
  refusal is reported first (R-TURN-05, R-RETREAT-01).
- **Status.** CONFIRMED
- **Evidence.** `01:4700`, `01:45BF`, `05:4250` (the AI uses the same check).
- **Test.** `paralyzed_cannot_attack_or_retreat`, `poisoned_can_attack_and_retreat`

### R-STATUS-05: Conditions disable the Active's Powers
- **Rule.** While the Active is Confused, Asleep or Paralyzed, its activated Power cannot be used and
  its passive Power stops working. That includes no longer counting for "is this Power in play".
  Poison does not disable Powers. Benched creatures are never affected.
- **Parameters.** `00:34A7(slot)`. Only slot 0 tests the nibble. Slot ≠ 0 skips straight to the
  blocker check.
- **Status.** CONFIRMED
- **Evidence.** `00:34A6-34B5`, `0B:4D0C-4D1B`, `00:34DC-34FF` (`34F6`).
- **Test.** `confused_active_power_blocked`, `poisoned_active_power_usable`

### R-STATUS-06: Confusion coin when attacking
- **Rule.** When a Confused creature attacks, its energy-discard cost is paid first, after any
  accuracy coin from its own substatus. Then a coin is flipped.
  - Heads: the attack resolves normally.
  - Tails: the attack does nothing else. The attacker takes 20 self-damage, modified as in
    R-DMG-05. Knock-outs are processed. The attack counts as used, so the turn ends.
- **Parameters.** Order inside `00:1695`: phase 1 → accuracy → phase 2 → phase 6 (costs) →
  `00:1837` coin → phase 5 → phase 3 → damage → phase 4 → status queue. Tails runs `00:1788`. The
  turn ends because of the no-carry return to `01:477A`, not because of `7227`.
- **Status.** CORRECTED. `01:7227` only clears the opponent's last-attack variables.
- **Evidence.** `00:16E1-16E9`, `00:1848-1856`, `00:1788-17AA`. The AI path `01:6B4C` uses the same
  order.
- **Test.** `confused_tails_pays_cost_takes_20_and_ends_turn`

### R-STATUS-07: Between-turns order
- **Rule.** After the turn ends (with T = the player whose turn just ended), these steps run in order:
  1. T's Active: poison damage. If it survives, a sleep coin. Then Paralysis is removed.
  2. T's +10 trainers are discarded.
  3. N's Active: poison damage, then a sleep coin if it survives. No Paralysis removal.
  4. N's −20 trainers are discarded.
  5. Knock-outs are handled for both sides together.

  Confusion is never processed here. If nothing needs processing, the condition steps are skipped
  silently and the knock-out check does not run.
- **Parameters.** `01:6C5C`. The skip test is `6D15`, plus nibble 3 for T only.
- **Status.** CONFIRMED
- **Evidence.** `01:410B`, `01:6C9D-6CF0`, `01:6D15`.
- **Test.** `paralysis_inflicted_lasts_through_owner_next_turn`, `between_turns_poison_before_sleep`

### R-STATUS-08: Poison damage
- **Rule.** A Poisoned Active takes 10 damage at every between-turns step (after either player's
  turn), or 20 if Double-Poisoned. This ignores weakness, resistance, trainers and every other
  modifier. Poison never wears off on its own. A knock-out from poison gives prizes as usual.
- **Parameters.** `$0A` or `$14`, subtracted raw through `00:19F6`.
- **Status.** CONFIRMED
- **Evidence.** `01:6DEC-6E2F`, `00:19F6`.
- **Test.** `poison_ignores_minus_20_trainer`, `double_poison_deals_20`

### R-STATUS-09: Sleep coin
- **Rule.** At every between-turns step, each Asleep Active flips a coin, after its poison damage.
  Heads wakes it (Poison stays). Tails keeps it Asleep. A creature knocked out by poison gets no coin.
- **Status.** CONFIRMED
- **Evidence.** `01:6DA7-6DD7`, called from `6CA8` and `6CE4`.
- **Test.** `sleep_flip_heads_wakes_up`, `sleep_flip_tails_stays_asleep`

### R-STATUS-10: How long Paralysis lasts
- **Rule.** Paralysis is removed without a coin at the between-turns step that ends its **owner's**
  turn. So it lasts through exactly one of the owner's turns. Poison stays.
- **Status.** CONFIRMED
- **Evidence.** `01:6CAB-6CC4`, in T's section only.
- **Test.** `paralysis_cured_after_owner_turn`

### R-STATUS-11: Confusion persists
- **Rule.** Confusion never wears off by itself. It ends only when the creature leaves the Active
  spot, evolves, is knocked out, or is cured by an effect.
- **Status.** CONFIRMED
- **Evidence.** `01:6C5C-6D2A` never tests nibble 1.
- **Test.** `confusion_persists_across_turns`

### R-STATUS-12: Cures
- **Rule.** All conditions and attack effects on the Active are cleared when:
  - it is switched out;
  - it evolves (Active only);
  - a creature is put into an empty Active spot, including replacement after a knock-out;
  - a card effect removes or replaces it.

  Generic cure effects also exist: a trainer that requires the Active to have a condition and then
  clears it, and a once-per-turn Power that clears both Actives.
- **Parameters.** `00:13C6`, called from `00:14A8`, `00:134A`, `00:142E`, `01:6FD7`, `0B:4462`,
  `0B:703A` and `0B:77F9`. Trainer cure at `0B:74C5/74D6`. Power cure at `0B:4E5C-4EAA`.
- **Status.** CONFIRMED
- **Evidence.** As listed in Parameters.
- **Test.** `evolving_active_clears_all_conditions`, `cure_trainer_requires_condition`

### R-STATUS-13: Record of the last inflicted conditions *(verifier-found)*
- **Rule.** The engine records which conditions the most recent attack inflicted. A copy-effect can
  re-apply those recorded conditions to the other side through the normal helpers.
- **Parameters.** `+F5` is cleared on both sides at each queue apply and mirrors the mask and value.
  The reader is at `0B:69F4-6A23`.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `01:6E9E`, `01:6EED`, `0B:6A8F`.
- **Test.** `last_inflicted_status_recorded`

### R-STATUS-14: Conditions applied directly and self-confusion *(verifier-found)*
- **Rule.** Some effects write a condition directly, without the queue. Those still check immunity,
  but negation does not stop them. An attack may confuse both the defender and the attacker. The
  self copy skips the immunity checks and survives negation.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `0B:7216-7251`, `0B:4793-479C`, `0B:5300-531F` (paralysis plus an immediate apply).
- **Test.** `self_confusion_survives_negation`

---

## 6. Damage (R-DMG)

### R-DMG-01: Order of the damage calculation for a direct attack
- **Rule.** An attack's direct damage to the defending Active is computed in this fixed order:
  1. Base damage.
  2. The attacker's own doubling effect.
  3. Weakness: ×2.
  4. Resistance: −30.
  5. +10 for each +10 trainer on the attacker.
  6. −20 for each −20 trainer on the defender.
  7. The defender's prevention and reduction (R-DMG-09).
  8. Clamp to 0.

  The value is clamped **only** at the end. The pipeline is wrapped by a prevention check before the
  calculation (R-DMG-12) and a coin hook after it (R-DMG-13).
- **Parameters.** ×2 is one shift. −30 is `$FFE2`. The trainer counts are multiplied by 10 and by 20.
  The clamp tests the 16-bit sign bit. Example: base 10, resisted, with +10 attached, gives
  10 → −20 → −10 → 0.
- **Status.** CONFIRMED
- **Evidence.** `00:18F4-196D` (the only caller is `00:171A`), `00:19C9-19F5`.
- **Test.** `damage_order_weakness_then_resistance_then_trainers`, `resisted_10_with_plus10_is_zero`

### R-DMG-02: Zero base damage skips all modifiers
- **Rule.** An attack with 0 base damage stays at 0. The same happens if the attacker step leaves it
  at 0.
- **Status.** CONFIRMED. Edge case: the weakness/resistance-ignore flag path skips the second zero
  check. This does not occur with real data.
- **Evidence.** `00:18F8-1902`, `00:191B-191D`.
- **Test.** `zero_base_damage_ignores_weakness_and_plus10`

### R-DMG-03: Where the type for weakness and resistance comes from
- **Rule.** Weakness and resistance compare the attacker's current colour with the defender's
  weakness and resistance sets.
  - The defender's changed weakness or resistance is used when it is nonzero, with no gate.
    Otherwise the printed value is used.
  - The attacker's changed colour (from a Power) is used only while that Power is working. If the
    attacking Active has a condition, or a power blocker is in play, its printed colour is used.
  - A Trainer-as-Basic creature counts as colour index 6.

  Weakness ×2 is applied before resistance −30.
- **Parameters.** Type mask table `80 40 20 10 08 04 02 01`. Changed colour is `+D4` bit 7, gated by
  `34A7`. `+E9` and `+EA` override the card's weakness and resistance.
- **Status.** CORRECTED. The original claim missed the gate on the changed colour.
- **Evidence.** `00:36AE-36DF`, `00:36E7-3713`, `00:197A`.
- **Test.** `changed_weakness_overrides_printed`, `weak_and_resistant_same_type_x2_then_minus30`

### R-DMG-04: Weakness and resistance apply only to direct damage on the Active
- **Rule.** Effect damage (to the Bench, or to either Active through an effect) never uses weakness
  or resistance.
  - Effect damage to an **Active** adds the attacker's +10 trainers, even when it comes from a Power.
  - Effect damage to **any** slot subtracts the −20 trainers on the damaged creature, except Power
    damage.
  - Effect damage to the Active also goes through the defender's reductions. It is cancelled
    entirely while the attack-prevention flag is set.
  - After the clamp, a damaged Benched creature's own reduction Power still applies (prevent at ≥30,
    or halve), unless the damage came from a Power or a power blocker is in play.
- **Parameters.** `00:1A53`. The target slot goes to `ccf1`. `ccb7 == 4` means Power damage. The
  clamp is at `1AB8`. The post hook `32AE` runs for every slot.
- **Status.** CORRECTED. The original claim missed the +10 on Power damage and the post-clamp Bench
  Power.
- **Evidence.** `00:1A53-1AEC`, `0B:4117-412D` (the Bench helper).
- **Test.** `bench_effect_damage_ignores_weakness`, `bench_minus20_trainer_reduces_effect_damage`

### R-DMG-05: Self-damage uses the attacker's own modifiers
- **Rule.** Damage a creature deals to itself (the confusion hit, or recoil) uses its **own**
  weakness and resistance against its **own** colour, then its own +10 and −20 trainers, then the
  clamp. Its substatus modifiers are skipped. The prevention flag is saved and forced to 0 during
  the self-hit, so self-damage cannot be prevented.
- **Parameters.** Confusion hit base 20. `18BC → 1982`, with no turn swap.
- **Status.** CONFIRMED
- **Evidence.** `00:1788-179D`, `00:18BC-18F0`, `00:1982-19C8`. `18B5` has 18 recoil callers.
- **Test.** `confusion_self_hit_weak_to_own_type_takes_40`, `confusion_self_hit_resists_own_type_takes_0`

### R-DMG-06: HP storage and the knock-out test
- **Rule.** Each creature in play stores its remaining HP as raw points (one byte). There are no
  damage counters. Damage is subtracted, and HP is set to 0 instead of going negative. A creature is
  knocked out **exactly** when its HP is 0.
- **Parameters.** `+C8+slot`. `00:19F6` sets carry when the creature is still alive.
- **Status.** CONFIRMED
- **Evidence.** `00:19F6-1A0B`. `01:7074`, `01:70A4`, `01:6FB5` and `01:6FD0` all test only the HP byte.
- **Test.** `damage_equal_to_hp_knocks_out`, `overkill_clamps_hp_to_zero`

### R-DMG-07: Halving rounds down to 10
- **Rule.** A halving effect rounds down to a multiple of 10: 30 → 10, 50 → 20, 60 → 30. Every built-in
  constant is a multiple of 10.
- **Parameters.** `e>>1`, then −5 if the result is odd. This is only correct up to 255, which covers
  all values that occur.
- **Status.** CONFIRMED
- **Evidence.** `00:3281-328E`, `00:32A0`.
- **Test.** `halve_damage_rounds_down_to_ten`

### R-DMG-08: The attacker's doubling comes before weakness
- **Rule.** The attacker's "next attack does double damage" state doubles the damage before
  weakness. With weakness the damage is ×4, then −30. The state has two parts. The attack sets a
  pending mark on its own Active. At the start of the owner's next turn the mark becomes "this
  turn's attacks are doubled", which then applies to every attack of that turn, including another
  use of the doubling attack itself (which sets the mark again). At the end of a turn the doubling
  is dropped unless the mark was set again that turn. Both parts are Active effects, so they are lost
  if the Active leaves (switch, evolve) before they are used.
- **Parameters.** The pending mark is `+E7 = $19`. The doubling is `+EB` bit 0, read inside `00:31D4`.
  `00:35A6-35AE` (start of turn) sets `+EB` bit 0 when `+E7 == $19`. `00:35C6-35C9` (end of turn)
  keeps bit 0 only while `+E7 == $19`.
- **Status.** CORRECTED. The original claim did not say how the state carries over, so a doubling
  attack used on the doubled turn was not doubled.
- **Evidence.** `00:31D4-31F9`, `00:359D-35B0`, `00:35B1-35CB`.
- **Test.** `double_damage_then_weakness_then_resistance`, `double_next_attack_applies_on_next_own_turn`,
  `double_next_attack_reused_on_doubled_turn`

### R-DMG-09: The defender's prevention and reduction
- **Rule.** After weakness, resistance and both trainers, these steps run in order:
  1. If the attack-prevention flag is set, damage becomes 0.
  2. Otherwise the defender's single "until next turn" effect applies: full prevention, −10, −20,
     "prevent if below 40", or halve. These are one byte, so only one can be active.
  3. Only if the defender has none of those effects, its own reduction Power applies: prevent if
     damage ≥ 30, or halve. That Power is gated by the conditions and the blocker check, and does not
     apply to Power damage.
  4. Finally, a −20 or −10 that the defender placed on the attacker.
  5. Clamp to 0.
- **Parameters.** Defender `+E7`: `$0F/$10/$11/$17` → 0, `$1E` → −10, `$13` → −20, `$0E` → 0 if
  < 40, `$15` → halve. Attacker `+E8`: `$03` → −20, `$07/$12` → −10.
- **Status.** CORRECTED. The steps exclude each other, and the reduction Power applies only when no
  `+E7` effect matched.
- **Evidence.** `00:3220-32AD`, `00:31FB-321F`, `00:324E`.
- **Test.** `defender_minus20_applies_after_weakness`

### R-DMG-10: Ignoring weakness and resistance
- **Rule.** An attack effect can mark its damage as unaffected by weakness and resistance. That
  damage still gets the doubling, both trainers, the defender's effects and the clamp.
- **Parameters.** Bit 15 of the damage word `ccbf/ccc0`.
- **Status.** CONFIRMED
- **Evidence.** `00:1909-1916`.
- **Test.** `ignore_wr_flag_skips_weakness_keeps_plus10`

### R-DMG-11: Flags for the damage message
- **Rule.** The engine records whether weakness and/or resistance was applied, for the UI.
- **Parameters.** `ccc7` bit 1 = weakness, bit 2 = resistance.
- **Status.** CONFIRMED
- **Evidence.** `00:193A/1951`, `06:51DD-51F2`.
- **Test.** `attack_result_reports_weakness_and_resistance_flags`

### R-DMG-12: Whole-attack prevention before the calculation *(verifier-found)*
- **Rule.** Before the before-damage phase, the defender's protection is checked. It is skipped for
  Powers and for attacks that do no direct damage. If the defender has a "prevent all effects of
  attacks" substatus, or a qualifying passive Power, the prevention flag is set. While the flag is
  set, direct damage is 0, effect damage to the defending Active is cancelled, conditions aimed at the
  defender are dropped, and the post coin hook is skipped.
- **Parameters.** `00:33E9`: defender `+E7 ∈ {$0D,$14,$0C}` gives `cccd` 3/2/1. The Power hook
  `3418` gives 5.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `00:170C`, `00:33E9`, `00:1A5C-1A63`.
- **Test.** `prevention_substatus_negates_damage_and_effects`

### R-DMG-13: Coin prevention after the calculation *(verifier-found)*
- **Rule.** After the damage is computed, a defender with a generic "flip: heads prevents the damage"
  Power flips a coin, if its Power is usable and the damage is not from a Power. Heads makes the
  damage 0 and sets the prevention flag.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `00:17FD`, `00:3441-3460`.
- **Test.** `coin_prevention_power_heads_zeroes_damage`

### R-DMG-14: Retaliation hook *(verifier-found)*
- **Rule.** When a creature with a generic retaliation Power takes nonzero damage that is not from a
  Power (and no blocker is in play), the other side's Active takes a flat 10 with no modifiers. A
  similar hook after phase 4 damages the attacker by 10 when a defender with a usable passive was
  damaged.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `00:32CE-3309`, `00:3632`.
- **Test.** `retaliation_power_deals_flat_10`

---

## 7. Evolution (R-EVO)

### R-EVO-01: Evolution follows the "evolves from" name
- **Rule.** A creature card with stage ≠ 0 played from hand evolves an in-play creature whose
  **name** matches the card's "evolves from" name. There is no stage arithmetic. A stage-0 card always
  goes to the Bench.
- **Parameters.** Hand card `+0A/+0B` is compared with the in-play card's name `+03/+04`.
- **Status.** CONFIRMED
- **Evidence.** `01:44DF`, `00:135C-138F`.
- **Test.** `evolve_requires_matching_evolves_from_name`

### R-EVO-02: No evolution on either player's first turn
- **Rule.** The settled flag is set at turn start only when the turn counter is ≥ 2, so neither
  player can evolve on their own first turn.
- **Status.** CONFIRMED
- **Evidence.** `01:4231` (`cp 2`), `01:71A3`, `01:7157/411A`.
- **Test.** `no_evolution_on_turns_zero_and_one`

### R-EVO-03: No evolution on the turn a creature came into play
- **Rule.** A creature placed in play has its flags cleared. It can evolve only after the start of its
  owner's next turn. The two errors differ: "no matching creature" versus "matching creature, but not
  this turn". The flag moves with the creature when it switches slots.
- **Status.** CONFIRMED
- **Evidence.** `00:140C`, `00:137B`, `01:4540-4552`, `00:14A8`.
- **Test.** `cannot_evolve_creature_benched_this_turn`, `no_evolvable_vs_not_this_turn_errors_differ`

### R-EVO-04: Only one evolution per creature per turn
- **Rule.** Evolving clears the slot's flags, so the same creature cannot evolve again that turn.
  Going from Basic to Stage 2 normally takes two turns.
- **Status.** CONFIRMED
- **Evidence.** `00:133C`.
- **Test.** `cannot_double_evolve_same_turn`

### R-EVO-05: No limit on evolutions per turn
- **Rule.** Different eligible creatures may each evolve once in the same turn. There is no counter
  across the turn.
- **Status.** CONFIRMED
- **Evidence.** `01:7193`, `01:4528-4583`, `00:1307-1359`.
- **Test.** `multiple_creatures_evolve_same_turn`

### R-EVO-06: Damage carries over
- **Rule.** New HP = current HP + (new max − old max), so the damage taken stays the same.
- **Parameters.** 8-bit arithmetic with no clamp.
- **Status.** CONFIRMED
- **Evidence.** `00:132C-133B`.
- **Test.** `evolve_keeps_damage`

### R-EVO-07: Attached cards and the evolution stack stay
- **Rule.** Attached energy and trainers stay with the creature. The earlier stage cards stay
  underneath in the same slot and go to the discard pile with it. Evolution does not reset the
  attached-trainer counters.
- **Status.** CONFIRMED
- **Evidence.** `00:1311-1327`, `00:1437`. The Bench path zeroes `E0/DA` at `00:1418`, and evolve
  does not.
- **Test.** `evolve_keeps_attached_energy`

### R-EVO-08: Evolving clears the slot's flags, and clears the Active's conditions
- **Rule.** For any slot, evolving clears the slot flags and the changed colour, and writes the new
  stage. For the Active only, all conditions and effects are cleared as well (R-STATUS-12). The
  player-wide trainer lock is not cleared.
- **Status.** CONFIRMED
- **Evidence.** `00:1342`, `00:13C6`, `00:134D`.
- **Test.** `evolve_active_clears_conditions`, `evolve_bench_keeps_slot_state`

### R-EVO-09: Evolution-lock Power
- **Rule.** While a creature with a generic evolution-lock Power is in play on either side, nobody
  can evolve, whether from hand or through the stage-skip trainer. The holder counts as in play when
  it is on the Bench, or when it is the Active without Confused, Asleep or Paralyzed. The lock is
  lifted while a power blocker is in play, using the same counting rule. The "nothing to evolve" and
  "not this turn" errors take priority over the lock.
- **Parameters.** `00:356E` is blocked when `count($8D) > 0` and `count($27) == 0`.
- **Status.** CORRECTED. The verifier refined how the blocker is counted.
- **Evidence.** `00:356E`, `00:34C0`, `00:34DC`, `01:455A`, `0B:76B8`.
- **Test.** `evolution_lock_power_blocks_evolution`, `evolution_lock_inactive_when_holder_asleep`

### R-EVO-10: Stage-skip trainer (optional effect)
- **Rule.** A trainer effect may put a Stage 2 from hand directly onto a settled Basic. The Basic's
  name must match the evolves-from name of the Stage 1 that the Stage 2 evolves from. The usual
  eligibility rules and the evolution lock still apply. The normal evolve body runs, and the stage is
  then recorded as 3.
- **Status.** CONFIRMED
- **Evidence.** `0B:76B3-7716`, `00:1390`, `00:2E84`.
- **Test.** `stage_skip_trainer_requires_settled_basic`

### R-EVO-11: Evolving refreshes a once-per-turn Power *(verifier-found)*
- **Rule.** Evolving zeroes the Power-used bit, so the newly evolved creature may use its own
  once-per-turn Power in the same turn.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `00:133C` (C2 = 0), `0B:4D0D`, `0B:4D69`.
- **Test.** `evolved_creature_may_use_power_same_turn`

### R-EVO-12: Devolve effect *(verifier-found)*
- **Rule.** A devolve effect sets HP to (the lower stage's max − damage), clamped at 0, restores the
  lower stage, clears the Active's conditions, and clears the slot flags. A devolved creature cannot
  evolve again that turn.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `0B:4440-4475`.
- **Test.** `devolve_keeps_damage_and_clears_flags`

### R-EVO-13: The evolve target picker *(verifier-found)*
- **Rule.** Choosing an invalid evolve target sends the player back to the picker. A creature
  placed from hand starts at stage 0.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `01:4572`, `01:44F7`.
- **Test.** `evolve_invalid_target_rejected`

---

## 8. Knock-outs, prizes and winning (R-KO)

### R-KO-01: One prize per knocked-out creature
- **Rule.** For each of the opponent's in-play creatures at 0 HP (Active or Bench), the player takes
  one prize. A knocked-out Trainer-as-Basic gives no prize. A player cannot take more prizes than
  remain. Prizes go to the hand one at a time. There are no multi-prize creatures.
- **Parameters.** The count goes to `ccce` (`01:7074`). The picking loop stops at count 0 or at
  `+EC == 0`.
- **Status.** CONFIRMED
- **Evidence.** `01:7074-70A3`, `01:5807-583C`, `02:4AAC-4B5A`.
- **Test.** `double_ko_gives_two_prizes`, `prizes_capped_at_remaining`, `trainer_as_basic_ko_gives_no_prize`

### R-KO-02: Choosing prizes
- **Rule.** A human chooses **which** remaining face-down prize slots to take. The AI picks at random
  among the remaining ones.
- **Status.** CONFIRMED
- **Evidence.** `02:4AAC` (cursor over the set bits), `05:41E5-421D`.
- **Test.** `take_prize_by_slot_must_be_remaining`

### R-KO-03: When knock-outs are checked
- **Rule.** The knock-out handler runs once, after the whole attack has resolved: damage, the
  after-damage effects (recoil, Bench damage), and queued conditions. So knock-outs on both sides,
  including the attacker knocking itself out, are handled together. It also runs after the confusion
  self-hit, after effects that deal damage outside an attack, and once at the end of between-turns
  processing.
- **Status.** CONFIRMED
- **Evidence.** `00:1764-177D`, `00:17A3`, `01:6CED`, `0B:58FF/6789/7CCC`.
- **Test.** `recoil_ko_and_defender_ko_resolved_together`

### R-KO-04: Order inside the knock-out handler
- **Rule.** With T as the turn player:
  1. Counter-KO (R-KO-09).
  2. Clear effects tied to a creature that was knocked out (R-KO-10).
  3. T takes prizes for N's knocked-out creatures.
  4. T may win early (R-KO-07).
  5. Otherwise N takes prizes for T's knocked-out creatures.
  6. T may lose early.
  7. N replaces its Active if it was knocked out, then T does.
  8. If any win bit is set, the result is looked up in the table.

  **In every case**, including the early exits, all 0-HP creatures with their attached cards go to the
  discard pile (T's first, then N's), and both play areas are compacted.
- **Parameters.** The discard path `6F5F-6F70` runs on every path.
- **Status.** CORRECTED. Discarding always happens. It is not an alternative to deciding the result.
- **Evidence.** `01:6EF6-6F7D`, `01:6FB5`, `00:1442-145C`, `00:1483`.
- **Test.** `ko_turn_player_takes_prizes_first_opponent_replaces_first`

### R-KO-05: Replacing a knocked-out Active
- **Rule.** A player whose Active is at 0 HP must immediately choose a new Active from Benched
  creatures with HP above 0. The choice cannot be cancelled. If none exists, that player has no
  creatures (a win condition for the opponent).
- **Status.** CONFIRMED. The AI's own filter was not traced, so the engine must enforce the rule.
- **Evidence.** `01:6FD0-7034`, `01:5FF6-6024`, `01:60BD-60C7`.
- **Test.** `ko_active_must_be_replaced_by_live_bench`, `no_live_bench_means_no_creatures`

### R-KO-06: Result table
- **Rule.** Each player can meet two conditions in a knock-out step:
  - (a) they took their last prize;
  - (b) the opponent's Active is knocked out and the opponent has no live Bench.

  Result = sign((T's conditions) − (N's conditions)). A positive value means T wins, a negative
  value means T loses, and equal nonzero counts mean a draw, which leads to sudden death.
- **Parameters.** `ccee = pT<<3 | pO<<2 | nO<<1 | nT`. Table `01:6F7F` =
  `00 02 01 03 02 02 03 02 01 03 01 01 03 02 01 03`.
- **Status.** CONFIRMED. All 16 entries were checked.
- **Evidence.** `01:6F58-6F7C`, `01:6FA3-6FB4`, `01:4161-4197`.
- **Test.** `last_prize_but_own_active_ko_no_bench_is_draw`, `both_take_last_prize_is_draw`

### R-KO-07: Early win and early loss
- **Rule.** If T takes its last prize, still has a creature with HP above 0, and N cannot also finish
  its prizes in this step, T wins at once. N's prize count is reduced silently, and no replacements
  happen. If only N finishes its prizes and N still has a live creature, T loses at once. The result
  always matches the table, so an engine may use the table path every time. The knock-out discard
  still happens.
- **Status.** CONFIRMED
- **Evidence.** `01:6F09-6F4A`, `01:720E-7226`, `01:70A4`.
- **Test.** `early_win_matches_table_result`

### R-KO-08: Knock-outs between turns
- **Rule.** A poison knock-out between turns uses the same handler. The player whose turn just ended
  counts as T: that player takes prizes first and replaces second.
- **Status.** CONFIRMED
- **Evidence.** `01:6C7D-6CF0`, `01:411A-4128`.
- **Test.** `between_turns_double_poison_ko_turn_ender_takes_prize_first`

### R-KO-09: Counter-KO effect
- **Rule.** If the defender was given a "counter-KO" effect during its own last turn, and the
  attacker's attack knocks it out while the attacker survives, the attacker is knocked out too. This
  is checked before prizes and is not checked between turns. The effect lasts through the opponent's
  turn, because `+E7` is cleared at the start of its owner's turn.
- **Parameters.** N's `+E7 == $16`.
- **Status.** CONFIRMED
- **Evidence.** `00:35F2-362E`, `00:359D`.
- **Test.** `counter_ko_effect_knocks_out_attacker`

### R-KO-10: Knock-out clears linked effects
- **Rule.** When one side's Active is knocked out, these values in the other side's `+E8` are
  cleared: 3, 5, 6, 7 and `$12` (effects that depended on it).
- **Status.** CONFIRMED
- **Evidence.** `01:6F8F-6FA2`, `00:357E-359C`.
- **Test.** `ko_clears_effects_placed_by_defender`

---

## 9. Deck and zones (R-DECK, R-ZONE)

### R-DECK-01: Exactly 60 cards
- **Rule.** A deck is legal only with exactly 60 cards.
- **Status.** CONFIRMED
- **Evidence.** `02:5545` (`cp $3C`), `00:0FD7-100D`, `01:71C5`.
- **Test.** `deck_must_have_exactly_sixty`

### R-DECK-02: At least one Basic
- **Rule.** A deck needs at least one Basic creature. Trainer-as-Basic cards do not count.
- **Status.** CONFIRMED
- **Evidence.** `02:5674`, `02:556F-557D`.
- **Test.** `deck_without_basic_rejected`

### R-DECK-03: At most 4 cards with the same name
- **Rule.** A deck may hold at most 4 cards that share a **name**. Different versions with the same
  name count together. Basic energy is exempt. The special 2-unit energy, trainers and creatures are
  limited.
- **Parameters.** Limit `cfd9 = 4`. Types `$08-$0D` are exempt.
- **Status.** CONFIRMED
- **Evidence.** `02:5D9F-5DDA`, `02:5D4D`.
- **Test.** `max_four_copies_by_name`, `basic_energy_unlimited`, `special_energy_limited_to_four`

### R-ZONE-01: Bench limit and placing a creature
- **Rule.** A player can have at most 6 creatures in play: 1 Active and 5 Benched. This is enforced
  for playing from hand, for setup placement and for effects. A newly placed creature has full HP, its
  flags cleared (so it cannot evolve this turn), and its stage recorded. If it goes to slot 0, its
  conditions are cleared.
- **Status.** CONFIRMED
- **Evidence.** `00:13EA` (`cp 6`), `01:44E5`, `01:4D68`, `0B:631C/6325/74B3`.
- **Test.** `bench_full_rejects_basic`

### R-ZONE-02: No hand-size limit
- **Rule.** There is no hand-size limit and no discard at the end of the turn.
- **Status.** CONFIRMED
- **Evidence.** `00:1088`. A full ROM sweep found the hand count compared to constants only in AI and
  effect-cost code.
- **Test.** `no_hand_limit_at_end_of_turn`

### R-ZONE-03: Shuffling
- **Rule.** A shuffle permutes only the cards still in the deck. Drawn and prize cards are untouched.
  The source uses a naive swap shuffle. The clean engine may use Fisher–Yates; the multiset must be
  preserved. Effects that search the deck shuffle afterwards.
- **Status.** CONFIRMED
- **Evidence.** `00:1021`, `00:11E4`, `00:0843`, `0B:40BD` (21 call sites).
- **Test.** `shuffle_preserves_deck_multiset`

### R-ZONE-04: Searching and returning to the deck
- **Rule.** Searching removes one specific card. The rest keep their order, and the card waits in
  limbo until an effect places it. A card can also be returned to the top of the deck.
- **Status.** CONFIRMED
- **Evidence.** `00:1061`, `00:104D`.
- **Test.** `search_deck_removes_exact_card`

### R-ZONE-05: Discard pile
- **Rule.** The discard pile is an ordered public list, and new cards go on top. Only cards in hand
  can be discarded from hand. A knocked-out creature goes there with every card in its slot (all
  stages, energy and trainers). In the source these are added in deck-index order. The play area is
  then compacted, starting from slot 0. Effects can take specific cards back from the discard pile.
- **Status.** CONFIRMED
- **Evidence.** `00:10D2`, `00:10C5`, `00:1442-145C`, `00:1490`, `00:10E7`.
- **Test.** `ko_stage1_with_two_energy_discards_four_cards`

---

## 10. Trainers and the effect framework (R-TRAINER, R-EFFECT)

### R-TRAINER-01: No limit on trainers
- **Rule.** Any number of trainers may be played in the main phase, on any turn including the first.
  The turn continues afterwards.
- **Status.** CONFIRMED
- **Evidence.** `01:4463-4478`, `01:7193`. `00:1859` never reads `cc06`.
- **Test.** `three_trainers_same_turn_on_turn_zero`

### R-TRAINER-02: How a trainer is played
- **Rule.** A trainer is played in this order:
  1. Lock check.
  2. Load the effect list.
  3. Phase 1 check. On failure an error is shown and the card stays in hand.
  4. Phase 2 selection. Cancelling returns to the menu, with the card still in hand.
  5. Phase 6 costs.
  6. Phase 5.
  7. Phase 3 main effect.
  8. Discard the card.
- **Status.** CONFIRMED
- **Evidence.** `00:1859-18A3`, `00:18A4`.
- **Test.** `trainer_failed_precondition_stays_in_hand`

### R-TRAINER-03: Discarded only if still in hand
- **Rule.** After resolving, a trainer goes to the discard pile only if it is still in the hand. A
  trainer that its own effect moved (for example a Trainer-as-Basic placed into play) stays where it
  is.
- **Status.** CONFIRMED
- **Evidence.** `00:10C5`.
- **Test.** `trainer_as_basic_stays_in_play_after_resolving`

### R-TRAINER-04: Trainer lock
- **Rule.** An attack effect can stop the defending **player** from playing trainers. It lasts through
  that player's next turn and is cleared at the end of it. The lock is set even when the defender's
  protection negated the attack. The lock belongs to the player, so switching does not remove it.
- **Parameters.** N's `+EB` bit 1. Checked at `00:3560`. Cleared by `00:35B1` at `01:4108`.
- **Status.** CONFIRMED
- **Evidence.** `0B:500E`, `00:3560`, `00:35B1`, `05:437F`, `08:4139`.
- **Test.** `trainer_lock_lasts_one_opponent_turn`

### R-TRAINER-05: Attached damage-modifier trainers
- **Rule.** The +10 and −20 trainers stay attached. They are counted per slot and move with the
  creature. At between-turns, T's +10 trainers are discarded (so they last for the turn they were
  played) and N's −20 trainers are discarded (so they last through the opponent's following turn).
- **Parameters.** `01:6CF1` (T's `$D8` cards, `E0`), `01:6D03` (N's `$D9` cards, `DA`).
- **Status.** UNCERTAIN. Only the call sites were traced.
- **Evidence.** `01:6CF1`, `01:6D03`, `00:19C9/19DE`.
- **Test.** `plus10_trainer_discarded_end_of_turn`, `minus20_trainer_lasts_opponent_turn`

### R-EFFECT-01: Effect-command framework
- **Rule.** Every attack, Power and trainer has a list of entries, each a phase byte plus a handler,
  ending with 0. The dispatcher runs the handler for the requested phase. A missing list or a
  missing phase counts as success. A handler signals failure or cancel with carry, optionally with an
  error.
- **Parameters.** Phases:

  | Phase | Meaning |
  |---|---|
  | 1 | Check (for a Power, marks it passive) |
  | 2 | Selection before the announcement |
  | 3 | Main / before damage |
  | 4 | After damage |
  | 5 | Selection or coin after the announcement |
  | 6 | Costs |
  | 7 | On placement |
  | 8 | AI target selection |
  | 9 | AI damage estimate |
  | 10 | AI |

- **Status.** CORRECTED. The example list address and the call sites were fixed.
- **Evidence.** `00:2F90-2FE2`, list `06:48B8`.
- **Test.** `effect_phases_called_in_pipeline_order`

### R-EFFECT-02: Attack resolution pipeline
- **Rule.** An attack resolves in this order:
  1. Phase 1 check (failure: no attack, the turn continues).
  2. Accuracy coin from the attacker's own substatus. The result is cached for the turn, and tails
     ends the turn with no costs paid.
  3. Phase 2.
  4. Phase 6 costs.
  5. Confusion coin.
  6. Announcement.
  7. Phase 5. Its carry is ignored.
  8. Prevention check (R-DMG-12).
  9. Phase 3.
  10. Damage calculation.
  11. Coin prevention hook (R-DMG-13).
  12. Apply damage.
  13. Phase 4.
  14. Retaliation hook.
  15. Status queue.
  16. Knock-out handler.
- **Parameters.** The accuracy cache `cc0a` is reset per turn. `ccb7` bit 7 marks an attack with no
  direct damage. `ccb7 == 4` means a Power.
- **Status.** CONFIRMED
- **Evidence.** `00:1695-177D`, `00:33CB`, `00:33B7`, `00:33BE/33E2`.
- **Test.** `attack_pipeline_order`, `accuracy_coin_cached_for_turn`

### R-EFFECT-03: Coin flips
- **Rule.** Each coin is 50/50. A single flip returns heads as true. A multi-flip always flips all N
  coins and returns the number of heads.
- **Status.** CONFIRMED
- **Evidence.** `00:3034`, `00:3028`, `01:72EC-7328`, `01:7392-73AB`.
- **Test.** `flip_coins_counts_heads_with_seeded_rng`

### R-EFFECT-04: Reusable effect kinds
- **Rule.** The source builds effects from shared helpers. An original card set may reuse these:
  - apply a condition, always or on heads;
  - damage every Benched creature on a side, with no weakness or resistance;
  - heal the Active, capped at max HP, requiring some damage;
  - set or add to the damage;
  - protect itself next turn;
  - put an effect on the defender (can't attack, can't retreat, accuracy, disable one attack);
  - discard energy as a cost;
  - switch out the defender (with an empty opposing Bench, the attack still resolves and only the
    switch is skipped);
  - search the deck, draw, or recover cards;
  - change weakness or resistance;
  - devolve;
  - lock the opponent's trainers.
- **Status.** CORRECTED. The force-switch is a phase-5 selection, not a precondition, and healing
  targets slot 0.
- **Evidence.** `0B:4117`, `0B:4221`, `0B:425B`, `0B:4140`, `0B:4149`, `0B:6539`, `0B:500E`.
- **Test.** `bench_damage_ignores_weakness`, `heal_capped_at_max_hp`, `force_switch_with_empty_bench_still_attacks`

### R-EFFECT-05: Attack usable once while in play *(verifier-found)*
- **Rule.** An attack may be flagged "only once while this creature stays in play". The flag is set
  during the cost phase, so it counts even if the confusion coin fails. It survives turn changes and
  switching. Evolving or a new placement resets it.
- **Parameters.** `+C2` bit 6.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `0B:6B1F`, `0B:6B2C`, `06:4DBA`.
- **Test.** `once_per_stay_attack_rejected_second_time`

### R-EFFECT-06: Copy one of the defender's attacks *(verifier-found)*
- **Rule.** An effect can run one of the defending creature's attacks through the same pipeline. It
  cannot copy the copying attack itself.
- **Status.** CONFIRMED (verifier-found)
- **Evidence.** `0B:6D20-6D7E`.
- **Test.** `copy_attack_runs_defender_effect_list`

---

## 11. Powers (R-POWER)

### R-POWER-01: Activating a Power
- **Rule.** The player chooses an in-play creature whose first ability is a Power.
  - A passive Power (one with a phase-1 entry) cannot be activated.
  - An activated Power runs, in order: the usability check (conditions, blocker, once per turn,
    specific checks), selection, then the effect.

  A Power has no cost phase, no confusion coin and no after-damage phase. It may be used on any turn
  before attacking, and it does not end the turn.
- **Status.** CONFIRMED
- **Evidence.** `01:6483-64BF`, `00:17AB-17D3`, `01:4392`.
- **Test.** `passive_power_not_activatable`, `power_does_not_end_turn`

### R-POWER-02: Conditions disable Powers
- **Rule.** See R-STATUS-05. Confused, Asleep or Paralyzed on the Active disables its Power. Poison
  does not.
- **Status.** CONFIRMED
- **Evidence.** `00:34A6-34B5` and 17 callers.
- **Test.** `asleep_active_power_refused`

### R-POWER-03: Power blocker and the "in play" counting rule
- **Rule.** While a power blocker is in play on either side, no other Power works, whether activated,
  passive or on-play. A copy counts as in play when it is on the Bench, or when it is the Active
  without Confused, Asleep or Paralyzed. Poison does not matter. The same counting rule applies to
  every "is Power X in play" test. The blocker does not block itself.
- **Status.** CONFIRMED
- **Evidence.** `00:34B7-34DB`, `00:34F6`, `00:15BE`.
- **Test.** `bench_power_blocker_stops_other_powers`, `asleep_active_blocker_does_not_block`

### R-POWER-04: Once per turn
- **Rule.** Most activated Powers can be used once per turn per creature. The flag moves with the
  creature when it switches slots. It is cleared at the start of the owner's turn, and also by
  evolving or by a new placement.
- **Parameters.** `+C2` bit 5. It is reset by `01:71A3`, which runs only from turn counter 2. That is
  harmless, because the bit starts at 0.
- **Status.** CONFIRMED
- **Evidence.** `0B:4D12/4DB1/4E5C/5808/62A5/6ADC` (checks), `0B:4D6E/4E0A/4E9F/58C2/62BB/6AF3`
  (sets), `00:14C2`.
- **Test.** `once_per_turn_power_second_use_rejected`

### R-POWER-05: "Not on the turn it was played" (optional per Power)
- **Rule.** Some Powers also require the creature to be settled (in play since the start of its
  owner's turn). Such a Power is never usable on either player's first turn, because the settled
  marking is skipped while the turn counter is below 2.
- **Status.** CORRECTED. The verifier added the first-turn consequence.
- **Evidence.** `0B:5293-52AD`, `01:4231-4236`, `00:1410`, `00:1340`.
- **Test.** `settled_power_unusable_on_turn_played`

### R-POWER-06: Powers that trigger on placement
- **Rule.** When a creature with a phase-7 Power is placed on the Bench **or evolves**, the effect
  fires automatically, unless a power blocker is in play. A blocker is not blocked by itself. When a
  blocker enters play, the opponent's per-slot colour changes made by Powers are removed.
- **Status.** CORRECTED. The original claim missed the evolve trigger and the colour reset.
- **Evidence.** `00:1583-1611`, called from `01:451B` and `01:457F`. `00:3690/36A0`.
- **Test.** `on_play_power_fires_on_bench_and_evolve`

### R-POWER-07: Passive Power kinds seen in the engine
- **Rule.** The engine core references these generic passive Powers:
  - free-attach (R-ENERGY-02);
  - retreat reducer (R-RETREAT-02);
  - evolution lock (R-EVO-09);
  - condition immunity (R-STATUS-03);
  - damage reduction: prevent ≥30, or halve (R-DMG-09 / R-DMG-04);
  - whole-attack prevention (R-DMG-12);
  - coin prevention (R-DMG-13);
  - retaliation (R-DMG-14);
  - colour change (R-DMG-03);
  - power blocker (R-POWER-03).

  The engine should expose these as hooks that original cards can use.
- **Status.** CONFIRMED
- **Evidence.** See the rules listed above.
- **Test.** Covered by the tests of those rules.

---

## 12. AI opponent (R-AI)

These describe the observed policy. A clean-room AI needs only to be **legal** and reasonable. These
notes give the decision order and the weightings to copy where helpful.

| ID | Rule (summary) | Status | Test |
|---|---|---|---|
| R-AI-01 | There are five entry points: take a turn, place at setup, choose a creature when forced to switch, choose a replacement after a knock-out, pick prizes. The AI acts only through the same action primitives as a human (`00:2B42-2B8A`, `05:4668`, `01:687E`). Some legality checks are done by the AI caller, not by the primitive, so the engine must validate every AI action. | CONFIRMED | `ai_never_emits_illegal_action` |
| R-AI-02 | Turn order is fixed (see AI policy notes). The phase-14 trainers run inside attack selection, after the attack is chosen. The second pass stops at phase 13 (`05:468B-4785`, `05:6A48`). | CORRECTED | `ai_turn_benches_before_attach_before_attack` |
| R-AI-03 | Random mistakes: weaker AIs skip a valid trainer or Power play with probability 1/4, and some with 1/2. Strong AIs never skip. Benching, evolving, energy, retreat and attacking are deterministic (`05:743B-7473`). | CONFIRMED | `ai_easy_skips_quarter_of_valid_trainer_plays` |
| R-AI-04 | Benching a Basic: score = 130; +50 if fewer than 4 in play, otherwise −20; +20 if the Active could be knocked out next turn; +20 if usable energy is in hand; +20 if the evolution is in hand; +10 if the evolution is in the deck. Bench at ≥180 and fewer than 6 in play (`05:5EAE-5F4B`). | CONFIRMED | `ai_benches_basic_under_four_in_play` |
| R-AI-05 | Evolving: score = 128, evolve at ≥133. It weighs: can attack after evolving, a knock-out gained or lost, leaving knock-out range, a cure for the Active's condition, the next stage being available, damage taken, and a small per-card AI byte (`05:5F4C-611F`). | CONFIRMED | `ai_evolves_statused_active` |
| R-AI-06 | Energy target: score = 128 for each creature, attach at ≥133, ties go to the lowest slot. A creature that can't use any energy in hand is never chosen. Only the Active gets the knock-out bonus for one missing energy (`05:64E8-67DF`, `05:653A`, `05:674E`). | CORRECTED | `ai_attaches_to_active_for_ko` |
| R-AI-07 | Retreat: at most once per turn, never after a failed confused retreat. Score = 128, retreat at ≥131, built from poison, confusion, knock-out threat, weakness and resistance matchups, a better creature on the Bench, retreat cost, the prize race, and turns without dealing damage (`05:58B2-5B32`). | CORRECTED | `ai_retreats_threatened_poisoned_active` |
| R-AI-08 | The AI plays a self-switch trainer instead of paying when the retreat cost is ≥3 or there isn't enough energy. In an urgent retreat only, and only if no energy was attached yet, it attaches one first when exactly 1 unit short (`08:462E`, `05:5D4F`). | CORRECTED | `ai_uses_switch_trainer_for_cost_three` |
| R-AI-09 | Choosing a creature to bring in (retreat, forced switch, replacement after a knock-out): score = 50 plus knock-out potential, damage, weakness and resistance, retreat cost, threat and HP. A creature at 0 HP scores 0. The highest score wins, and ties go to the later slot (`05:5B72-5D4C`). | CORRECTED | `ai_promotes_creature_that_can_ko` |
| R-AI-10 | Attack choice: score = 80 for each attack. An unusable attack scores 0. Adjustments include +20 for a knock-out, +damage/10, recoil, discard cost, condition value, and a prize-race check for Bench or self-knock-out effects. A tie goes to the 2nd attack. It attacks only if the best score is ≥80; otherwise it ends the turn (`05:69F8-6DCC`). | CORRECTED | `ai_picks_attack_that_kos`, `ai_ends_turn_with_useless_attack` |
| R-AI-11 | The +10 trainer is played when it turns a non-knock-out into a knock-out, or 30% of the time afterwards. The −20 trainer is played when it prevents the Active being knocked out. The heal-20 trainer is played on the Active when it lets it survive and the AI isn't retreating (`08:4501`, `08:45A5`, `08:4406`, `08:41D1`, `08:4204`). | CORRECTED | `ai_plays_plus10_only_for_ko` |
| R-AI-12 | Draw trainers: draw-2 needs ≥10 cards in the deck. Discard-hand-draw-7 needs ≥15 and a score ≥60. If it is played, the turn sequence runs again (`08:4878`, `08:4CC1-4D9B`). | CORRECTED | `ai_skips_draw_trainer_on_small_deck` |
| R-AI-13 | Force-switch trainer: once per turn, only if the opponent has a Bench and the Active can't already knock out the defender. It prefers a creature it can knock out, then one weak to its colour, then one with no energy, then the lowest HP (`08:467E-4821`). | CORRECTED | `ai_force_switches_ko_able_bench` |
| R-AI-14 | Setup: the AI places every Basic in hand order. The first becomes the Active, up to 6 in play. The AI makes no mulligan choice (`05:4226-4249`). | CONFIRMED | `ai_setup_first_basic_active` |
| R-AI-15 | Strong AIs stack their opening by reshuffling until the hand and next draws are good. **Do not implement.** This is unfair. A disabled prize filter (`cp a`) is also present (`05:72AF-7377`). | CONFIRMED | n/a (not implemented) |
| R-AI-16 | Prizes: picked at random among those remaining (`05:41E5-421D`). | CONFIRMED | `ai_prize_pick_is_remaining_slot` |
| R-AI-17 | Powers: each Power has its own heuristic. All Powers are skipped while a blocker is in play or the random gate fires. A condition on the Active skips only the Active's own Power (`08:637F-6401`). | CORRECTED | `ai_skips_active_power_when_confused` |
| R-AI-18 | *(verifier-found)* A no-damage counter raises the retreat score. An anti-stall mode against one particular defender exists (card-specific, skip it) (`05:6A62-6A84`, `05:5649`). | CONFIRMED | `ai_no_damage_counter_increments` |

---

## 13. Not in this game

| Mechanic | Evidence of absence | Recommendation |
|---|---|---|
| **Burned** condition | `+F0` encodes only nibble 1/2/3 and poison bits. No writer stores another nibble. Between-turns `01:6C5C` handles only poison, sleep and paralysis. No condition text exists. | Omit. Could be added later as an optional extension behind a ruleset flag. |
| Conditions on Benched creatures | One condition byte per player for the Active only, wiped on every switch (`00:13C6`). | Omit; this is by design. |
| Confusion wearing off on its own | No nibble-1 handler between turns. | Omit. |
| Coin flip to recover from Paralysis | Removed without a coin at `01:6CAB`. | Omit. |
| First player skips the first draw | The draw at `01:4241` is unconditional. | Omit. A modern-rules variant could be optional. |
| First player can't attack or play trainers on turn 0 | `cc06` is read only for the evolution gate (`01:4231`) and practice scripts. | Omit. A modern-rules variant could be optional. |
| Choosing to go first or second | Heads or tails sets the turn directly (`01:4C41-4C46`). | Omit. |
| Optional mulligan, or the opponent drawing extra cards on a mulligan | `01:4B8B-4BAF` jumps straight to placement with no prompt. | Omit. |
| Retreat limited to once per turn, or retreat ending the turn | The non-confused path reads no flag and returns to the menu (`01:43F5-4420`). | Omit. |
| Hand-size limit or discard at end of turn | `00:1088` has no bound. | Omit. |
| Trainer subtypes (support, stadium, tool) or per-turn trainer limits | A single trainer type `$10` and no counter (`00:1859`, `01:7193`). | Omit. The attached ±damage trainers are the only trainers that stay in play. |
| Creatures worth more than one prize | Exactly one `inc b` per knock-out (`01:7074`). | Omit. |
| Turn or time limit in normal duels | The 15-turn exit `01:411E` is gated by `cc13` (scripted demo only). Note: `cc09 == $80` simply means "against the AI", contrary to the investigator. | Omit. |
| Losing by deck-out through an effect | `cc07` is set only at `01:4248` and `01:6F79`. | Omit. |
| A plain draw ending the duel | `cc07 = 3` always leads to sudden death (`01:41B4`). | Omit. Sudden death is implemented as written. |
| Other win conditions (concession, alternate wins) | `cc07` has no other writers. | Omit. A UI "concede" button could be offered as an extension. |
| A variable weakness multiplier, resistance other than −30, or weakness and resistance on Bench damage | Fixed shift at `00:1933`. Only `$FFE2`. `00:1A53` never calls the weakness or resistance helpers. | Omit. |
| Checking that the stage goes up by one when evolving | Only the name is matched (`00:135C`). | Omit, but keep data consistent in the card set. |
| A per-turn evolution limit | No counter (`01:7193`). | Omit. |
| A confirmation prompt on Done | `01:51ED` is a practice-duel hook. | Omit. The UI may add its own optional confirmation. |
| The AI choosing whether to mulligan, or AI search and lookahead | There is no mulligan handler (`05:4668`). Scores are additive with a one-move threat check (`05:73B1`). | Omit. A smarter AI is optional. |

---

## 14. Engine spec summary (test-first checklist)

1. Each deck must have exactly 60 cards, at least 1 Basic creature (Trainer-as-Basic does not count), and at most 4 cards per name (basic energy exempt; the special 2-unit energy is limited).
2. Setup: reset, shuffle, and draw 7 for each player.
3. Mulligan: a hand without a Basic is revealed, reshuffled and redrawn until it has one. It is forced, with 0 bonus cards for the opponent. If neither player has a Basic, setup restarts for both. Trainer-as-Basic does not count.
4. Placement: human first, then the opponent. Exactly 1 Basic as Active (it can't be cancelled), then 0 to 5 on the Bench. Only Basics or Trainer-as-Basic cards are allowed.
5. Prizes: N prize cards from the top of the deck, set after placement. N is configurable from 1 to 6 (default 6). Sudden death uses 1.
6. First player: one coin after prizes. Heads means the flipper goes first. No choice is offered.
7. Every turn starts with a draw of 1, including turn 0. An empty deck at that draw means the turn player loses immediately.
8. Effect draws are capped at the cards left in the deck. Search effects (`0B:42E0` callers) are refused on an empty deck. Plain draw effects can still be played and draw nothing (`01:4939` clamps, `01:4973` message). Neither case is ever a loss.
9. Turn start: clear own-turn substatus. If the turn counter ≥ 2, mark all of the turn player's in-play creatures as settled and clear their Power-used flags. Reset energy-attached, retreat-failed and accuracy-cache. Then draw.
10. Main phase: any number of actions in any order (bench, evolve, attach, trainer, Power, retreat). Only an attack or Done ends the turn.
11. Done ends the turn without a prompt and clears the opponent's last-attack and disabled-attack memory.
12. Turn end: check the result, clear the trainer lock and `+E8`, run between-turns, check the result, increment the turn counter, swap.
13. There is no first-turn restriction on attacking, trainers, energy, retreat or Powers (except settled-only Powers).
14. Bench: at most 6 creatures in play. A placed creature has full HP and cleared flags. Conditions are cleared when it lands in slot 0.
15. There is no hand-size limit.
16. Energy: one attachment per turn, to the Active or the Bench. Cancelling doesn't use it up.
17. Free-attach Power: a matching energy onto a matching-colour creature doesn't count toward the limit, while the holder (own side) is in play and no blocker is in play.
18. Energy counting: a basic energy is 1 unit of its colour. The special energy is 2 colorless units.
19. Attack cost: each typed requirement must be met by that colour, and the total minus the typed sum must cover colorless. Empty slots and Powers are never attacks.
20. An attack is blocked by, in this order: an attack-lock substatus, Asleep or Paralyzed, no attacks, not enough energy, or the specific disabled attack.
21. A committed attack always ends the turn, including a confusion tails or an accuracy miss. Only cancelling or a failed precondition returns to the menu.
22. Attack pipeline: check → accuracy coin (cached per turn) → select → costs → confusion coin → phase 5 → prevention check → before-damage → damage calc → coin prevention → apply → after-damage → retaliation → status queue → knock-out handler.
23. Damage order: base → attacker double → weakness ×2 → resistance −30 → +10 per +10 trainer → −20 per −20 trainer on the defender → defender prevention/reduction → attacker-placed reduction → clamp to 0. Clamp only at the end.
24. Base damage 0 skips all modifiers.
25. Weakness and resistance: use the defender's changed weakness or resistance if set. The attacker's changed colour applies only while its Power works (no condition, no blocker). Trainer-as-Basic is colour index 6.
26. Effect damage never uses weakness or resistance. +10 applies only when the target is the Active (including Power damage). −20 applies on any slot (except Power damage). The defender's reductions apply on the Active. The Bench reduction Power applies after the clamp.
27. An ignore-weakness-and-resistance flag skips only those two steps.
28. Defender reduction: the prevention flag, else one "until next turn" effect (0 / −10 / −20 / <40 → 0 / halve), else the defender's reduction Power (≥30 → 0, or halve). Then the attacker-placed −20/−10.
29. Halving rounds down to a multiple of 10.
30. Self-damage (confusion 20, recoil) uses its own weakness and resistance against its own colour, plus its own trainers, then the clamp. No substatus modifiers apply and it can't be prevented.
31. HP is stored as remaining points and floored at 0. A knock-out happens exactly at HP 0.
32. Report the weakness-applied and resistance-applied flags in the attack result.
33. Conditions: one byte for the Active. Confused, Asleep and Paralyzed exclude each other. Poison or Double-Poison stacks with them. The Bench never has conditions.
34. Applying a condition: the newest nibble condition replaces the old one and keeps poison. Poison overwrites the poison level.
35. Conditions are queued after damage. A negated attack drops conditions on the defender but keeps the attacker's self-conditions. Trainer-as-Basic and immunity-Power holders are immune.
36. Asleep or Paralyzed: can't attack or retreat. Confused may try both, with a coin.
37. Confused, Asleep or Paralyzed disables the Active's Powers, including passive "in play" counting. Poison doesn't.
38. Confused attack: costs are paid, then a coin. Tails means 20 self-damage, a knock-out check and the end of the turn.
39. Between turns (T = the player who just ended): T's poison → sleep coin → paralysis cure → discard T's +10 trainers → N's poison → sleep coin → discard N's −20 trainers → knock-outs. Skip the condition steps if there is nothing to process.
40. Poison: 10 (double: 20) at every between-turns step, raw with no modifiers. It never expires.
41. Sleep: a coin at every between-turns step after poison. Heads wakes it.
42. Paralysis: removed at the end of the owner's turn, with no coin.
43. Confusion never expires on its own.
44. All conditions and effects are cleared by switching, by the Active evolving, by placement into an empty Active spot, and by cure effects.
45. Retreat preconditions in order: can't-retreat substatus, Asleep or Paralyzed, no live Bench, Active is Trainer-as-Basic, energy < cost. Poison and Confusion are OK.
46. Retreat cost: printed cost minus own-Bench reducers (ignored with a blocker), floored at 0. The player chooses the energy. Stop once paid ≥ cost. Special energy counts 2 with no refund.
47. Confused retreat: discard first, then a coin. Tails means it stays and the retreat-failed flag blocks confused retries this turn. The flip happens even at cost 0.
48. A switch clears the outgoing Active's conditions and effects. Damage, cards, stage, flags, colour override and trainer counters move with the creature.
49. No per-turn retreat limit. Retreating doesn't end the turn.
50. Cancelling a retreat costs nothing.
51. Evolve: the hand card's evolves-from name must match the in-play creature's name. There is no stage arithmetic.
52. Evolve requires settled status: not on turns 0 or 1, not on the turn the creature was placed, and not twice in one turn. There is no per-turn evolution limit.
53. Evolve keeps damage (HP += newMax − oldMax), attached cards and the lower stages underneath.
54. Evolve clears the slot flags (including the Power-used flag) and the colour override. Evolving the Active also clears its conditions and effects.
55. Evolution-lock Power (either side, counted by the in-play rule) blocks all evolution unless a blocker is in play. "No target" and "not this turn" errors take priority.
56. Optional stage-skip trainer and devolve effects follow R-EVO-10 and R-EVO-12.
57. Knock-out check: runs once after the full attack, after the confusion self-hit, after damaging effects, and after between-turns.
58. Knock-out handler order: counter-KO → clear linked effects → T takes prizes → (early win) → N takes prizes → (early loss) → N replaces, then T replaces → result table → always discard all 0-HP creatures with their stacks, then compact.
59. One prize per knocked-out non-Trainer-as-Basic creature, capped at the prizes left. A human picks the prize slot, the AI picks at random. Prizes go to the hand.
60. Replacement: choose from the live Bench and it can't be cancelled. No live Bench means "no creatures".
61. Result = sign((T took last prize + N has no creatures) − (N took last prize + T has no creatures)). Positive means T wins, negative means T loses, equal and nonzero means a draw.
62. A draw restarts as sudden death: 1 prize each, turn counter 0, full setup and a new coin flip. It can repeat.
63. A result set mid-turn ends the turn at once and skips between-turns. A result set between turns ends the duel before the next turn.
64. Counter-KO effect: if the defender with the effect is knocked out by the attack, the attacker is knocked out too (not between turns).
65. A knock-out clears effects (E8 ∈ {3,5,6,7,$12}) on the other side that were placed by the knocked-out Active.
66. Shuffle permutes only the remaining deck. Searching removes one card. A card can be returned to the top.
67. Discard pile: ordered, public, new cards on top. Only cards in hand can be discarded from hand.
68. Trainers: unlimited per turn. Order is lock check → check → select (cancel keeps the card) → costs → effect → discard if still in hand.
69. Trainer lock: set on the opponent player and lasts through their next turn. It survives switching and is set even if the attack was negated.
70. Attached +10 and −20 trainers are counted per slot. +10 is discarded at the end of the owner's turn. −20 is discarded at the end of the opponent's following turn (UNCERTAIN).
71. Effects are lists of (phase, handler). A missing phase counts as success. Handlers may fail with an error.
72. Coins are 50/50. A multi-flip flips all N and returns the heads count. Use a seedable RNG.
73. Powers: activated only (not passive). The flow is usability → select → effect. A Power doesn't end the turn and can be used on any turn before attacking.
74. Power blocker: while it is in play (Bench, or an Active without a nibble condition), all other Powers are off. It doesn't block itself.
75. Once-per-turn Powers: a per-slot flag that moves with the creature and is cleared at turn start, by evolving and by placement.
76. Optional settled-only Powers can't be used on the turn the creature was played or evolved, nor on either first turn.
77. Phase-7 on-play Powers fire on bench placement and on evolve unless a blocker is in play. A blocker entering play wipes the opponent's Power-made colour changes.
78. Optional attack flag "once while in play", set at the cost phase.
79. Prevention hooks: a whole-attack prevention substatus or Power (negates damage, effect damage to the Active, and conditions), a coin-prevention Power, and a retaliation Power (flat 10).
80. The AI must act only through engine actions, and every AI action must pass the same validation as a human action.

---

## 15. Open questions

- What the AI's setup and replacement handlers filter on (whether they skip 0-HP creatures). The engine
  must enforce the rule regardless.
- The linked-duel first-player and setup paths (`01:41F7`, `01:4CEB`). Not needed for single player.
- Who reads stage value 3, which the stage-skip trainer writes (`0B:7716`).
- The 8-bit HP formula on evolution when the new max is lower than the old one (it could wrap). The
  clean engine should use signed or saturating arithmetic.
- Behaviour when the damage high byte is ≥ `$81`, and the halving routine for values > 255. Neither
  happens with real data. The clean engine should use plain integers.
- Exactly when the attached ±damage trainers are discarded (`01:6CF1/6D03`). This is R-TRAINER-05,
  still UNCERTAIN.
- The meaning of bit 6 in a card's location byte (limbo during play or search).
- The full set of sources for the `cccd` prevention codes, and which effects set `+E7 = $16`,
  `+E8 = 9` and `+E8 ∈ {3,5,6,7,$12}`. For an original card set, the engine defines its own effects
  on these hooks.
- Which activated Powers skip the once-per-turn flag (only 6 set it).
- The exact AI hint flags (attack flags 0-`$11`). The mapping from flags 1/2/3 to conditions is unclear.
- The remaining AI trainer checks (search, revive, energy removal), the deck-specific AI turn
  routines, and SRAM `$A00A` (a debug or options flag).
- The step between phases 2 and 6 in trainer play (`00:66FF` / link 6). It looks like link sync or a
  display step, not a rule.

---

## 16. AI policy notes

In our own words, this is the default AI's turn as a fixed pipeline. Every step is greedy: each
candidate gets an additive score from a base, and the AI acts when the score crosses a fixed
threshold.

1. Reset the per-turn AI flags.
2. Play "hand reset or gamble draw" trainers.
3. Use Powers. Each Power has its own heuristic. All are skipped if a blocker is in play.
4. Play search and draw trainers.
5. Bench Basics: always while fewer than 4 are in play.
6. Evolve, where it helps (a knock-out, survival, a cure, attack readiness).
7. Play disruption and support trainers: energy removal, revive, heal, force-switch, stage-skip.
8. Decide whether to retreat, at most once per turn. If so, use a self-switch trainer when paying is
   expensive. The replacement is the best-scored Bench creature.
9. Play more heal and force-switch trainers, then energy recovery or energy search trainers.
10. Attach one energy, to the best target: the Active first when one more energy enables a knock-out.
11. Bench and evolve again, then use Powers again.
12. Play damage-buff and shield trainers, then (last) the discard-hand-draw-7 trainer. If that was
    played, re-run steps 2-11 once.
13. Score each attack. If the best is good enough (≥ base), play attack-dependent trainers, then
    attack. Otherwise end the turn.

**Difficulty levers** (optional in the clean-room AI):
- A random 1/4 (or 1/2) skip on trainer and Power plays for weaker opponents.
- Strong opponents get a few extra deterministic terms.
- The stacked-opening cheat must **not** be copied.

Prizes are picked at random. Setup places every Basic in hand order.

---

## Appendix: Refuted claims

The verifier rejected no claim outright. These parts of the original claims were wrong and have been
replaced:

- **Done asks for confirmation.** False. That hook is for practice duels only (R-TURN-06).
- **`01:7227` ends the turn after a confusion self-hit.** False. It clears the opponent's
  last-attack variables. The turn ends because of the no-carry return (R-STATUS-06).
- **Attack blocks list.** The disabled-attack check `00:3398` was missing, and so were the coin
  failures that still end the turn (R-TURN-05).
- **Linked sudden death decides the first player from link data.** False. A coin still decides it
  (R-TURN-08).
- **Prize count 0..6.** False. It is 1 to 6 (R-SETUP-05).
- **Knock-out handler step 8 "result OR discard".** False. Discarding always happens (R-KO-04).
- **The free-attach exception uses a plain status test.** It was refined: the holder test is
  nibble-only, the blocker counting rule applies, the colour is the effective colour, and the AI
  executor always sets the flag (R-ENERGY-02).
- **The changed attacker colour is always used.** False. It is gated by the holder's condition and
  by the blocker (R-DMG-03).
- **Effect damage +10 and −20 details.** The +10 also applies to Power damage, and the Bench
  reduction Power applies after the clamp (R-DMG-04).
- **The defender's reduction effects and Powers all stack.** False. They exclude each other
  (R-DMG-09).
- **Effect-list example address `06:48BE` with pointer `$0D09`.** Wrong. It is `06:48B8` with
  pointer `$4D09` (R-EFFECT-01).
- **Force-switching the defender requires an opposing Bench.** False. With an empty Bench the
  attack still resolves (R-EFFECT-04).
- **On-play Powers fire only on bench placement.** False. They also fire on evolve (R-POWER-06).
- **Evolution lock lifted by the blocker under any condition.** The blocker counting rule applies
  (R-EVO-09).
- **The "no turn limit" claim said `cc09 == $80` means practice duel.** False. It means a duel
  against the AI. The 15-turn exit is gated by `cc13` instead.
- **The power-blocked message lists four conditions.** False. It lists three. The
  four-condition text is the cure message.
- **Status AI-hint and AI rules.** Several weightings and gates were corrected. See the CORRECTED
  rows in section 12.
