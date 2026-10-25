//! French text for the card set: one row per card of [`crate::set::CARDS`],
//! in the same order. Each row repeats the English name so the tests below
//! catch a row that drifted. The English `name` stays the identity key; these
//! are only what the UI shows (see [`crate::CardDef::label`]).

/// One card's French text.
#[derive(Debug)]
pub struct CardFr {
    /// The card's English name (alignment check only).
    #[cfg_attr(not(test), allow(dead_code))]
    pub en: &'static str,
    pub name: &'static str,
    /// (name, rules text) per attack, in the card's order. An attack with no
    /// English text has none here either.
    pub attacks: &'static [(&'static str, &'static str)],
    /// The Power's (name, rules text).
    pub power: Option<(&'static str, &'static str)>,
    /// A trainer's rules text (empty for creatures and energy).
    pub text: &'static str,
}

/// A creature (or Trainer-as-Basic).
const fn c(
    en: &'static str,
    name: &'static str,
    attacks: &'static [(&'static str, &'static str)],
    power: Option<(&'static str, &'static str)>,
) -> CardFr {
    CardFr {
        en,
        name,
        attacks,
        power,
        text: "",
    }
}

/// A trainer.
const fn t(en: &'static str, name: &'static str, text: &'static str) -> CardFr {
    CardFr {
        en,
        name,
        attacks: &[],
        power: None,
        text,
    }
}

/// An energy card.
const fn e(en: &'static str, name: &'static str) -> CardFr {
    CardFr {
        en,
        name,
        attacks: &[],
        power: None,
        text: "",
    }
}

