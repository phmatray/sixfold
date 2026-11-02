# Boosters / Boosters

How a pack is built, what each booster favours, and the odds that follow. The code is
`meta/src/booster.rs`. The tests there check this page: `slot_rates_match_the_design` opens 12,000
seeded packs of every booster. The numbers below come from `booster_report`, which opens 40,000 packs per
booster and completes each set 400 times:

```bash
cargo test --release -p sixfold-engine booster_report -- --ignored --nocapture
```

*La version française suit la version anglaise.*

---

## English

### 1. Goals

- **No dead slot.** Basic energy is unlimited and never stored, so the old energy slot paid nothing. It
  is now a sixth Common.
- **A themed pack is about its themes.** In the 1.x packs, Ember drew evenly from everything it held:
  only 44 % of its Commons were Ember or Stone, and 70 % of its Rares were Gear cards. Now the share of
  each slot is chosen on purpose.
- **A chase card.** Holo Rares (the Awaken 2 Hexons) are the cards a deck is built around. They turn up
  in three packs of five, with their own foil, so the Rare slot is a Hexon more often than not (it was
  one pack in three until 1.3, when the slot was a creature only a third of the time).
- **Transparent odds.** The shop and the pack screen show the numbers below, taken from the same
  constants the engine uses.
- **Stay in the economy band.** Pack prices and income don't change (`economy_sim`: 5.7 packs an hour
  on a first playthrough and 5.2 while exploring, target 3.5–6). Completing a set still takes play,
  packs and crafting together.

### 2. The pack

10 cards, in reveal order, never the same card twice:

| Slots | Rarity | How each slot is drawn |
|---|---|---|
| 1–6 | Common | a group by **share** (below), then a card of it, evenly |
| 7–9 | Uncommon | the same |
| 10 | Rare slot | **Holo Rare in 3 packs of 5** (60 %), otherwise a Rare (40 %) |

- **Holo Rare**: the 12 Awaken 2 Hexons. A themed pack only holds its own types' Holos; Wild holds
  all 12.
- **Rare**: the 8 Rare Gear cards and the special energy (9 cards, the same in every pack).
- **Duplicate protection**: the Rare slot skips any card you already hold 4 of (a full playset) while
  another card of that slot is left. Rares can't be crafted until you've pulled one, so this slot must
  keep paying. Commons and Uncommons sell back and can be crafted, so they have no protection.
- **No pity timer.** With a Holo in 3 packs of 5, the chance of none in 3 packs in a row is 6.4 %, and in
  6 packs 0.4 %. That is short enough not to need a counter in the save.

### 3. Shares in a themed pack

Every Common and Uncommon slot first picks a group:

| Group | Share | What it holds |
|---|---|---|
| Own types | **60 %** | Hexons of the pack's two types |
| Neutral | **35 %** | Gear cards, the special energy and Neutral Hexons |
| Other types | **5 %** | Hexons of the four other types: a stray card, so a themed pack never fully locks you out |

60 / 35 matches a deck: about 20 Hexons for 12–14 Gear cards. Stone's own types are Stone **and
Neutral**, so its neutral group is Gear cards only.

| Booster | Own types | Holo Rares |
|---|---|---|
| Ember | Ember, Stone | Pyrequill, Solarlynx, Knollossus, Cragmastiff |
| Tide | Tide, Dream | Tidemaw, Gulfwyrm, Wraithelle, Hexarch |
| Grove | Grove, Storm | Elderhart, Bogcrown, Fulgurex |
| Storm | Storm, Tide | Tidemaw, Gulfwyrm, Fulgurex |
| Stone | Stone, Neutral | Knollossus, Cragmastiff, Warrenking |
| Dream | Dream, Grove | Wraithelle, Hexarch, Elderhart, Bogcrown |
| **Wild** | every type, **evenly** | all 12 |

**Wild** has no shares: each slot draws evenly from every card of its rarity. It is the pack for filling
gaps across the whole set.

### 4. Per-card odds

The chance that a given card is in one pack, with a fresh collection:

| Card | Ember | Tide | Grove | Storm | Stone | Dream | Wild |
|---|---|---|---|---|---|---|---|
| own-type Common | 45 % (8) | 60 % (6) | 40 % (9) | 45 % (8) | 45 % (8) | 51 % (7) | — |
| neutral Common | 21 % (10) | 21 % (10) | 21 % (10) | 21 % (10) | 35 % (6) | 21 % (10) | — |
| other-type Common | 2.0 % | 1.7 % | 2.1 % | 2.0 % | 1.6 % | 1.9 % | — |
| any Common (33) | | | | | | | 18.2 % |
| own-type Uncommon | 20 % (9) | 18 % (10) | 22.5 % (8) | 20 % (9) | 18 % (10) | 20 % (9) | — |
| neutral Uncommon | 6.6 % (16) | 6.6 % (16) | 6.6 % (16) | 6.6 % (16) | 9.5 % (11) | 6.6 % (16) | — |
| other-type Uncommon | 0.8 % | 0.9 % | 0.8 % | 0.8 % | 0.7 % | 0.8 % | — |
| any Uncommon (43) | | | | | | | 7.0 % |
| each Rare (9) | 4.4 % | 4.4 % | 4.4 % | 4.4 % | 4.4 % | 4.4 % | 4.4 % |
| each Holo Rare | 15 % (4) | 15 % (4) | 20 % (3) | 20 % (3) | 20 % (3) | 15 % (4) | 5.0 % (12) |

(n) is how many cards share that line. A Holo's odds are exactly ⅗ ÷ n and a Rare's ⅖ ÷ 9; the rest are
measured.

### 5. Packs to complete a set

These are the packs needed for one copy of every card a booster is meant for: its own types and its
neutral group (for Wild, the whole set of 97 pack cards). The 4.4 % Rares are what make the tail long:
since 1.3 a themed set takes about 12 packs more (the Rares are rarer), and the whole set through Wild
about 24 fewer (the Holos are commoner).

| Booster | Cards | Mean | Median | 9 runs in 10 by |
|---|---|---|---|---|
| Ember, Tide, Grove, Storm, Dream | 55–56 | 64 | 62 | 87 |
| Stone | 47 | 57 | 56 | 79 |
| Wild (whole set) | 97 | 72 | 69 | 91 |

