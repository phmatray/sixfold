# Sixfold — Bestiary / Bestiaire

The creatures of Sixfold are **Hexons** (*un Hexon, des Hexons*, the same word in English and French).
This file holds the world lore, the design language of each type and the rules for illustrating a
Hexon. The species sheets themselves are in [`bestiary/`](bestiary/), one file per type, in evolution
order. The short **dex line** of every sheet ships with the game in `engine/src/lore.rs`.

*Les créatures de Sixfold sont les **Hexons**. Ce fichier rassemble le contexte du monde, le langage
visuel de chaque type et les règles d'illustration. Les fiches d'espèce sont dans
[`bestiary/`](bestiary/), un fichier par type, dans l'ordre des lignées. La courte **ligne de
registre** de chaque fiche est livrée avec le jeu dans `engine/src/lore.rs`.*

---

## 1. What Hexons are / Ce que sont les Hexons

**EN.** The old wardens say the world was cut like a seal stone, with six faces: **Fire, Water, Grass, Lightning, Fighting and Psychic**. Everything alive carries a little of one face, but in a Hexon the face has
taken root. Its **grain** (kiln clay, sea glass, bark, copper, rock strata or lantern gauze) grows over
the shape of an ordinary beast the way frost grows over a twig, until the animal underneath is only a
memory in the silhouette. That is why a Hexon reminds you of a bird, a frog or a hare without being one.

The seventh face, smooth and blank on every seal stone, belongs to the **Colourless** Hexons. They carry
no grain at all, only the plain stuff of the meadow and the road (felt sewn in patchwork, thread and
yarn, chalk, unglazed porcelain), and they live wherever the six roads meet. Having no face of their own,
each line grew the shape of a tool travellers carry: a bellows, a hearing horn, a pincushion, a spool. Seal stone is grain without a body: the Facets in every purse are
chips of it.

**FR.** Les anciens gardiens disent que le monde a été taillé comme une pierre de sceau, à six faces : **Feu, Eau, Herbe, Foudre, Combat et Psychique**. Tout ce qui vit porte un peu d'une face ; chez un
Hexon, la face a pris racine. Son **grain** (argile de four, verre de mer, écorce, cuivre, strates de
roche ou gaze de lanterne) recouvre la forme d'une bête ordinaire comme le givre recouvre une
brindille, jusqu'à ce que l'animal d'origine ne survive plus que dans la silhouette. Voilà pourquoi un
Hexon rappelle un oiseau, une grenouille ou un lièvre sans en être un.

La septième face, lisse et vierge sur chaque pierre de sceau, appartient aux Hexons **Incolores**. Ils
ne portent aucun grain, seulement la matière simple du pré et de la route (feutre cousu en patchwork,
fil et laine, craie, porcelaine brute), et vivent là où les six routes se croisent. Faute d'une face à
eux, chaque lignée a pris la forme d'un outil de voyageur : un soufflet, un cornet acoustique, une
pelote à épingles, une bobine. La pierre de sceau, c'est du grain sans corps : les
Facettes de chaque bourse en sont des éclats.

### Awakening / L'Éveil

