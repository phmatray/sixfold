//! Dex lines: the one-line field-guide entry of every Hexon, in English and
//! French, keyed by the card's English name. The full species sheets they
//! come from are in `docs/bestiary/` (see `docs/BESTIARY.md`).

/// A Hexon's dex line in both languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lore {
    pub en: &'static str,
    pub fr: &'static str,
}

/// `(English card name, lore)`, in dex order.
pub static LORE: &[(&str, Lore)] = &[
    // 001
    (
        "Cindrel",
        Lore {
            en: "Its clay egg hatches only when a kiln is lit. Glassworkers read the heat of their fires from the colour of its crest.",
            fr: "Son œuf d'argile n'éclot que lorsqu'on allume un four. Les verriers lisent la chaleur de leurs feux à la couleur de sa crête.",
        },
    ),
    // 002
    (
        "Flarrow",
        Lore {
            en: "It never walks when it can sprint. A Flarrow racing downhill is the quarry's warning that a kiln is running too hot.",
            fr: "Il ne marche jamais quand il peut courir. Un Flambèche qui dévale la pente avertit la carrière qu'un four chauffe trop.",
        },
    ),
    // 003
    (
        "Pyrequill",
        Lore {
            en: "Its ember heart burns in an open firebox. It relights any kiln that dies near its roost and warms young Cindrels under its wings.",
            fr: "Son cœur de braise brûle dans un foyer ouvert. Il rallume tout four éteint près de son perchoir et réchauffe les jeunes Braisillon sous ses ailes.",
        },
    ),
    // 004
    (
        "Ashpaw",
        Lore {
            en: "It sleeps with its clay paws buried in hot ash. Quarry workers warm their hands on them on cold mornings.",
            fr: "Il dort les pattes d'argile enfouies dans la cendre chaude. Les carriers s'y réchauffent les mains les matins froids.",
        },
    ),
    // 005
    (
        "Blazelynx",
        Lore {
            en: "It waits on a ledge with its ruff folded, then flares all twelve blades at once and drops on its prey in a wave of heat.",
            fr: "Il attend sur une corniche, collerette repliée, puis dresse ses douze lames d'un coup et fond sur sa proie dans une vague de chaleur.",
        },
    ),
    // 006
    (
        "Solarlynx",
        Lore {
            en: "It greets the sunrise with its ruff wide open. After a Supernova Leap it lies grey for days, and the ash where it rests keeps glowing.",
            fr: "Il salue le lever du jour collerette grande ouverte. Après un Saut supernova, il reste gris des jours\u{202f}; la cendre où il dort luit encore.",
        },
    ),
    // 007
    (
        "Kilnback",
        Lore {
            en: "It digs coal and swallows it to feed the kiln on its back. On cold nights, the quarry's young Hexons sleep pressed against its dome.",
            fr: "Il creuse le charbon et l'avale pour nourrir le four de son dos. Les nuits froides, les jeunes Hexons de la carrière dorment contre son dôme.",
        },
    ),
    // 008
    (
        "Furnaceback",
        Lore {
            en: "It feeds its own firedoor one coal at a time and can strip a seam in a night. Kiln stokers swear it stokes better than they do.",
            fr: "Il nourrit sa porte de foyer un charbon à la fois et vide une veine en une nuit. Les chauffeurs de four jurent qu'il attise mieux qu'eux.",
        },
    ),
    // 009
    (
        "Waxwick",
        Lore {
            en: "Its crest is a wick in a cup of its own wax. One lit Waxwick lights its whole flock, wick to wick, until the ash pine glitters.",
            fr: "Sa huppe est une mèche plantée dans sa propre cire. Un seul Cirèche allumé embrase toute sa volée, mèche à mèche, et le pin scintille.",
        },
    ),
    // 010
    (
        "Cinderhorn",
        Lore {
            en: "Its horns are hollow clay flues its ember core breathes through. Rivals clash on narrow ledges until one pair of horns overheats.",
            fr: "Ses cornes sont des conduits d'argile où respire son cœur de braise. Les rivaux s'affrontent sur les corniches jusqu'à ce qu'une paire surchauffe.",
        },
    ),
    // 011
    (
        "Kilnfox",
        Lore {
            en: "It keeps a live coal in the glass bulb of its tail and steals from glassworks furnaces to feed it. It blows into its tail when pleased.",
            fr: "Il garde un charbon vivant dans le bulbe de verre de sa queue et chipe aux fours de la Verrerie pour le nourrir. Content, il souffle dedans.",
        },
    ),
    // 012
    (
        "Puddock",
        Lore {
            en: "It never loses its tadpole tail. When its throat sac turns cloudy, Gullhaven fishers stay ashore.",
            fr: "Il ne perd jamais sa queue de têtard. Quand son sac vocal se trouble, les pêcheurs de Havre-aux-Mouettes restent à terre.",
        },
    ),
    // 013
    (
        "Brookhorn",
        Lore {
            en: "It guards a single pool by spinning up a whirlpool. A shed Brookhorn horn makes a whistle that sailors say calls the current home.",
            fr: "Il garde une seule mare en levant un tourbillon. Une corne de Ruissecorne fait un sifflet qui, disent les marins, ramène le courant.",
        },
    ),
    // 014
    (
        "Tidemaw",
        Lore {
            en: "It carries a living tide pool on its back and a whole wave in its mouth. It spends the wave once, then waits for the next spring tide.",
            fr: "Il porte une mare vivante sur le dos et une vague entière dans la gueule. Il ne la lâche qu'une fois, puis attend la grande marée suivante.",
        },
    ),
    // 015
    (
        "Rillet",
        Lore {
            en: "A little stream runs through its glass body all day long. Bellflats clam diggers check which way a Rillet points to know when the tide will turn.",
            fr: "Un petit ruisseau traverse son corps de verre toute la journée. Les pêcheurs à pied regardent où pointe un Ruisselin pour savoir quand tournera la marée.",
        },
    ),
    // 016
    (
        "Streamfin",
        Lore {
            en: "It holds still in the racing tide with its blades laid flat, then cuts across the current in one stroke. It rides the Gullhaven ferry's wake for fun.",
            fr: "Immobile dans la marée qui court, lames couchées, il coupe le courant d'un seul trait. Il suit le sillage du bac de Havre-aux-Mouettes pour le plaisir.",
        },
    ),
    // 017
    (
        "Gulfwyrm",
        Lore {
            en: "Its inner current runs so deep and cold that frost forms on its own glass. The bell tower stones it coils around stay rimed all summer.",
            fr: "Son courant intérieur est si profond et si froid que le givre prend sur son propre verre. Les pierres du clocher qu'il enlace restent givrées tout l'été.",
        },
    ),
    // 018
    (
        "Kelpup",
        Lore {
            en: "Kelp grows from its own hide, held up by a glass float full of tide. It wraps its whiskers round anything that drifts by, just to see what it is.",
            fr: "Le varech pousse sur sa peau, porté par un flotteur de verre plein de marée. Il enlace de ses moustaches tout ce qui dérive, juste pour voir.",
        },
    ),
    // 019
    (
        "Kelpwarden",
        Lore {
            en: "It keeps a kelp bed the way a gardener keeps a hedge. Each glass float on its mantle holds a bead of tide, and it gives the oldest away.",
            fr: "Il entretient sa forêt d'algues comme un jardinier sa haie. Chaque flotteur de son manteau garde une perle de marée\u{202f}; il donne le plus vieux.",
        },
    ),
    // 020
    (
        "Shellmote",
        Lore {
            en: "It blows its own house from sea glass, light enough to float. Gullhaven net menders find its old globes in their nets and use them as floats.",
            fr: "Il souffle sa propre maison de verre de mer, assez légère pour flotter. Les ramendeurs de Havre-aux-Mouettes en font des flotteurs.",
        },
    ),
    // 021
    (
        "Frosteel",
        Lore {
            en: "Its crest pulls the cold out of the sea and grows it into ice needles. Clam diggers know its pool by the frost on the rim in high summer.",
            fr: "Sa crête tire le froid de la mer pour en faire des aiguilles de glace. On reconnaît sa mare au givre sur le bord, en plein été.",
        },
    ),
    // 022
    (
        "Brinemoth",
        Lore {
            en: "Each of its four glass wings holds a skin of living seawater. When it skims the flooded Bellflats at night, the tide follows it.",
            fr: "Chacune de ses quatre ailes de verre porte une peau d'eau de mer vivante. Quand il rase les Grèves inondées la nuit, la marée le suit.",
        },
    ),
    // 023
    (
        "Sprigget",
        Lore {
            en: "It sleeps with its root-hooves in the soil. Wherever a Sprigget has slept, the ground grows twice as well next year.",
            fr: "Il dort les sabots-racines plantés dans le sol. Là où un Brindelet a dormi, la terre donne deux fois plus l'année suivante.",
        },
    ),
    // 024
    (
        "Thornbuck",
        Lore {
            en: "Its antlers are living briar, regrown thornier every autumn. The barbs it sheds carry a stinging sap.",
            fr: "Ses bois sont de la ronce vivante, plus épineuse chaque automne. Les épines qu'il perd portent une sève qui pique.",
        },
    ),
    // 025
    (
        "Elderhart",
        Lore {
            en: "A small tree blossoms on its head. When the Deepgrove is hurt it walks, and a pulse of green light mends everything nearby.",
            fr: "Un petit arbre fleurit sur sa tête. Quand le Bois-Profond souffre, il se met en marche, et une onde de lumière verte soigne tout alentour.",
        },
    ),
    // 026
    (
        "Mirebud",
        Lore {
            en: "Its shell is a seed pod and it breathes through the bud on top. If the buds along the bank stop moving, the water is going bad.",
            fr: "Sa carapace est une cosse et il respire par le bourgeon du sommet. Si les bourgeons de la berge cessent de bouger, l'eau se gâte.",
        },
    ),
    // 027
    (
        "Bogblossom",
        Lore {
            en: "Rootholm calls its leaf funnel a blossom. Lean in to smell it at dusk and you'll still be standing there at dawn.",
            fr: "Racinemont appelle sa corolle de feuilles une fleur. Penchez-vous pour la sentir au crépuscule, et vous y serez encore à l'aube.",
        },
    ),
    // 028
    (
        "Bogcrown",
        Lore {
            en: "It breathes in the dark under Rootholm's fallen trunk. Seven blossoms ring its shell, and each one closes on whatever lands.",
            fr: "Il respire dans le noir sous le tronc couché de Racinemont. Sept fleurs couronnent sa carapace, et chacune se referme sur ce qui s'y pose.",
        },
    ),
    // 029
    (
        "Pollenpup",
        Lore {
            en: "Its chest is a seed pod that fills with nectar and glows through the husk. Hives give a third more honey when one sleeps nearby.",
            fr: "Sa poitrine est une cosse qui s'emplit de nectar et luit à travers l'enveloppe. Les ruches donnent un tiers de miel en plus s'il dort à côté.",
        },
    ),
    // 030
    (
        "Honeymane",
        Lore {
            en: "Its mane is beaded with nectar. It shakes a few drops onto anything hurt, and flails at anyone who comes for the hive.",
            fr: "Sa crinière est perlée de nectar. Il en secoue quelques gouttes sur tout ce qui souffre, et fouette qui s'en prend à la ruche.",
        },
    ),
    // 031
    (
        "Briarhog",
        Lore {
            en: "Every thorn sits in a socket of sap. Strike its burr and the thorns you hit fire straight back at you.",
            fr: "Chaque épine est plantée dans une loge de sève. Frappez sa bogue, et les épines touchées vous reviennent droit dessus.",
        },
    ),
    // 032
    (
        "Fernwhisk",
        Lore {
            en: "Its tail is a fern crozier coiled tighter than a fist. One flick unrolls it into a fan, and the gust carries it clear of any fight.",
            fr: "Sa queue est une crosse de fougère roulée plus serré qu'un poing. D'un coup, elle s'ouvre en éventail et la rafale l'emporte hors du combat.",
        },
    ),
    // 033
    (
        "Thistlewick",
        Lore {
            en: "Its wick is a thistle on a stalk that drinks the dew. The bud stays shut for life, brewing the violet sap on every thorn.",
            fr: "Sa mèche est un chardon sur une tige qui boit la rosée. Le bouton reste fermé toute sa vie et brasse la sève violette de chaque épine.",
        },
    ),
    // 034
    (
        "Sparkit",
        Lore {
            en: "A spark always jumps between its two copper prongs. When it grows, the observers of Stormwatch close their shutters.",
            fr: "Une étincelle saute toujours entre ses deux pointes de cuivre. Quand elle grandit, les observateurs de Guette-Orage ferment leurs volets.",
        },
    ),
    // 035
    (
        "Arcfang",
        Lore {
            en: "An arc crackles between its two copper fangs. One bite and the spark leaps from its prey to everything nearby.",
            fr: "Un arc crépite entre ses deux crocs de cuivre. Une morsure, et l'étincelle saute de sa proie à tout ce qui l'entoure.",
        },
    ),
    // 036
    (
        "Fulgurex",
        Lore {
            en: "It catches lightning on the copper rods along its spine, then roams the Downs releasing the charge bolt by bolt.",
            fr: "Il capte la foudre sur les paratonnerres de son échine, puis parcourt les Landes en relâchant la charge éclair par éclair.",
        },
    ),
    // 037
    (
        "Buzzlet",
        Lore {
            en: "A single spark drifts inside the mica bulb of its abdomen. On the Downs, the heather hums wherever Buzzlets feed.",
            fr: "Une seule étincelle flotte dans le bulbe de mica de son abdomen. Sur les Landes, la bruyère bourdonne là où butinent les Buzzlets.",
        },
    ),
    // 038
    (
        "Hivevolt",
        Lore {
            en: "Its mica abdomen holds a swarm of sparks. It lets them out three at a time, and every one flies home.",
            fr: "Son abdomen de mica abrite un essaim d'étincelles. Il les lâche trois par trois, et chacune rentre au nid.",
        },
    ),
    // 039
    (
        "Ohmlet",
        Lore {
            en: "A bead of light slides up the copper rails of its tail and pops off the tips. Its scorch marks freckle every wall on the Downs.",
            fr: "Une perle de lumière remonte les rails de cuivre de sa queue et jaillit des pointes. Ses brûlures piquettent tous les murets des Landes.",
        },
    ),
    // 040
    (
        "Ohmrunner",
        Lore {
            en: "To aim, it stands perfectly still. Then a bead of light races up its tail rails and strikes a foe far down the line.",
            fr: "Pour viser, il se fige. Puis une perle de lumière file le long de ses rails et frappe un adversaire loin derrière les lignes.",
        },
    ),
    // 041
    (
        "Stormox",
        Lore {
            en: "The mica sheets on its hump fill with storm light. When they are full, it must empty them all at once, and it feels every bit.",
            fr: "Les feuilles de mica de sa bosse se remplissent de lumière d'orage. Pleines, il doit les vider d'un coup, et il le sent passer.",
        },
    ),
    // 042
    (
        "Zippet",
        Lore {
            en: "Its hind legs are copper springs. Blink, and it has jumped three walls away in a crack of static.",
            fr: "Ses pattes arrière sont des ressorts de cuivre. Un clignement d'yeux, et il a bondi trois murets plus loin dans un claquement d'étincelle.",
        },
    ),
    // 043
    (
        "Flashfin",
        Lore {
            en: "The mica bulb on its brow flashes at the top of each leap. Ilse Marr logged three nights of falling stars before learning the truth.",
            fr: "Le bulbe de mica de son front éclate au sommet de chaque saut. Ilse Marr a noté trois nuits d'étoiles filantes avant de comprendre.",
        },
    ),
    // 044
    (
        "Squallkite",
        Lore {
            en: "It hangs in the gale without a wingbeat, its beaded tether trailing to the ground. When it dives, the lightning follows it down.",
            fr: "Il tient dans la rafale sans un battement d'aile, son attache perlée traînant au sol. Quand il plonge, la foudre le suit.",
        },
    ),
    // 045
    (
        "Knollop",
        Lore {
            en: "It carries a little hill of sandstone on its back. Anvilstead's miners follow its tunnels: a Knollop never digs where the roof will fall.",
            fr: "Il porte une petite colline de grès sur le dos. Les mineurs de Bourg-l'Enclume suivent ses galeries\u{a0}: un Mottelot ne creuse jamais là où le plafond va tomber.",
        },
    ),
    // 046
    (
        "Knollgrim",
        Lore {
            en: "Its back is a crust of jagged slate. It braces behind it at first, then loses its head entirely once the fight gets going.",
            fr: "Son dos est une croûte d'ardoise coupante. Il s'abrite d'abord derrière, puis perd complètement la tête une fois la bagarre lancée.",
        },
    ),
    // 047
    (
        "Knollossus",
        Lore {
            en: "Its back is a hillside with a tree on top. Every blow that cracks the hill only makes it angrier and stronger.",
            fr: "Son dos est un versant coiffé d'un arbre. Chaque coup qui fend sa colline le rend plus furieux et plus fort.",
        },
    ),
    // 048
    (
        "Shalepup",
        Lore {
            en: "A geode rattles under its chin. It knocks Scree Pass's cairns over for fun, then brings back the pebble from the top.",
            fr: "Une géode cliquette sous son menton. Il renverse les cairns du Col des Éboulis pour s'amuser, puis rapporte le caillou du sommet.",
        },
    ),
    // 049
    (
        "Shalehound",
        Lore {
            en: "Its geode rings like a bell when it bays. In the dark of the Undermine, it hunts by the echo that comes back off the rock.",
            fr: "Sa géode sonne comme une cloche quand il aboie. Dans le noir des Galeries Basses, il chasse à l'écho que lui renvoie la roche.",
        },
    ),
    // 050
    (
        "Cragmastiff",
        Lore {
            en: "A great geode hangs at its chest. When it bays, the earth splits, and on the Bastion the menhirs hum back.",
            fr: "Une grande géode pend sur son poitrail. Quand il aboie, la terre se fend, et au Bastion les menhirs lui répondent.",
        },
    ),
    // 051
    (
        "Brawlet",
        Lore {
            en: "A seam of quartz runs down its back, and every blow it takes grows a new crystal. Every crystal goes into the next punch.",
            fr: "Une veine de quartz court le long de son dos, et chaque coup reçu y fait pousser un cristal. Chaque cristal repart dans le coup suivant.",
        },
    ),
    // 052
    (
        "Ironbrawl",
        Lore {
            en: "Seven quartz crystals ridge its back, one for every grudge. It waits to be hit, then gives it all back at once.",
            fr: "Sept cristaux de quartz hérissent son dos, un par rancune. Il attend qu'on le frappe, puis rend tout d'un coup.",
        },
    ),
    // 053
    (
        "Tusklet",
        Lore {
            en: "Its snout ends in a disc of knapped flint. It charges walls, carts and duelists alike, and every charge chips the disc.",
            fr: "Son groin finit en disque de silex taillé. Il charge murs, wagonnets et duellistes, et chaque charge ébrèche le disque.",
        },
    ),
    // 054
    (
        "Gritfang",
        Lore {
            en: "A millstone turns in each of its cheeks. When it bites, the wheels lock, and it lets go only when it chooses to.",
            fr: "Une meule tourne dans chacune de ses joues. Quand il mord, les roues se bloquent, et il ne lâche que quand il le décide.",
        },
    ),
    // 055
    (
        "Bouldergut",
        Lore {
            en: "It carries a round boulder in an open cradle in its belly. To move a thing, it leans on it; to stop one, it drops the boulder.",
            fr: "Il porte un rocher rond dans un berceau ouvert au creux du ventre. Pour déplacer une chose, il s'appuie\u{202f}; pour l'arrêter, il lâche le rocher.",
        },
    ),
    // 056
    (
        "Slatebrow",
        Lore {
            en: "A wall of slate rises from its brow, one course added every winter. It lowers the wall and lets every blow land on stone.",
            fr: "Un mur d'ardoise s'élève de son front, une assise de plus chaque hiver. Il baisse le mur et laisse chaque coup tomber sur la pierre.",
        },
    ),
    // 057
    (
        "Wispun",
        Lore {
            en: "It hangs in the willows like a little paper lantern and hums anything nearby to sleep.",
            fr: "Il pend dans les saules comme une petite lanterne de papier et endort de son fredonnement tout ce qui l'entoure.",
        },
    ),
    // 058
    (
        "Wispelle",
        Lore {
            en: "Never follow two lights in the fog. The mist trailing from a Wispelle's veils brings bad dreams.",
            fr: "Ne suivez jamais deux lumières dans la brume. Celle qui traîne des voiles d'une Lueurelle apporte des cauchemars.",
        },
    ),
    // 059
    (
        "Wraithelle",
        Lore {
            en: "It closes its veil around whatever threatens the Moonwell. The intruder wakes at dawn far away, weak and with no memory of the night.",
            fr: "Elle referme son voile sur ce qui menace le Puits-de-Lune. L'intrus se réveille à l'aube, loin, affaibli et sans souvenir de la nuit.",
        },
    ),
    // 060
    (
        "Hexlet",
        Lore {
            en: "It perches on the wishing tree among the ribbons. One flicker of its little lamp, and a passer-by trips on nothing.",
            fr: "Il perche sur l'arbre à vœux parmi les rubans. Un vacillement de sa petite lampe, et un passant trébuche sur rien.",
        },
    ),
    // 061
    (
        "Hexmage",
        Lore {
            en: "It never forgets who swatted at it. Its jinxes land hardest on anyone already having a bad day.",
            fr: "Il n'oublie jamais qui l'a chassé d'un revers de main. Ses sorts frappent plus fort ceux qui passent déjà une mauvaise journée.",
        },
    ),
    // 062
    (
        "Hexarch",
        Lore {
            en: "Its crown of three lanterns flickers at every sound. Whoever stands nearest loses the thread of what they were doing.",
            fr: "Sa couronne de trois lanternes vacille au moindre bruit. Le plus proche perd le fil de ce qu'il faisait.",
        },
    ),
    // 063
    (
        "Dozel",
        Lore {
            en: "Its dreams drift in the glowing sac on its head. When it yawns, everyone around it yawns too.",
            fr: "Ses rêves dérivent dans le sac lumineux de sa tête. Quand il bâille, tout le monde autour bâille aussi.",
        },
    ),
    // 064
    (
        "Somnitar",
        Lore {
            en: "It sleeps sitting up on inn roofs. The cloud above its head gathers the dreams of everyone asleep below.",
            fr: "Il dort assis sur les toits des auberges. Le nuage au-dessus de sa tête recueille les rêves de tous les dormeurs.",
        },
    ),
    // 065
    (
        "Glimmox",
        Lore {
            en: "Whatever looks into the mirror on its brow sees its own move reflected back, and the Glimmox borrows it.",
            fr: "Qui regarde le miroir de son front y voit sa propre attaque se refléter, et le Miroitox la lui emprunte.",
        },
    ),
    // 066
    (
        "Gloamowl",
        Lore {
            en: "By day it is an unlit lantern on a jetty post. At dusk the shade opens, and whoever meets its stare feels night fall.",
            fr: "Le jour, c'est une lanterne éteinte sur un poteau. Au crépuscule l'abat-jour s'ouvre, et qui croise son regard sent la nuit tomber.",
        },
    ),
    // 067
    (
        "Moonmoth",
        Lore {
            en: "Between its raised wings it carries a small moon of its own, full at midnight. Where it lands, the moonflowers open wider.",
            fr: "Entre ses ailes dressées, il porte sa propre petite lune, pleine à minuit. Là où il se pose, les fleurs de lune s'ouvrent davantage.",
        },
    ),
    // 068
    (
        "Burrowbit",
        Lore {
            en: "It keeps whatever shines in the pouch on its chest. Meadow folk say a lost key is never lost, only borrowed.",
            fr: "Il garde tout ce qui brille dans la poche de sa poitrine. Au pré, on dit qu'une clé perdue n'est jamais perdue, seulement empruntée.",
        },
    ),
    // 069
    (
        "Warrenbuck",
        Lore {
            en: "Four ears hear four winds. Couriers race it at dawn on the six roads; nobody has won yet.",
            fr: "Quatre oreilles entendent quatre vents. Les coursiers des six routes font la course avec lui à l'aube\u{202f}; personne n'a encore gagné.",
        },
    ),
    // 070
    (
        "Warrenking",
        Lore {
            en: "Its ears have grown into a crown of chalk. When it arrives, the whole warren pours out to meet it with gifts.",
            fr: "Ses oreilles sont devenues une couronne de craie. Quand il arrive, toute la garenne accourt l'accueillir avec des présents.",
        },
    ),
    // 071
    (
        "Pecklet",
        Lore {
            en: "Its body is a felt bellows. It stands in the fountain for an hour, then puffs, and whoever was in its way is one step aside.",
            fr: "Son corps est un soufflet de feutre. Il reste une heure dans la fontaine, puis souffle, et l'importun se retrouve un pas plus loin.",
        },
    ),
    // 072
    (
        "Galeheron",
        Lore {
            en: "It wrings its nine-fold neck like a cloth, then lets go. The spiral gale it breathes out decides who steps up next.",
            fr: "Il tord son cou à neuf plis comme un linge, puis le relâche. La rafale en spirale qu'il souffle décide qui s'avance ensuite.",
        },
    ),
    // 073
    (
        "Scampit",
        Lore {
            en: "Its tail is a felt larder, open at the tip. Whatever it tucks in at dusk, it finds again at dawn, plus a crust it forgot.",
            fr: "Sa queue est un garde-manger de feutre ouvert au bout. Ce qu'il y range au soir, il le retrouve à l'aube, plus une croûte oubliée.",
        },
    ),
    // 074
    (
        "Echopup",
        Lore {
            en: "Its ears are two horn trumpets. It catches the sound of your attack, keeps it a moment, then plays it back at you.",
            fr: "Ses oreilles sont deux trompes de corne. Il capte le bruit de votre attaque, le garde un instant, puis vous le renvoie.",
        },
    ),
    // 075
    (
        "Haulox",
        Lore {
            en: "A yoke of chalk horn grows across its shoulders. Carters hitch it with no harness and chalk one stroke on its flank per load.",
            fr: "Un joug de corne de craie lui pousse sur les épaules. On l'attelle sans harnais, et chaque chargement vaut un bâton de craie sur son flanc.",
        },
    ),
    // 076
    (
        "Pocketmouse",
        Lore {
            en: "Its body is a ball of its own wound thread. Lost travellers follow the loose end out; so do tired duelists.",
            fr: "Son corps est une pelote de son propre fil. Les voyageurs perdus suivent le bout libre pour sortir, les duellistes las aussi.",
        },
    ),
    // 077
    (
        "Mendlark",
        Lore {
            en: "Its beak is a porcelain needle. It stitches its frayed felt back together as it sings, and the whole hedge settles down.",
            fr: "Son bec est une aiguille de porcelaine. Il recoud son feutre effiloché tout en chantant, et toute la haie s'apaise.",
        },
    ),
    // 078
    (
        "Hexcrest",
        Lore {
            en: "Six chalk horns crown it, one for each face, and its face stays blank for the seventh. Nothing that sours a duel will stick to it.",
            fr: "Six cornes de craie le couronnent, une par face, et sa face reste nue pour la septième. Rien de ce qui gâte un duel n'a prise sur lui.",
        },
    ),
];

/// The dex line of the creature with this English name.
pub fn lore(name: &str) -> Option<&'static Lore> {
    LORE.iter().find(|(n, _)| *n == name).map(|(_, l)| l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sixfold_engine::{set::CARDS, CardKind};

    #[test]
    fn every_hexon_has_a_dex_line_in_both_languages() {
        for def in CARDS
            .iter()
            .filter(|d| matches!(d.kind, CardKind::Creature(_)))
        {
            let l = lore(def.name).unwrap_or_else(|| panic!("{} has no dex line", def.name));
            assert!(
                !l.en.trim().is_empty() && !l.fr.trim().is_empty(),
                "{}",
                def.name
            );
        }
    }

    #[test]
    fn every_dex_line_names_a_hexon_once() {
        for (i, (name, _)) in LORE.iter().enumerate() {
            assert!(
                CARDS
                    .iter()
                    .any(|d| d.name == *name && matches!(d.kind, CardKind::Creature(_))),
                "{name} is not a creature card"
            );
            assert!(LORE[..i].iter().all(|(n, _)| n != name), "{name} twice");
        }
    }
}
