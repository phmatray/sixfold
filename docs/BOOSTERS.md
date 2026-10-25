# Boosters / Boosters

How a pack is built, what each booster favours, and the odds that follow. The code is
`engine/src/booster.rs`. The tests there check this page: `slot_rates_match_the_design` opens 12,000
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
  only 44 % of its Commons were Fire or Fighting, and 70 % of its Rares were trainers. Now the share of
  each slot is chosen on purpose.
- **A chase card.** Holo Rares (the Awaken 2 Hexons) are the cards a deck is built around. They turn up
  one pack in three, with their own foil.
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
| 10 | Rare slot | **Holo Rare 1 pack in 3** (33.3 %), otherwise a Rare (66.7 %) |

- **Holo Rare**: the 12 Awaken 2 Hexons. A themed pack only holds its own types' Holos; Wild holds
  all 12.
- **Rare**: the 8 Rare trainers and the special energy (9 cards, the same in every pack).
- **Duplicate protection**: the Rare slot skips any card you already hold 4 of (a full playset) while
  another card of that slot is left. Rares can't be crafted until you've pulled one, so this slot must
  keep paying. Commons and Uncommons sell back and can be crafted, so they have no protection.
- **No pity timer.** With a Holo one pack in 3, the chance of none in 6 packs in a row is 8.8 %, and in
  10 packs 1.7 %. That is short enough not to need a counter in the save.

### 3. Shares in a themed pack

Every Common and Uncommon slot first picks a group:

| Group | Share | What it holds |
|---|---|---|
| Own types | **60 %** | Hexons of the pack's two types |
| Neutral | **35 %** | trainers, the special energy and Colorless Hexons |
| Other types | **5 %** | Hexons of the four other types: a stray card, so a themed pack never fully locks you out |

60 / 35 matches a deck: about 20 Hexons for 12–14 trainers. Stone's own types are Fighting **and
Colorless**, so its neutral group is trainers only.

| Booster | Own types | Holo Rares |
|---|---|---|
| Ember | Fire, Fighting | Pyrequill, Solarlynx, Knollossus, Cragmastiff |
| Tide | Water, Psychic | Tidemaw, Gulfwyrm, Wraithelle, Hexarch |
| Grove | Grass, Lightning | Elderhart, Bogcrown, Fulgurex |
| Storm | Lightning, Water | Tidemaw, Gulfwyrm, Fulgurex |
| Stone | Fighting, Colorless | Knollossus, Cragmastiff, Warrenking |
| Dream | Psychic, Grass | Wraithelle, Hexarch, Elderhart, Bogcrown |
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
| each Rare (9) | 7.4 % | 7.4 % | 7.4 % | 7.4 % | 7.4 % | 7.4 % | 7.4 % |
| each Holo Rare | 8.3 % (4) | 8.3 % (4) | 11.1 % (3) | 11.1 % (3) | 11.1 % (3) | 8.3 % (4) | 2.8 % (12) |

(n) is how many cards share that line. A Holo's odds are exactly ⅓ ÷ n and a Rare's ⅔ ÷ 9; the rest are
measured.

### 5. Packs to complete a set

These are the packs needed for one copy of every card a booster is meant for: its own types and its
neutral group (for Wild, the whole set of 97 pack cards). The Holos and the 7.4 % Rares are what make
the tail long.

| Booster | Cards | Mean | Median | 9 runs in 10 by |
|---|---|---|---|---|
| Ember, Tide, Grove, Storm, Dream | 55–56 | 52 | 47–48 | 78 |
| Stone | 47 | 38 | 38 | 50 |
| Wild (whole set) | 97 | 96 | 96 | 123 |