**EN.** A Hexon does not grow up, it **awakens**. When it has drunk enough of its face (a kiln's heat,
a storm's charge, a spring tide, a long dream), its grain spreads through its body in a single night.
A **Basic** becomes **Awaken 1**, and the lines that go further reach **Awaken 2**. Each Awakening keeps
the colours, markings and signature parts of the stage before and enlarges them, so a duelist can
always tell which line a Hexon belongs to.

**FR.** Un Hexon ne grandit pas, il **s'éveille**. Quand il a bu assez de sa face (la chaleur d'un four,
la charge d'un orage, une grande marée, un long rêve), son grain gagne tout son corps en une seule
nuit. Un Hexon de **Base** devient **Éveil 1**, et les lignées qui vont plus loin atteignent l'**Éveil 2**.
Chaque Éveil garde les couleurs, les marques et les parties distinctives du stade précédent en les
amplifiant : un duelliste reconnaît toujours la lignée d'un Hexon.

### The Field Register / Le Registre de terrain

**EN.** Sable, the **Hex Warden**, keeps the Field Register in the duel hall on **Sixfold Meadow**, where the six roads start. Every warden, ranger and duelist who walks the ring of regions (the Emberlands, the Galewold, the Saltreach, the Hushlands, the Deepgrove and the Stonebacks) sends back what they saw. The register's one-line entries are the dex lines
printed in the game.

**FR.** Sable, la **Gardienne des Six**, tient le Registre de terrain dans la salle de duel du **Pré de Sixfold**, là où partent les six routes. Chaque gardien, garde forestier ou duelliste qui parcourt l'anneau des régions (les Braisières, les Hautes-Rafales, la Côte Salée, les Terres du Murmure, le Bois-Profond et les Échines de Pierre) y consigne ce qu'il a vu. Les entrées d'une
ligne du registre sont les lignes de registre imprimées dans le jeu.

---

## 2. Design language per type / Langage visuel par type

Every Hexon is built from three things: the **animal base** (kept only in the silhouette and one or two
traits), the **grain** of its type (the materials below) and **one invented organ** of its own line that
no real animal has (an ember heart behind glaze, a tide pool on the back, a crown of rods…). There is
**no shared logo mark**: species are told apart by anatomy, materials and silhouette.

*Chaque Hexon se construit à partir de trois éléments : la **base animale** (conservée seulement dans la
silhouette et un ou deux traits), le **grain** de son type (les matières ci-dessous) et **un organe
inventé** propre à sa lignée, qu'aucun animal réel ne possède. Il n'y a **pas de marque commune** :
on distingue les espèces par l'anatomie, les matières et la silhouette.*

| Type | Region / Région | Grain | Signature materials | Recurring trait | Palette |
|---|---|---|---|---|---|
| Fire / Feu | The Emberlands / Les Braisières: Emberstep Quarry, Cinderwick, the Kilnmouth | kiln clay / argile de four | kiln-glazed ceramic plates, crackle glaze, fired-clay quills and horns with ember wicks, glass-bead eyes | **glowing seams**: hairline glaze cracks lit from an ember core; each Awakening widens them into vents | terracotta, amber, soot charcoal, cream ash, molten gold |
| Water / Eau | The Saltreach / La Côte Salée: Saltwind Boardwalk, Gullhaven, the Bellflats | sea glass / verre de mer | translucent glassy fins and frills, nacre plates, bubble-lens eyes, kelp ribbons | **tide rings**: pale concentric ripple bands on the hide, more rings with each Awakening | teal, sea-glass green, deep indigo, pearl, foam cream |
| Grass / Herbe | The Deepgrove / Le Bois-Profond: Fernhollow Trail, Rootholm, the Elder Ring | bark / écorce | **structural** bark: the body itself is plated bark with growth-ring grain, legs are roots, the chest is a seed pod, leaves and shoots grow out of the body (never laid on top) | **buds that open**: closed buds on a Basic, leaves at Awaken 1, blossom at Awaken 2 | moss green, bark brown, fresh lime, pollen gold, petal cream |
| Lightning / Foudre | The Galewold / Les Hautes-Rafales: Thrumming Downs, Stormwatch, the Rodspire | copper and mica / cuivre et mica | copper-coil horns and prongs, glassy mica insulator beads, filament whiskers, slate storm-felt fur | **paired prongs**: every Storm Hexon carries copper prongs in pairs with a spark jumping between them | storm slate, electric yellow, copper, white-blue |
| Fighting / Combat | The Stonebacks / Les Échines de Pierre: the Undermine, Scree Pass, Anvilstead, the Bastion | rock strata / strates de roche | sandstone and slate strata plates, quartz knuckles and claws, iron-banded joints, gravel manes, moss in the cracks | **strata**: layered bands of rock like a cliff face, more layers with each Awakening | ochre sandstone, slate grey, rust, quartz white, umber |
| Psychic / Psychique | The Hushlands / Les Terres du Murmure: Hushwillow Lane, Lanternmere, the Moonwell Garden | lantern gauze / gaze de lanterne | translucent paper-lantern membranes lit from inside, veils and gauze trails, mother-of-pearl masks, moth dust | **orbiting motes**: small glowing motes that circle the body, one more with each Awakening | violet, dusk indigo, moon silver, lantern amber, lilac |
| Colorless / Incolore | the blank face / la face vierge: Sixfold Meadow, Brightwell Square, every road | none / aucun | **structural** felt: no fur, feathers or skin, the body itself is sewn from patchwork panels of thick matted felt joined by bold stitched seams; its working parts are travellers' tools grown as organs (bellows flanks, hearing-horn ears, spool joints, a pincushion back, a stocking tail, a yarn-ball body, raised moth wings); hard parts are chalk horn and unglazed porcelain / *feutre **structurel** : ni fourrure, ni plumes, ni peau, le corps est cousu de pièces de feutre épais en patchwork aux coutures bien visibles ; ses organes sont des outils de voyageur (flancs-soufflets, oreilles-cornets, articulations-bobines, dos-pelote à épingles, queue-bas, corps-pelote, ailes de phalène dressées) ; les parties dures sont de corne de craie et de porcelaine brute* | **living chalk**: chalk markings that differ per line and mean something (a tunnel map, wind arrows, sound rings, store tallies, load tallies, a route, stitch marks), plus **one or two impossible organs** per line that change the silhouette; no mark is shared across species / *des marques de craie vivantes, propres à chaque lignée et qui disent quelque chose, et un ou deux organes impossibles par lignée qui changent la silhouette ; aucune marque commune* | oatmeal, cream, warm grey, slate, chalk white |

**What each type avoids / Ce que chaque type évite.** Fire: crimson or dripping red (reads as blood),
a body made only of flames, realistic fur with a flame added. Water: a real frog, fish or seal with a
splash added. Grass: a real deer, turtle or dog wearing leaves; the animal is only a silhouette hint under structural bark, root and pod anatomy. Lightning: a real weasel or bee with a
glow. Fighting: a real mole or dog painted grey. Psychic: a real glider or owl tinted violet. Colorless: a real rabbit, hare, mouse, hamster, bird, bison or ram with felt texture or chalk lines added; fur, feathers or skin; a plush or toy look; an upright humanoid stance; a shared chest disc, oval or any emblem repeated across species. The animal is only a silhouette hint under sewn patchwork and the line's tool-organ: a Colorless Hexon must not read as a real animal. *Incolore : un vrai lapin, lièvre, souris, hamster, oiseau, bison ou bélier avec du feutre ou de la craie par-dessus ; fourrure, plumes ou peau ; un air de peluche ; une posture humaine ; un disque, un ovale ou un emblème commun à plusieurs espèces. L'animal n'est qu'un souvenir de silhouette sous le patchwork cousu et l'organe-outil de la lignée.* *Pour tous : aucun animal réel avec un effet ajouté par-dessus.*

---

## 3. Illustration rules / Règles d'illustration

**Style.** Painted creature-design illustration in gouache and watercolour on cold-press paper, with
ink linework, matching the trainer and energy art: an invented species from a fantasy bestiary, never a
wildlife study. One Hexon per picture, a soft wash in the type's palette behind it, no text, no frame,
no card border. Commons and uncommons stand in a three-quarter view in warm side light; **Awaken 2,
rares and promos** get a low angle, backlight and a more dramatic pose.

*Style : illustration peinte de création de créature, gouache et aquarelle sur papier à grain, trait
d'encre, assortie aux dresseurs et aux énergies ; une espèce inventée d'un bestiaire fantastique, jamais
une étude animalière. Un seul Hexon par image, un lavis dans la palette du type, ni texte, ni cadre, ni
bord de carte. Les Éveils 2, rares et promos ont une contre-plongée, un contre-jour et une pose plus
dramatique.*

**Safe area / Zone sûre.** The picture is 800×560 (10:7). Full cards (140px and wider) show all of
it; compact cards (hand, table) show about the central 70% of the width; picture-only cards (the
Bench on a phone, the top of the discard) show the central 50% (see `tools/art/README.md`). So the
**whole Hexon**, ears, tails, crests and veils included, sits inside the **central 50% of the width
and 90% of the height** (x 200–600, y 28–532). Silhouettes are compact: seated, perched, coiled or
front three-quarter poses, tails curled up, wings folded or raised. Side-on quadrupeds, crouches and
spread wings or veils spread too wide. Only the painted wash reaches the sides.

*L'image fait 800×560. Les cartes complètes la montrent entière, les cartes compactes environ 70 %
de la largeur au centre, les images seules (banc sur téléphone, dessus de la défausse) les 50 %
centraux. Le **Hexon entier**, oreilles, queues, crêtes et voiles compris, tient donc dans les
**50 % centraux de la largeur et 90 % de la hauteur**. Poses compactes : assis, perché, enroulé ou
de trois quarts face ; seul le lavis touche les bords.*

**Never / Jamais.** No hexagon shapes or markings on any Hexon, and no mark shared across species (no chest disc, oval or emblem). No upright human stance or human silhouette (a Hexon sits, crouches, floats or stands like an animal). No real animal with an effect painted
on top. No franchise likeness. No red liquids. Exact counts for legs, wings and horns in every caption.
*Aucun hexagone ni marque commune à plusieurs espèces (disque, ovale, emblème), aucune silhouette ou posture humaine, aucun animal réel retouché, aucune ressemblance avec une franchise, aucun liquide
rouge.*

---

## 4. Species sheets / Fiches d'espèce

Each sheet has fifteen sections, every one filled in English and French (a sheet with fewer than ten
filled sections is incomplete):

1. Identity / Identité · 2. Animal base and inspiration / Base animale · 3. Morphology / Morphologie ·
4. Size and weight / Taille et poids · 5. Colours and materials / Couleurs et matières · 6. Habitat ·
7. Behaviour / Comportement · 8. Diet and preferences / Régime et goûts · 9. Life cycle and Awakening /
Cycle de vie et Éveil · 10. Abilities / Capacités · 11. People and duelists / Relations avec les
humains · 12. Rarity and sightings / Rareté et observation · 13. Folklore · 14. Visual key / Clé
visuelle · 15. Dex line / Ligne de registre

The FLUX caption of each card (`tools/art/prompts.json`) is written from sections 3, 5, 6 and 14. Headings give the English identity name, then the French card name; the French text uses the French card, attack and Power names. *Les titres donnent le nom anglais d'identité puis le nom français de la carte ; le texte français emploie les noms français des cartes, attaques et pouvoirs.*

| File | Lines (dex numbers) | Sheets |
|---|---|---|
| [fire.md](bestiary/fire.md) | Cindrel line 001–003 · Ashpaw line 004–006 · Kilnback line 007–008 · Waxwick 009 · Cinderhorn 010 · Kilnfox 011 | complete |
| [water.md](bestiary/water.md) | Puddock line 012–014 · Rillet line 015–017 · Kelpup line 018–019 · Shellmote 020 · Frosteel 021 · Brinemoth 022 | complete |
| [grass.md](bestiary/grass.md) | Sprigget line 023–025 · Mirebud line 026–028 · Pollenpup line 029–030 · Briarhog 031 · Fernwhisk 032 · Thistlewick 033 | complete |
| [lightning.md](bestiary/lightning.md) | Sparkit line 034–036 · Buzzlet line 037–038 · Ohmlet line 039–040 · Stormox 041 · Zippet 042 · Flashfin 043 · Squallkite 044 | complete |
| [fighting.md](bestiary/fighting.md) | Knollop line 045–047 · Shalepup line 048–050 · Brawlet line 051–052 · Tusklet 053 · Gritfang 054 · Bouldergut 055 · Slatebrow 056 | complete |
| [psychic.md](bestiary/psychic.md) | Wispun line 057–059 · Hexlet line 060–062 · Dozel line 063–064 · Glimmox 065 · Gloamowl 066 · Moonmoth 067 | complete |
| [colorless.md](bestiary/colorless.md) | Burrowbit line 068–070 · Pecklet line 071–072 · Scampit 073 · Echopup 074 · Haulox 075 · Pocketmouse 076 · Mendlark 077 · Hexcrest 078 | complete |