#[rustfmt::skip]
pub static FR: &[CardFr] = &[
    // ----- Ember Rush: Fire line -----
    c("Cindrel", "Braisillon", &[
        ("Pichenette ardente", "Lancez une pièce. Si c'est face, cette attaque inflige 10 dégâts de plus."),
        ("Ruée de cendres", ""),
    ], None),
    c("Flarrow", "Flambèche", &[
        ("Mirage brûlant", "Pendant le prochain tour de votre adversaire, le Hexon en défense doit obtenir face à pile ou face pour attaquer."),
        ("Piqué enflammé", "Ce Hexon s'inflige 10 dégâts."),
    ], None),
    c("Pyrequill", "Bûcheplume", &[
        ("Pluie d'étincelles", "Inflige 10 dégâts à chaque Hexon du Banc adverse."),
        ("Panache infernal", "Défaussez 2 cartes Énergie Braise attachées à ce Hexon."),
    ], Some(("Cœur de braise", "Une fois pendant votre tour, attacher une énergie Braise à l'un de vos Hexons Braise ne compte pas comme votre énergie attachée du tour."))),
    // ----- Ember Rush: Fighting -----
    c("Knollop", "Mottelot", &[
        ("Jet de motte", ""),
        ("Grondement", "Inflige 10 dégâts à chaque Hexon de Banc, y compris les vôtres."),
    ], None),
    c("Knollgrim", "Mottegrogne", &[
        ("Garde de granit", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à ce Hexon."),
        ("Charge furieuse", "Ce Hexon est maintenant Confus."),
    ], Some(("Croûte acérée", "Chaque fois que ce Hexon subit des dégâts qui ne viennent pas d'un Pouvoir, le Hexon Actif de votre adversaire subit 10 dégâts."))),
    c("Tusklet", "Défensin", &[
        ("Coup de défense", "Lancez 2 pièces. Cette attaque inflige 20 dégâts pour chaque face."),
        ("Charge sauvage", "Ce Hexon s'inflige 10 dégâts."),
    ], None),
    c("Gritfang", "Gravicroc", &[
        ("Gonflé à bloc", "Pendant votre prochain tour, l'attaque de ce Hexon inflige le double de dégâts."),
        ("Mâchoire broyeuse", "Le Hexon en défense ne peut pas battre en retraite pendant le prochain tour de votre adversaire."),
    ], None),
    // ----- Tide Mind: Water line -----
    c("Puddock", "Flaquaud", &[
        ("Crachat de bulles", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
        ("Nuage de vase", "Votre adversaire ne peut pas jouer de cartes Dresseur pendant son prochain tour."),
    ], None),
    c("Brookhorn", "Ruissecorne", &[
        ("Remous", "Votre adversaire échange le Hexon en défense contre 1 des Hexons de son Banc."),
        ("Coup de vague", ""),
    ], None),
    c("Tidemaw", "Marégueule", &[
        ("Trempette", "Défaussez 1 carte Énergie attachée au Hexon en défense."),
        ("Grande déferlante", "Cette attaque ne peut être utilisée qu'une seule fois tant que ce Hexon reste en jeu."),
    ], Some(("Pluie apaisante", "Une fois pendant votre tour, vous pouvez soigner 10 dégâts de votre Hexon Actif."))),
    c("Shellmote", "Coquillot", &[
        ("Repli", "Lancez une pièce. Si c'est face, évitez tous les dégâts infligés à ce Hexon par des attaques pendant le prochain tour de votre adversaire."),
        ("Détrempe", "La Faiblesse du Hexon en défense devient Marée."),
    ], Some(("Coque flottante", "Tant que ce Hexon est sur votre Banc, le coût de Retraite de votre Hexon Actif diminue de 1."))),
    // ----- Tide Mind: Psychic -----
    c("Wispun", "Lueurin", &[
        ("Berceuse", "Le Hexon en défense est maintenant Endormi."),
        ("Chant du silence", "Choisissez 1 attaque du Hexon en défense : il ne pourra pas l'utiliser pendant le prochain tour de votre adversaire."),
    ], None),
    c("Wispelle", "Lueurelle", &[
        ("Brume hantée", "Le Hexon en défense est maintenant Empoisonné."),
        ("Brise-esprit", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Gravement empoisonné."),
    ], None),
    c("Glimmox", "Miroitox", &[
        ("Rayon troublant", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Confus."),
        ("Regard miroir", "Choisissez 1 attaque du Hexon en défense et utilisez-la comme cette attaque."),
    ], Some(("Aiguille mentale", "Une fois pendant votre tour, si ce Hexon était en jeu au début de votre tour, vous pouvez infliger 10 dégâts à 1 des Hexons de votre adversaire."))),
    // ----- Shared -----
    c("Scampit", "Galopiot", &[
        ("Grignotage", ""),
    ], Some(("Fouinage", "Une fois pendant votre tour, si ce Hexon était en jeu au début de votre tour, vous pouvez piocher une carte."))),
    // ----- Trainers -----
    t("Trail Compass", "Boussole", "Cherchez une carte Hexon de Base dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."),
    t("Lucky Satchel", "Besace fétiche", "Piochez 2 cartes."),
    t("Moss Poultice", "Cataplasme", "Soigne jusqu'à 30 dégâts de votre Hexon Actif."),
    t("Clearwater Vial", "Fiole limpide", "Votre Hexon Actif guérit de toutes ses altérations."),
    t("Relay Baton", "Témoin de relais", "Échangez votre Hexon Actif contre 1 des Hexons de votre Banc."),
    t("Snare Hook", "Crochet-piège", "Choisissez 1 des Hexons du Banc adverse et échangez-le contre le Hexon Actif de votre adversaire."),
    t("Drain Brush", "Brosse-siphon", "Défaussez 1 carte Énergie attachée au Hexon Actif de votre adversaire."),
    t("Spike Charm", "Talisman épineux", "Attachez cette carte à votre Hexon Actif. Ses attaques infligent 10 dégâts de plus pendant ce tour."),
    t("Bark Shield", "Bouclier d'écorce", "Attachez cette carte à 1 de vos Hexons. Les attaques lui infligent 20 dégâts de moins jusqu'à la fin du prochain tour de votre adversaire."),
    t("Recall Horn", "Cor de rappel", "Ajoutez une carte Hexon de votre défausse à votre main."),
    t("Bloom Seed", "Graine d'éclosion", "Posez une carte Éveil 2 de votre main sur le Hexon de Base correspondant en jeu. Celui-ci doit être en jeu depuis le début de votre tour."),
    t("Unwind Spray", "Brume régressive", "Le Hexon Actif éveillé de votre adversaire régresse au stade précédent. Mettez la carte du dessus dans la main de votre adversaire."),
    // ----- Energy -----
    e("Grass Energy", "Énergie Bosquet"),
    e("Fire Energy", "Énergie Braise"),
    e("Water Energy", "Énergie Marée"),
    e("Lightning Energy", "Énergie Orage"),
    e("Fighting Energy", "Énergie Roc"),
    e("Psychic Energy", "Énergie Songe"),
    e("Twofold Energy", "Énergie Double"),
    // ===== 0.4.0 expansion =====
    // ----- Grass -----
    c("Sprigget", "Brindelet", &[
        ("Entaille feuillue", ""),
        ("Appel des racines", "Cherchez une carte Énergie Bosquet dans votre deck et attachez-la à ce Hexon, puis mélangez votre deck."),
    ], None),
    c("Thornbuck", "Brocépine", &[
        ("Salve d'épines", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Empoisonné."),
        ("Coup de bois", ""),
    ], None),
    c("Elderhart", "Sylvecerf", &[
        ("Écrasement sylvestre", "Soigne 20 dégâts de ce Hexon."),
    ], Some(("Souffle verdoyant", "Une fois pendant votre tour, vous pouvez soigner 10 dégâts de chacun de vos Hexons."))),
    c("Mirebud", "Tourbouton", &[
        ("Spore des marais", "Le Hexon en défense est maintenant Empoisonné."),
        ("Jet de sève", ""),
    ], None),
    c("Bogblossom", "Tourbefleur", &[
        ("Pollen engourdissant", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
        ("Nuage putride", "Le Hexon en défense est maintenant Empoisonné."),
    ], None),
    c("Bogcrown", "Tourbereine", &[
        ("Floraison asphyxiante", "Le Hexon en défense est maintenant Gravement empoisonné."),
        ("Racine buveuse", "Soigne 30 dégâts de ce Hexon."),
    ], None),
    c("Pollenpup", "Pollinou", &[
        ("Reniflement", ""),
        ("Jet de pétales", ""),
    ], None),
    c("Honeymane", "Mielcrin", &[
        ("Nectar partagé", "Soigne 20 dégâts de chacun de vos Hexons."),
        ("Coup de crinière", ""),
    ], None),
    c("Briarhog", "Roncisson", &[
        ("Roulé-boulé", ""),
        ("Barrage de ronces", ""),
    ], Some(("Peau piquante", "Chaque fois que ce Hexon subit des dégâts qui ne viennent pas d'un Pouvoir, le Hexon Actif de votre adversaire subit 10 dégâts."))),
    c("Fernwhisk", "Fougerette", &[
        ("Coup d'éventail", "Échangez ce Hexon contre 1 des Hexons de votre Banc."),
        ("Tir de graine", ""),
    ], None),
    // ----- Fire -----
    c("Ashpaw", "Cendrepatte", &[
        ("Griffe chaude", ""),
        ("Bond de flammes", "Défaussez 1 carte Énergie Braise attachée à ce Hexon."),
    ], None),
    c("Blazelynx", "Brasilynx", &[
        ("Griffe de braise", ""),
        ("Ruée flamboyante", "Défaussez 1 carte Énergie Braise attachée à ce Hexon."),
    ], None),
    c("Solarlynx", "Héliolynx", &[
        ("Crinière ardente", ""),
        ("Saut supernova", "Défaussez toutes les énergies attachées à ce Hexon."),
    ], None),
    c("Kilnback", "Fourdos", &[
        ("Jet de charbon", ""),
        ("Coup de four", ""),
    ], None),
    c("Furnaceback", "Fournaidos", &[
        ("Attisement", "Cherchez une carte Énergie Braise dans votre deck et attachez-la à ce Hexon, puis mélangez votre deck."),
        ("Vague de magma", "Défaussez 2 cartes Énergie Braise attachées à ce Hexon."),
    ], None),
    c("Waxwick", "Cirèche", &[
        ("Mèche vive", "Lancez une pièce. Si c'est face, cette attaque inflige 20 dégâts de plus."),
    ], None),
    c("Cinderhorn", "Braisecorne", &[
        ("Corne brûlante", ""),
        ("Charge ardente", "Ce Hexon s'inflige 20 dégâts."),
    ], None),
    // ----- Water -----
    c("Rillet", "Ruisselin", &[
        ("Éclaboussure", ""),
        ("Coup de limon", "Défaussez 1 carte Énergie attachée au Hexon en défense."),
    ], None),
    c("Streamfin", "Nageflot", &[
        ("Nageoire tranchante", ""),
        ("Ressac", "Échangez le Hexon en défense contre 1 des Hexons du Banc adverse, de votre choix."),
    ], None),
    c("Gulfwyrm", "Golfeguivre", &[
        ("Prison de givre", "Lancez une pièce. Si c'est face, le Hexon en défense ne peut pas attaquer pendant le prochain tour de votre adversaire."),
        ("Fracas des abysses", "Défaussez 1 carte Énergie attachée au Hexon en défense."),
    ], None),
    c("Kelpup", "Varéchiot", &[
        ("Morsure salée", ""),
        ("Ligotage d'algues", "Le Hexon en défense ne peut pas battre en retraite pendant le prochain tour de votre adversaire."),
    ], None),
    c("Kelpwarden", "Gardevarech", &[
        ("Don des marées", "Cherchez une carte Énergie Marée dans votre deck et attachez-la à ce Hexon, puis mélangez votre deck."),
        ("Courant broyeur", ""),
    ], None),
    c("Frosteel", "Givranguille", &[
        ("Éclat de givre", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
        ("Morsure glacée", ""),
    ], None),
    // ----- Lightning -----
    c("Sparkit", "Étincelot", &[
        ("Pincée statique", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
        ("Élan électrique", "Ce Hexon s'inflige 10 dégâts."),
    ], None),
    c("Arcfang", "Arcroc", &[
        ("Morsure d'arc", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
        ("Étincelle en chaîne", "Inflige 10 dégâts à chaque Hexon du Banc adverse."),
    ], None),
    c("Fulgurex", "Fulgurion", &[
        ("Éclair fourchu", "Inflige 20 dégâts à 1 des Hexons de votre adversaire."),
        ("Déluge orageux", "Lancez 4 pièces. Cette attaque inflige 30 dégâts pour chaque face."),
    ], None),
    c("Buzzlet", "Bourdonnet", &[
        ("Bzzz", ""),
        ("Aile de choc", ""),
    ], None),
    c("Hivevolt", "Ruchevolt", &[
        ("Essaim survolté", "Lancez 3 pièces. Cette attaque inflige 10 dégâts de plus pour chaque face."),
        ("Champ statique", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
    ], None),
    c("Ohmlet", "Ohmelette", &[
        ("Mini-arc", "Inflige 10 dégâts à 1 des Hexons de votre adversaire."),
        ("Bourdonnement", ""),
    ], None),
    c("Ohmrunner", "Ohmcoureur", &[
        ("Courant vif", "Cherchez une carte Énergie Orage dans votre deck et attachez-la à ce Hexon, puis mélangez votre deck."),
        ("Tir sur rail", "Inflige 20 dégâts à 1 des Hexons de votre adversaire."),
    ], None),
    c("Stormox", "Taurorage", &[
        ("Corne chargée", ""),
        ("Surcharge", "Ce Hexon s'inflige 20 dégâts."),
    ], None),
    c("Zippet", "Filéclair", &[
        ("Scintillement", ""),
        ("Saut statique", "Échangez ce Hexon contre 1 des Hexons de votre Banc."),
    ], None),
    c("Flashfin", "Nagéclair", &[
        ("Morsure de choc", ""),
        ("Flash fulgurant", "Lancez une pièce. Si c'est face, cette attaque inflige 20 dégâts de plus."),
    ], None),
    // ----- Fighting -----
    c("Knollossus", "Mottecolosse", &[
        ("Piétinement sismique", "Inflige 10 dégâts à chaque Hexon de Banc, y compris les vôtres."),
        ("Chute de montagne", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur ce Hexon."),
    ], None),
    c("Brawlet", "Bagarrot", &[
        ("Petit poing", ""),
        ("Coup de rancune", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur ce Hexon."),
    ], None),
    c("Ironbrawl", "Ferbagarre", &[
        ("Contre-attaque", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur ce Hexon."),
        ("Crochet du droit", ""),
    ], None),
    c("Shalepup", "Schistiot", &[
        ("Coup de gravier", "Lancez une pièce. Si c'est face, cette attaque inflige 10 dégâts de plus."),
        ("Jet de roc", ""),
    ], None),
    c("Shalehound", "Schistedogue", &[
        ("Croc de pierre", ""),
        ("Ruée escarpée", "Le Hexon en défense ne peut pas battre en retraite pendant le prochain tour de votre adversaire."),
    ], None),
    c("Cragmastiff", "Rochemâtin", &[
        ("Morsure sismique", "Pendant le prochain tour de votre adversaire, les attaques du Hexon en défense infligent 20 dégâts de moins."),
        ("Fend-la-terre", "Défaussez 1 carte Énergie Roc attachée à ce Hexon."),
    ], None),
    c("Bouldergut", "Rocpanse", &[
        ("Poids mort", ""),
        ("Chute de rocher", ""),
    ], None),
    // ----- Psychic -----
    c("Wraithelle", "Spectrelle", &[
        ("Voile cauchemar", "Le Hexon en défense est maintenant Endormi et Empoisonné."),
        ("Siphon d'âme", "Soigne 20 dégâts de ce Hexon."),
    ], None),
    c("Dozel", "Roupillot", &[
        ("Regard somnolent", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Endormi."),
        ("Coup de rêve", ""),
    ], None),
    c("Somnitar", "Songeastre", &[
        ("Onde berceuse", "Le Hexon en défense est maintenant Endormi."),
        ("Pression mentale", ""),
    ], None),
    c("Hexlet", "Sorcelet", &[
        ("Petit sort", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Confus."),
        ("Mauvais œil", "Pendant le prochain tour de votre adversaire, le Hexon en défense doit obtenir face à pile ou face pour attaquer."),
    ], None),
    c("Hexmage", "Sorcemage", &[
        ("Torsion mentale", "Le Hexon en défense est maintenant Confus."),
        ("Éclair de dépit", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur le Hexon en défense."),
    ], None),
    c("Hexarch", "Sorcarque", &[
        ("Grand sortilège", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur le Hexon en défense."),
    ], Some(("Égarement", "Une fois pendant votre tour, si ce Hexon était en jeu au début de votre tour, vous pouvez lancer une pièce. Si c'est face, le Hexon Actif de votre adversaire est maintenant Confus."))),
    c("Gloamowl", "Crépuhibou", &[
        ("Regard du soir", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Endormi."),
        ("Piqué d'ombre", ""),
    ], None),
    // ----- Colorless -----
    c("Burrowbit", "Terrinot", &[
        ("Fouille", "Piochez une carte."),
        ("Bousculade", ""),
    ], None),
    c("Warrenbuck", "Garenbouquin", &[
        ("Coup de patte", ""),
        ("Fuite au terrier", "Échangez ce Hexon contre 1 des Hexons de votre Banc."),
    ], None),
    c("Warrenking", "Garenroi", &[
        ("Débandade", ""),
        ("Séisme de garenne", ""),
    ], Some(("Bienvenue au terrier", "Quand vous mettez ce Hexon en jeu en l'éveillant, piochez 2 cartes."))),
    c("Pecklet", "Picorin", &[
        ("Coup de bec", ""),
        ("Battement d'ailes", "Votre adversaire échange le Hexon en défense contre 1 des Hexons de son Banc."),
    ], None),
    c("Galeheron", "Rafaléron", &[
        ("Aile tranchante", "Lancez une pièce. Si c'est face, cette attaque inflige 20 dégâts de plus."),
        ("Tourbillon", "Échangez le Hexon en défense contre 1 des Hexons du Banc adverse, de votre choix."),
    ], None),
    c("Echopup", "Échouaf", &[
        ("Jappement", ""),
        ("Écho", "Choisissez 1 attaque du Hexon en défense et utilisez-la comme cette attaque."),
    ], None),
    c("Haulox", "Halebœuf", &[
        ("Coup d'épaule", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à ce Hexon."),
        ("Piétinement lourd", ""),
    ], None),
    c("Pocketmouse", "Souripoche", &[
        ("Grignote-miette", ""),
    ], Some(("Détalage", "Tant que ce Hexon est sur votre Banc, le coût de Retraite de votre Hexon Actif diminue de 1."))),
    c("Mendlark", "Pansalouette", &[
        ("Gazouillis soignant", "Soigne 20 dégâts de ce Hexon."),
        ("Air apaisant", "Guérit les deux Hexons Actifs de toutes leurs altérations."),
    ], None),
    // ----- Trainers -----
    t("Dowsing Rod", "Baguette sourcière", "Cherchez une carte Énergie de base dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."),
    t("Field Guide", "Guide nature", "Cherchez une carte Hexon dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."),
    t("Ash Sifter", "Tamis à cendres", "Ajoutez une carte Énergie de base de votre défausse à votre main."),
    t("Scrap Ledger", "Carnet de récup", "Ajoutez une carte Dresseur de votre défausse à votre main."),
    t("Clean Slate", "Table rase", "Défaussez votre main, puis piochez 7 cartes."),
    t("Grove Balm", "Baume du bosquet", "Soigne jusqu'à 20 dégâts de chacun de vos Hexons."),
    t("Charge Coil", "Bobine de charge", "Cherchez une carte Énergie de base dans votre deck et attachez-la à votre Hexon Actif, puis mélangez votre deck."),
    t("Jolt Wire", "Fil de choc", "Infligez 20 dégâts à 1 des Hexons de votre adversaire."),
    t("Static Net", "Filet statique", "Le Hexon Actif de votre adversaire ne peut pas battre en retraite pendant son prochain tour."),
    t("Smoke Pellet", "Fumigène", "Pendant le prochain tour de votre adversaire, son Hexon Actif doit obtenir face à pile ou face pour attaquer."),
    t("Venom Dart", "Dard venimeux", "Lancez une pièce. Si c'est face, le Hexon Actif de votre adversaire est maintenant Empoisonné."),
    t("Wide Satchel", "Grande besace", "Piochez 3 cartes."),
    t("Hearty Broth", "Bouillon tonique", "Soigne jusqu'à 50 dégâts de votre Hexon Actif."),
    // ----- Promos -----
    c("Thistlewick", "Chardomèche", &[
        ("Piqûre", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Empoisonné."),
        ("Fouet de ronces", ""),
    ], None),
    c("Brinemoth", "Saumite", &[
        ("Contre-courant", "Échangez le Hexon en défense contre 1 des Hexons du Banc adverse, de votre choix."),
        ("Gerbe saline", ""),
    ], None),
    c("Kilnfox", "Fournard", &[
        ("Attisement", "Cherchez une carte Énergie Braise dans votre deck et attachez-la à ce Hexon, puis mélangez votre deck."),
        ("Flambée", "Défaussez 1 carte Énergie Braise attachée à ce Hexon."),
    ], None),
    c("Squallkite", "Milanorage", &[
        ("Rafale statique", "Lancez une pièce. Si c'est face, le Hexon en défense est maintenant Paralysé."),
        ("Piqué tonnerre", "Ce Hexon s'inflige 10 dégâts."),
    ], None),
    c("Slatebrow", "Ardoisfront", &[
        ("Rempart rocheux", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à ce Hexon."),
        ("Éboulement", ""),
    ], None),
    c("Moonmoth", "Phalune", &[
        ("Ailes feutrées", "Le Hexon en défense est maintenant Endormi."),
        ("Rayon de lune", "Lancez 2 pièces. Cette attaque inflige 20 dégâts de plus pour chaque face."),
    ], None),
    c("Hexcrest", "Sorcrête", &[
        ("Garde des Six", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à ce Hexon."),
        ("Frappe couronnée", ""),
    ], Some(("Calme du champion", "Ce Hexon est immunisé contre les altérations."))),
];

#[cfg(test)]
mod tests {
    use super::FR;
    use crate::card::CardKind;
    use crate::set::CARDS;

    /// One French row per card, in order, with the same attacks, Power and
    /// trainer text, nothing left empty that English fills, and unique names.
    #[test]
    fn french_rows_match_the_set() {
        assert_eq!(FR.len(), CARDS.len(), "one French row per card");
        for (i, (c, f)) in CARDS.iter().zip(FR).enumerate() {
            assert_eq!(f.en, c.name, "row {i} is out of order");
            assert!(!f.name.trim().is_empty(), "{}: no French name", c.name);
            assert!(
                FR[..i].iter().all(|g| g.name != f.name),
                "{}: French name {} used twice",
                c.name,
                f.name
            );
            let (attacks, power, text) = match &c.kind {
                CardKind::Creature(cr) | CardKind::TrainerBasic(cr) => {
                    (cr.attacks, cr.power.as_ref(), "")
                }
                CardKind::Trainer(t) => (&[][..], None, t.text),
                CardKind::Energy(_) => (&[][..], None, ""),
            };
            assert_eq!(f.attacks.len(), attacks.len(), "{}: attack count", c.name);
            for (a, (name, rules)) in attacks.iter().zip(f.attacks) {
                assert!(
                    !name.is_empty(),
                    "{}: {} has no French name",
                    c.name,
                    a.name
                );
                assert_eq!(
                    a.text.is_empty(),
                    rules.is_empty(),
                    "{}: {} text",
                    c.name,
                    a.name
                );
            }
            assert_eq!(power.is_some(), f.power.is_some(), "{}: Power", c.name);
            if let (Some(p), Some((name, rules))) = (power, f.power) {
                assert!(
                    !name.is_empty() && !rules.is_empty(),
                    "{}: {}",
                    c.name,
                    p.name
                );
            }
            assert_eq!(
                text.is_empty(),
                f.text.is_empty(),
                "{}: trainer text",
                c.name
            );
        }
    }

    /// The accessors switch with the language; the identity name doesn't.
    #[test]
    fn labels_follow_the_language() {
        use crate::lang::{set_lang, Lang};
        let c = &CARDS[0];
        let a = &c.creature().unwrap().attacks[0];
        set_lang(Lang::Fr);
        let fr = (c.label(), a.label(), a.rules(), c.name);
        set_lang(Lang::En);
        assert_eq!(fr.0, FR[0].name);
        assert_eq!(fr.1, FR[0].attacks[0].0);
        assert_eq!(fr.2, FR[0].attacks[0].1);
        assert_eq!(fr.3, "Cindrel");
        assert_eq!(c.label(), "Cindrel");
        assert_eq!(a.label(), a.name);
    }
}