At about 5 packs an hour, one theme takes roughly 12 hours from packs alone. Crafting the last Commons
and Uncommons (⬡ 80 / 240 for one you've never owned) cuts that well short, and singles and selling help
too. That is the "play, packs and crafting together" of `docs/WORLD_DESIGN.md` §3.4.

### 6. Effect on the campaign

The boosters made the campaign easier. Over 16 pack seeds with no duel budget, the automatic
playthrough of `meta/src/playtest.rs` (`campaign_sweep`) took a median of 52 duels (40–105) with
these packs, against 65 (44–100) with the 1.x packs: its greedy deck builder gets more cards of the
types it buys. The 1.3 rules (`docs/BALANCE.md`) make rivals harder to beat and bring the median back
to 62 (38–83). The 1.3 holo rate helps: with the old one pack in three, the same rules take a median of
66. With Tide and Dream's 39 route duelists the campaign takes a median of 66 duels (55–83), against
55 without the 1.3 rules.
Because one seeded run swings this much, the test plays three seeds and requires two to finish within
budget. `economy_sim` doesn't change, since prices and income are untouched.

Unlockable decks (1.9, `docs/BALANCE.md`) are lent, not given: unlocking a deck adds no card to the
collection, so packs stay the only source of cards and none of the numbers above move.

### 7. Changing it

`COMMONS`, `UNCOMMONS`, `HOLO_ODDS` and `SHARES` are public constants. The shop's odds panel and the pack
screen read them, so the UI follows. After a change, run `booster_report` and update §4–5, then run the
campaign playthrough and `economy_sim`.

---

## Français

### 1. Objectifs

- **Aucun emplacement inutile.** L'énergie de base est illimitée et jamais stockée : l'ancien
  emplacement énergie ne rapportait rien. Il devient une sixième Commune.
- **Un booster à thème parle de ses thèmes.** Dans les boosters 1.x, Braise tirait uniformément dans
  tout ce qu'il contenait : 44 % seulement de ses Communes étaient Braise ou Roc, et 70 % de ses Rares
  des Objets. La part de chaque emplacement est désormais choisie.
- **Une carte à chasser.** Les Rares Holo (les Hexons d'Éveil 2) sont les cartes autour desquelles on
  construit un deck. Elles sortent dans trois boosters sur cinq, avec leur propre effet holographique :
  l'emplacement Rare est donc le plus souvent un Hexon (jusqu'à la 1.3, une Holo sortait un booster
  sur trois et l'emplacement n'était un Hexon qu'une fois sur trois).
- **Des probabilités affichées.** La boutique et l'écran des boosters montrent les chiffres ci-dessous,
  tirés des constantes du moteur.
- **Rester dans la fourchette économique.** Prix et revenus inchangés (`economy_sim` : 5,7 boosters par
  heure en première partie, 5,2 en exploration, cible 3,5–6). Compléter une série demande toujours de
  jouer, d'ouvrir des boosters et de fabriquer.

### 2. Le booster

10 cartes, dans l'ordre de révélation, jamais deux fois la même :

| Emplacements | Rareté | Tirage de chaque emplacement |
|---|---|---|
| 1–6 | Commune | un groupe selon la **part** (ci-dessous), puis une carte de ce groupe, à parts égales |
| 7–9 | Peu commune | idem |
| 10 | emplacement Rare | **Rare Holo dans 3 boosters sur 5** (60 %), sinon une Rare (40 %) |

- **Rare Holo** : les 12 Hexons d'Éveil 2. Un booster à thème ne contient que les Holo de ses types ;
  Sauvage contient les 12.
- **Rare** : les 8 Objets Rares et l'énergie spéciale (9 cartes, les mêmes dans tous les boosters).
- **Protection anti-doublons** : l'emplacement Rare saute toute carte que vous avez déjà en 4
  exemplaires (un jeu complet), tant qu'il reste une autre carte possible. Une Rare ne se fabrique
  qu'après en avoir tiré une : cet emplacement doit donc toujours rapporter. Les Communes et Peu
  communes se revendent et se fabriquent, elles n'ont pas de protection.
- **Pas de compteur de malchance.** Avec une Holo dans 3 boosters sur 5, la probabilité de n'en voir
  aucune sur 3 boosters d'affilée est de 6,4 %, et sur 6 boosters de 0,4 %. C'est assez court pour se passer
  d'un compteur dans la sauvegarde.

### 3. Les parts d'un booster à thème

Chaque emplacement Commune et Peu commune choisit d'abord un groupe :

| Groupe | Part | Contenu |
|---|---|---|
| Types du booster | **60 %** | Hexons des deux types du booster |
| Neutre | **35 %** | Objets, énergie spéciale et Hexons Neutres |
| Autres types | **5 %** | Hexons des quatre autres types : une carte égarée, pour qu'un booster à thème ne ferme jamais tout à fait la porte |

60 / 35 correspond à un deck : environ 20 Hexons pour 12 à 14 Objets. Les types du booster Roc sont
Roc **et Neutre**, son groupe neutre ne contient donc que des Objets.

| Booster | Types | Rares Holo |
|---|---|---|
| Braise | Braise, Roc | Pyrequill, Solarlynx, Knollossus, Cragmastiff |
| Marée | Marée, Songe | Tidemaw, Gulfwyrm, Wraithelle, Hexarch |
| Bosquet | Bosquet, Orage | Elderhart, Bogcrown, Fulgurex |
| Orage | Orage, Marée | Tidemaw, Gulfwyrm, Fulgurex |
| Roc | Roc, Neutre | Knollossus, Cragmastiff, Warrenking |
| Songe | Songe, Bosquet | Wraithelle, Hexarch, Elderhart, Bogcrown |
| **Sauvage** | tous les types, **à parts égales** | les 12 |

**Sauvage** n'a pas de parts : chaque emplacement tire à parts égales parmi toutes les cartes de sa
rareté. C'est le booster pour combler les trous de toute la série.

### 4. Probabilités par carte

La probabilité qu'une carte donnée soit dans un booster, avec une collection vide :

| Carte | Braise | Marée | Bosquet | Orage | Roc | Songe | Sauvage |
|---|---|---|---|---|---|---|---|
| Commune de ses types | 45 % (8) | 60 % (6) | 40 % (9) | 45 % (8) | 45 % (8) | 51 % (7) | — |
| Commune neutre | 21 % (10) | 21 % (10) | 21 % (10) | 21 % (10) | 35 % (6) | 21 % (10) | — |
| Commune d'un autre type | 2,0 % | 1,7 % | 2,1 % | 2,0 % | 1,6 % | 1,9 % | — |
| toute Commune (33) | | | | | | | 18,2 % |
| Peu commune de ses types | 20 % (9) | 18 % (10) | 22,5 % (8) | 20 % (9) | 18 % (10) | 20 % (9) | — |
| Peu commune neutre | 6,6 % (16) | 6,6 % (16) | 6,6 % (16) | 6,6 % (16) | 9,5 % (11) | 6,6 % (16) | — |
| Peu commune d'un autre type | 0,8 % | 0,9 % | 0,8 % | 0,8 % | 0,7 % | 0,8 % | — |
| toute Peu commune (43) | | | | | | | 7,0 % |
| chaque Rare (9) | 4,4 % | 4,4 % | 4,4 % | 4,4 % | 4,4 % | 4,4 % | 4,4 % |
| chaque Rare Holo | 15 % (4) | 15 % (4) | 20 % (3) | 20 % (3) | 20 % (3) | 15 % (4) | 5,0 % (12) |

(n) est le nombre de cartes de la ligne. Une Holo vaut exactement ⅗ ÷ n et une Rare ⅖ ÷ 9 ; le reste
est mesuré.

### 5. Boosters pour compléter une série

Voici le nombre de boosters pour un exemplaire de chaque carte qu'un booster vise : ses types et son
groupe neutre (pour Sauvage, toute la série de 97 cartes de booster). Les Rares à 4,4 % font la longue
traîne : depuis la 1.3, une série à thème demande environ 12 boosters de plus (les Rares sont plus
rares), et toute la série par Sauvage environ 24 de moins (les Holo sont plus fréquentes).

| Booster | Cartes | Moyenne | Médiane | 9 fois sur 10 en |
|---|---|---|---|---|
| Braise, Marée, Bosquet, Orage, Songe | 55–56 | 64 | 62 | 87 |
| Roc | 47 | 57 | 56 | 79 |
| Sauvage (toute la série) | 97 | 72 | 69 | 91 |

À environ 5 boosters par heure, un thème prend une douzaine d'heures avec les seuls boosters. Fabriquer
les dernières Communes et Peu communes (⬡ 80 / 240 pour une carte jamais possédée) raccourcit nettement
ce temps, et les cartes à l'unité et la revente aident aussi. C'est le « jeu, boosters et fabrication
ensemble » de `docs/WORLD_DESIGN.md` §3.4.

### 6. Effet sur la campagne

Les boosters ont rendu la campagne plus facile. Sur 16 graines de boosters et sans plafond de duels,
la partie automatique de `meta/src/playtest.rs` (`campaign_sweep`) demandait une médiane de 52
duels (40–105) avec ces boosters, contre 65 (44–100) avec ceux de la 1.x : son constructeur de deck
glouton reçoit plus de cartes des types qu'il achète. Les règles de la 1.3 (`docs/BALANCE.md`)
rendent les rivaux plus durs à battre et ramènent la médiane à 62 (38–83). Le taux de Holo de la 1.3
aide : avec l'ancien, un booster sur trois, les mêmes règles demandent une médiane de 66. Avec les 39
duellistes de route de Marée et Songe, la campagne demande une médiane de 66 duels (55–83), contre 55
sans les règles de la 1.3. Comme une seule partie
tirée au sort varie autant, le test joue trois graines et exige que deux finissent dans le budget.
`economy_sim` ne change pas, car prix et revenus sont intacts.

Les decks à débloquer (1.9, `docs/BALANCE.md`) sont prêtés, pas donnés : débloquer un deck n'ajoute
aucune carte à la collection, les boosters restent donc la seule source de cartes et aucun des chiffres
ci-dessus ne bouge.

### 7. Modifier

`COMMONS`, `UNCOMMONS`, `HOLO_ODDS` et `SHARES` sont des constantes publiques. Le panneau des
probabilités de la boutique et l'écran des boosters les lisent, l'interface suit donc. Après un
changement, lancez `booster_report` et mettez à jour les §4–5, puis lancez la partie de campagne
automatique et `economy_sim`.