At about 5 packs an hour, one theme takes roughly 10 hours from packs alone. Crafting the last Commons
and Uncommons (⬡ 80 / 240 for one you've never owned) cuts that well short, and singles and selling help
too. That is the "play, packs and crafting together" of `docs/WORLD_DESIGN.md` §3.4.

### 6. Effect on the campaign

The campaign gets easier, not harder. Over 16 pack seeds with no duel budget, the automatic
playthrough of `engine/src/playtest.rs` takes a median of 52 duels (40–91) with these packs, against 65
(44–100) with the 1.x packs. Its greedy deck builder gets more cards of the types it buys. Because one
seeded run swings this much, the test now plays three seeds and requires two to finish within budget
(`docs/BALANCE.md`). `economy_sim` doesn't change, since prices and income are untouched.

### 7. Changing it

`COMMONS`, `UNCOMMONS`, `HOLO_IN` and `SHARES` are public constants. The shop's odds panel and the pack
screen read them, so the UI follows. After a change, run `booster_report` and update §4–5, then run the
campaign playthrough and `economy_sim`.

---

## Français

### 1. Objectifs

- **Aucun emplacement inutile.** L'énergie de base est illimitée et jamais stockée : l'ancien
  emplacement énergie ne rapportait rien. Il devient une sixième Commune.
- **Un booster à thème parle de ses thèmes.** Dans les boosters 1.x, Braise tirait uniformément dans
  tout ce qu'il contenait : 44 % seulement de ses Communes étaient Feu ou Combat, et 70 % de ses Rares
  des Dresseurs. La part de chaque emplacement est désormais choisie.
- **Une carte à chasser.** Les Rares Holo (les Hexons d'Éveil 2) sont les cartes autour desquelles on
  construit un deck. Elles sortent un booster sur trois, avec leur propre effet holographique.
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
| 10 | emplacement Rare | **Rare Holo 1 booster sur 3** (33,3 %), sinon une Rare (66,7 %) |

- **Rare Holo** : les 12 Hexons d'Éveil 2. Un booster à thème ne contient que les Holo de ses types ;
  Sauvage contient les 12.
- **Rare** : les 8 Dresseurs Rares et l'énergie spéciale (9 cartes, les mêmes dans tous les boosters).
- **Protection anti-doublons** : l'emplacement Rare saute toute carte que vous avez déjà en 4
  exemplaires (un jeu complet), tant qu'il reste une autre carte possible. Une Rare ne se fabrique
  qu'après en avoir tiré une : cet emplacement doit donc toujours rapporter. Les Communes et Peu
  communes se revendent et se fabriquent, elles n'ont pas de protection.
- **Pas de compteur de malchance.** Avec une Holo un booster sur 3, la probabilité de n'en voir aucune
  sur 6 boosters d'affilée est de 8,8 %, et sur 10 boosters de 1,7 %. C'est assez court pour se passer
  d'un compteur dans la sauvegarde.

### 3. Les parts d'un booster à thème

Chaque emplacement Commune et Peu commune choisit d'abord un groupe :

| Groupe | Part | Contenu |
|---|---|---|
| Types du booster | **60 %** | Hexons des deux types du booster |
| Neutre | **35 %** | Dresseurs, énergie spéciale et Hexons Incolore |
| Autres types | **5 %** | Hexons des quatre autres types : une carte égarée, pour qu'un booster à thème ne ferme jamais tout à fait la porte |

60 / 35 correspond à un deck : environ 20 Hexons pour 12 à 14 Dresseurs. Les types de Pierre sont
Combat **et Incolore**, son groupe neutre ne contient donc que des Dresseurs.

| Booster | Types | Rares Holo |
|---|---|---|
| Braise | Feu, Combat | Pyrequill, Solarlynx, Knollossus, Cragmastiff |
| Marée | Eau, Psychique | Tidemaw, Gulfwyrm, Wraithelle, Hexarch |
| Bosquet | Herbe, Foudre | Elderhart, Bogcrown, Fulgurex |
| Orage | Foudre, Eau | Tidemaw, Gulfwyrm, Fulgurex |
| Pierre | Combat, Incolore | Knollossus, Cragmastiff, Warrenking |
| Songe | Psychique, Herbe | Wraithelle, Hexarch, Elderhart, Bogcrown |
| **Sauvage** | tous les types, **à parts égales** | les 12 |

**Sauvage** n'a pas de parts : chaque emplacement tire à parts égales parmi toutes les cartes de sa
rareté. C'est le booster pour combler les trous de toute la série.

### 4. Probabilités par carte

La probabilité qu'une carte donnée soit dans un booster, avec une collection vide :

| Carte | Braise | Marée | Bosquet | Orage | Pierre | Songe | Sauvage |
|---|---|---|---|---|---|---|---|
| Commune de ses types | 45 % (8) | 60 % (6) | 40 % (9) | 45 % (8) | 45 % (8) | 51 % (7) | — |
| Commune neutre | 21 % (10) | 21 % (10) | 21 % (10) | 21 % (10) | 35 % (6) | 21 % (10) | — |
| Commune d'un autre type | 2,0 % | 1,7 % | 2,1 % | 2,0 % | 1,6 % | 1,9 % | — |
| toute Commune (33) | | | | | | | 18,2 % |
| Peu commune de ses types | 20 % (9) | 18 % (10) | 22,5 % (8) | 20 % (9) | 18 % (10) | 20 % (9) | — |
| Peu commune neutre | 6,6 % (16) | 6,6 % (16) | 6,6 % (16) | 6,6 % (16) | 9,5 % (11) | 6,6 % (16) | — |
| Peu commune d'un autre type | 0,8 % | 0,9 % | 0,8 % | 0,8 % | 0,7 % | 0,8 % | — |
| toute Peu commune (43) | | | | | | | 7,0 % |
| chaque Rare (9) | 7,4 % | 7,4 % | 7,4 % | 7,4 % | 7,4 % | 7,4 % | 7,4 % |
| chaque Rare Holo | 8,3 % (4) | 8,3 % (4) | 11,1 % (3) | 11,1 % (3) | 11,1 % (3) | 8,3 % (4) | 2,8 % (12) |

(n) est le nombre de cartes de la ligne. Une Holo vaut exactement ⅓ ÷ n et une Rare ⅔ ÷ 9 ; le reste
est mesuré.

### 5. Boosters pour compléter une série

Voici le nombre de boosters pour un exemplaire de chaque carte qu'un booster vise : ses types et son
groupe neutre (pour Sauvage, toute la série de 97 cartes de booster). Les Holo et les Rares à 7,4 %
font la longue traîne.

| Booster | Cartes | Moyenne | Médiane | 9 fois sur 10 en |
|---|---|---|---|---|
| Braise, Marée, Bosquet, Orage, Songe | 55–56 | 52 | 47–48 | 78 |
| Pierre | 47 | 38 | 38 | 50 |
| Sauvage (toute la série) | 97 | 96 | 96 | 123 |

À environ 5 boosters par heure, un thème prend une dizaine d'heures avec les seuls boosters. Fabriquer
les dernières Communes et Peu communes (⬡ 80 / 240 pour une carte jamais possédée) raccourcit nettement
ce temps, et les cartes à l'unité et la revente aident aussi. C'est le « jeu, boosters et fabrication
ensemble » de `docs/WORLD_DESIGN.md` §3.4.

### 6. Effet sur la campagne

La campagne devient plus facile, pas plus dure. Sur 16 graines de boosters et sans plafond de duels,
la partie automatique de `engine/src/playtest.rs` demande une médiane de 52 duels (40–91) avec ces
boosters, contre 65 (44–100) avec ceux de la 1.x. Son constructeur de deck glouton reçoit plus de
cartes des types qu'il achète. Comme une seule partie tirée au sort varie autant, le test joue
désormais trois graines et exige que deux finissent dans le budget (`docs/BALANCE.md`). `economy_sim`
ne change pas, car prix et revenus sont intacts.

### 7. Modifier

`COMMONS`, `UNCOMMONS`, `HOLO_IN` et `SHARES` sont des constantes publiques. Le panneau des
probabilités de la boutique et l'écran des boosters les lisent, l'interface suit donc. Après un
changement, lancez `booster_report` et mettez à jour les §4–5, puis lancez la partie de campagne
automatique et `economy_sim`.
