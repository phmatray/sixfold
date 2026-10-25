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
        ("Mirage brûlant", "Pendant le prochain tour de votre adversaire, la créature en défense doit obtenir face à pile ou face pour attaquer."),
        ("Piqué enflammé", "Cette créature s'inflige 10 dégâts."),
    ], None),
    c("Pyrequill", "Bûcheplume", &[
        ("Pluie d'étincelles", "Inflige 10 dégâts à chaque créature du Banc adverse."),
        ("Panache infernal", "Défaussez 2 cartes Énergie Feu attachées à cette créature."),
    ], Some(("Cœur de braise", "Attacher une énergie Feu à l'une de vos créatures Feu ne compte pas comme votre énergie attachée du tour."))),
    // ----- Ember Rush: Fighting -----
    c("Knollop", "Mottelot", &[
        ("Jet de motte", ""),
        ("Grondement", "Inflige 10 dégâts à chaque créature de Banc, y compris les vôtres."),
    ], None),
    c("Knollgrim", "Mottegrogne", &[
        ("Garde de granit", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à cette créature."),
        ("Charge furieuse", "Cette créature est maintenant Confuse."),
    ], Some(("Croûte acérée", "Chaque fois que cette créature subit des dégâts qui ne viennent pas d'un Pouvoir, la créature Active de votre adversaire subit 10 dégâts."))),
    c("Tusklet", "Défensin", &[
        ("Coup de défense", "Lancez 2 pièces. Cette attaque inflige 20 dégâts pour chaque face."),
        ("Charge sauvage", "Cette créature s'inflige 10 dégâts."),
    ], None),
    c("Gritfang", "Gravicroc", &[
        ("Gonflé à bloc", "Pendant votre prochain tour, l'attaque de cette créature inflige le double de dégâts."),
        ("Mâchoire broyeuse", "La créature en défense ne peut pas battre en retraite pendant le prochain tour de votre adversaire."),
    ], None),
    // ----- Tide Mind: Water line -----
    c("Puddock", "Flaquaud", &[
        ("Crachat de bulles", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
        ("Nuage de vase", "Votre adversaire ne peut pas jouer de cartes Dresseur pendant son prochain tour."),
    ], None),
    c("Brookhorn", "Ruissecorne", &[
        ("Remous", "Votre adversaire échange la créature en défense contre 1 des créatures de son Banc."),
        ("Coup de vague", ""),
    ], None),
    c("Tidemaw", "Marégueule", &[
        ("Trempette", "Défaussez 1 carte Énergie attachée à la créature en défense."),
        ("Grande déferlante", "Cette attaque ne peut être utilisée qu'une seule fois tant que cette créature reste en jeu."),
    ], Some(("Pluie apaisante", "Une fois pendant votre tour, vous pouvez soigner 10 dégâts de votre créature Active."))),
    c("Shellmote", "Coquillot", &[
        ("Repli", "Lancez une pièce. Si c'est face, évitez tous les dégâts infligés à cette créature par des attaques pendant le prochain tour de votre adversaire."),
        ("Détrempe", "La Faiblesse de la créature en défense devient Eau."),
    ], Some(("Coque flottante", "Tant que cette créature est sur votre Banc, le coût de Retraite de votre créature Active diminue de 1."))),
    // ----- Tide Mind: Psychic -----
    c("Wispun", "Lueurin", &[
        ("Berceuse", "La créature en défense est maintenant Endormie."),
        ("Chant du silence", "Choisissez 1 attaque de la créature en défense : elle ne pourra pas l'utiliser pendant le prochain tour de votre adversaire."),
    ], None),
    c("Wispelle", "Lueurelle", &[
        ("Brume hantée", "La créature en défense est maintenant Empoisonnée."),
        ("Brise-esprit", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Gravement empoisonnée."),
    ], None),
    c("Glimmox", "Miroitox", &[
        ("Rayon troublant", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Confuse."),
        ("Regard miroir", "Choisissez 1 attaque de la créature en défense et utilisez-la comme cette attaque."),
    ], Some(("Aiguille mentale", "Une fois pendant votre tour, si cette créature était en jeu au début de votre tour, vous pouvez infliger 10 dégâts à 1 des créatures de votre adversaire."))),
    // ----- Shared -----
    c("Scampit", "Galopiot", &[
        ("Grignotage", ""),
    ], Some(("Fouinage", "Une fois pendant votre tour, si cette créature était en jeu au début de votre tour, vous pouvez piocher une carte."))),
    // ----- Trainers -----
    t("Trail Compass", "Boussole", "Cherchez une carte créature de Base dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."),
    t("Lucky Satchel", "Besace fétiche", "Piochez 2 cartes."),
    t("Moss Poultice", "Cataplasme", "Soigne jusqu'à 30 dégâts de votre créature Active."),
    t("Clearwater Vial", "Fiole limpide", "Votre créature Active guérit de toutes ses altérations."),
    t("Relay Baton", "Témoin de relais", "Échangez votre créature Active contre 1 des créatures de votre Banc."),
    t("Snare Hook", "Crochet-piège", "Choisissez 1 des créatures du Banc adverse et échangez-la contre la créature Active de votre adversaire."),
    t("Drain Brush", "Brosse-siphon", "Défaussez 1 carte Énergie attachée à la créature Active de votre adversaire."),
    t("Spike Charm", "Talisman épineux", "Attachez cette carte à votre créature Active. Ses attaques infligent 10 dégâts de plus pendant ce tour."),
    t("Bark Shield", "Bouclier d'écorce", "Attachez cette carte à 1 de vos créatures. Les attaques lui infligent 20 dégâts de moins jusqu'à la fin du prochain tour de votre adversaire."),
    t("Recall Horn", "Cor de rappel", "Ajoutez une carte créature de votre défausse à votre main."),
    t("Bloom Seed", "Graine d'éclosion", "Posez une carte créature de Stade 2 de votre main sur la créature de Base correspondante en jeu. Celle-ci doit être en jeu depuis le début de votre tour."),
    t("Unwind Spray", "Brume régressive", "La créature Active évoluée de votre adversaire régresse au stade précédent. Mettez la carte du dessus dans la main de votre adversaire."),
    // ----- Energy -----
    e("Grass Energy", "Énergie Herbe"),
    e("Fire Energy", "Énergie Feu"),
    e("Water Energy", "Énergie Eau"),
    e("Lightning Energy", "Énergie Foudre"),
    e("Fighting Energy", "Énergie Combat"),
    e("Psychic Energy", "Énergie Psychique"),
    e("Twofold Energy", "Énergie Double"),
    // ===== 0.4.0 expansion =====
    // ----- Grass -----
    c("Sprigget", "Brindelet", &[
        ("Entaille feuillue", ""),
        ("Appel des racines", "Cherchez une carte Énergie Herbe dans votre deck et attachez-la à cette créature, puis mélangez votre deck."),
    ], None),
    c("Thornbuck", "Brocépine", &[
        ("Salve d'épines", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Empoisonnée."),
        ("Coup de bois", ""),
    ], None),
    c("Elderhart", "Sylvecerf", &[
        ("Écrasement sylvestre", "Soigne 20 dégâts de cette créature."),
    ], Some(("Souffle verdoyant", "Une fois pendant votre tour, vous pouvez soigner 10 dégâts de chacune de vos créatures."))),
    c("Mirebud", "Tourbouton", &[
        ("Spore des marais", "La créature en défense est maintenant Empoisonnée."),
        ("Jet de sève", ""),
    ], None),
    c("Bogblossom", "Tourbefleur", &[
        ("Pollen engourdissant", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
        ("Nuage putride", "La créature en défense est maintenant Empoisonnée."),
    ], None),
    c("Bogcrown", "Tourbereine", &[
        ("Floraison asphyxiante", "La créature en défense est maintenant Gravement empoisonnée."),
        ("Racine buveuse", "Soigne 30 dégâts de cette créature."),
    ], None),
    c("Pollenpup", "Pollinou", &[
        ("Reniflement", ""),
        ("Jet de pétales", ""),
    ], None),
    c("Honeymane", "Mielcrin", &[
        ("Nectar partagé", "Soigne 20 dégâts de chacune de vos créatures."),
        ("Coup de crinière", ""),
    ], None),
    c("Briarhog", "Roncisson", &[
        ("Roulé-boulé", ""),
        ("Barrage de ronces", ""),
    ], Some(("Peau piquante", "Chaque fois que cette créature subit des dégâts qui ne viennent pas d'un Pouvoir, la créature Active de votre adversaire subit 10 dégâts."))),
    c("Fernwhisk", "Fougerette", &[
        ("Coup d'éventail", "Échangez cette créature contre 1 des créatures de votre Banc."),
        ("Tir de graine", ""),
    ], None),
    // ----- Fire -----
    c("Ashpaw", "Cendrepatte", &[
        ("Griffe chaude", ""),
        ("Bond de flammes", "Défaussez 1 carte Énergie Feu attachée à cette créature."),
    ], None),
    c("Blazelynx", "Brasilynx", &[
        ("Griffe de braise", ""),
        ("Ruée flamboyante", "Défaussez 1 carte Énergie Feu attachée à cette créature."),
    ], None),
    c("Solarlynx", "Héliolynx", &[
        ("Crinière ardente", ""),
        ("Saut supernova", "Défaussez toutes les énergies attachées à cette créature."),
    ], None),
    c("Kilnback", "Fourdos", &[
        ("Jet de charbon", ""),
        ("Coup de four", ""),
    ], None),
    c("Furnaceback", "Fournaidos", &[
        ("Attisement", "Cherchez une carte Énergie Feu dans votre deck et attachez-la à cette créature, puis mélangez votre deck."),
        ("Vague de magma", "Défaussez 2 cartes Énergie Feu attachées à cette créature."),
    ], None),
    c("Waxwick", "Cirèche", &[
        ("Mèche vive", "Lancez une pièce. Si c'est face, cette attaque inflige 20 dégâts de plus."),
    ], None),
    c("Cinderhorn", "Braisecorne", &[
        ("Corne brûlante", ""),
        ("Charge ardente", "Cette créature s'inflige 20 dégâts."),
    ], None),
    // ----- Water -----
    c("Rillet", "Ruisselin", &[
        ("Éclaboussure", ""),
        ("Coup de limon", "Défaussez 1 carte Énergie attachée à la créature en défense."),
    ], None),
    c("Streamfin", "Nageflot", &[
        ("Nageoire tranchante", ""),
        ("Ressac", "Échangez la créature en défense contre 1 des créatures du Banc adverse, de votre choix."),
    ], None),
    c("Gulfwyrm", "Golfeguivre", &[
        ("Prison de givre", "Lancez une pièce. Si c'est face, la créature en défense ne peut pas attaquer pendant le prochain tour de votre adversaire."),
        ("Fracas des abysses", "Défaussez 1 carte Énergie attachée à la créature en défense."),
    ], None),
    c("Kelpup", "Varéchiot", &[
        ("Morsure salée", ""),
        ("Ligotage d'algues", "La créature en défense ne peut pas battre en retraite pendant le prochain tour de votre adversaire."),
    ], None),
    c("Kelpwarden", "Gardevarech", &[
        ("Don des marées", "Cherchez une carte Énergie Eau dans votre deck et attachez-la à cette créature, puis mélangez votre deck."),
        ("Courant broyeur", ""),
    ], None),
    c("Frosteel", "Givranguille", &[
        ("Éclat de givre", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
        ("Morsure glacée", ""),
    ], None),
    // ----- Lightning -----
    c("Sparkit", "Étincelot", &[
        ("Pincée statique", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
        ("Élan électrique", "Cette créature s'inflige 10 dégâts."),
    ], None),
    c("Arcfang", "Arcroc", &[
        ("Morsure d'arc", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
        ("Étincelle en chaîne", "Inflige 10 dégâts à chaque créature du Banc adverse."),
    ], None),
    c("Fulgurex", "Fulgurion", &[
        ("Éclair fourchu", "Inflige 20 dégâts à 1 des créatures de votre adversaire."),
        ("Déluge orageux", "Lancez 4 pièces. Cette attaque inflige 30 dégâts pour chaque face."),
    ], None),
    c("Buzzlet", "Bourdonnet", &[
        ("Bzzz", ""),
        ("Aile de choc", ""),
    ], None),
    c("Hivevolt", "Ruchevolt", &[
        ("Essaim survolté", "Lancez 3 pièces. Cette attaque inflige 10 dégâts de plus pour chaque face."),
        ("Champ statique", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
    ], None),
    c("Ohmlet", "Ohmelette", &[
        ("Mini-arc", "Inflige 10 dégâts à 1 des créatures de votre adversaire."),
        ("Bourdonnement", ""),
    ], None),
    c("Ohmrunner", "Ohmcoureur", &[
        ("Courant vif", "Cherchez une carte Énergie Foudre dans votre deck et attachez-la à cette créature, puis mélangez votre deck."),
        ("Tir sur rail", "Inflige 20 dégâts à 1 des créatures de votre adversaire."),
    ], None),
    c("Stormox", "Taurorage", &[
        ("Corne chargée", ""),
        ("Surcharge", "Cette créature s'inflige 20 dégâts."),
    ], None),
    c("Zippet", "Filéclair", &[
        ("Scintillement", ""),
        ("Saut statique", "Échangez cette créature contre 1 des créatures de votre Banc."),
    ], None),
    c("Flashfin", "Nagéclair", &[
        ("Morsure de choc", ""),
        ("Flash fulgurant", "Lancez une pièce. Si c'est face, cette attaque inflige 20 dégâts de plus."),
    ], None),
    // ----- Fighting -----
    c("Knollossus", "Mottecolosse", &[
        ("Piétinement sismique", "Inflige 10 dégâts à chaque créature de Banc, y compris les vôtres."),
        ("Chute de montagne", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur cette créature."),
    ], None),
    c("Brawlet", "Bagarrot", &[
        ("Petit poing", ""),
        ("Coup de rancune", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur cette créature."),
    ], None),
    c("Ironbrawl", "Ferbagarre", &[
        ("Contre-attaque", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur cette créature."),
        ("Crochet du droit", ""),
    ], None),
    c("Shalepup", "Schistiot", &[
        ("Coup de gravier", "Lancez une pièce. Si c'est face, cette attaque inflige 10 dégâts de plus."),
        ("Jet de roc", ""),
    ], None),
    c("Shalehound", "Schistedogue", &[
        ("Croc de pierre", ""),
        ("Ruée escarpée", "La créature en défense ne peut pas battre en retraite pendant le prochain tour de votre adversaire."),
    ], None),
    c("Cragmastiff", "Rochemâtin", &[
        ("Morsure sismique", "Pendant le prochain tour de votre adversaire, les attaques de la créature en défense infligent 20 dégâts de moins."),
        ("Fend-la-terre", "Défaussez 1 carte Énergie Combat attachée à cette créature."),
    ], None),
    c("Bouldergut", "Rocpanse", &[
        ("Poids mort", ""),
        ("Chute de rocher", ""),
    ], None),
    // ----- Psychic -----
    c("Wraithelle", "Spectrelle", &[
        ("Voile cauchemar", "La créature en défense est maintenant Endormie et Empoisonnée."),
        ("Siphon d'âme", "Soigne 20 dégâts de cette créature."),
    ], None),
    c("Dozel", "Roupillot", &[
        ("Regard somnolent", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Endormie."),
        ("Coup de rêve", ""),
    ], None),
    c("Somnitar", "Songeastre", &[
        ("Onde berceuse", "La créature en défense est maintenant Endormie."),
        ("Pression mentale", ""),
    ], None),
    c("Hexlet", "Sorcelet", &[
        ("Petit sort", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Confuse."),
        ("Mauvais œil", "Pendant le prochain tour de votre adversaire, la créature en défense doit obtenir face à pile ou face pour attaquer."),
    ], None),
    c("Hexmage", "Sorcemage", &[
        ("Torsion mentale", "La créature en défense est maintenant Confuse."),
        ("Éclair de dépit", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur la créature en défense."),
    ], None),
    c("Hexarch", "Sorcarque", &[
        ("Grand sortilège", "Cette attaque inflige 10 dégâts de plus pour chaque tranche de 10 dégâts sur la créature en défense."),
    ], Some(("Égarement", "Une fois pendant votre tour, si cette créature était en jeu au début de votre tour, vous pouvez lancer une pièce. Si c'est face, la créature Active de votre adversaire est maintenant Confuse."))),
    c("Gloamowl", "Crépuhibou", &[
        ("Regard du soir", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Endormie."),
        ("Piqué d'ombre", ""),
    ], None),
    // ----- Colorless -----
    c("Burrowbit", "Terrinot", &[
        ("Fouille", "Piochez une carte."),
        ("Bousculade", ""),
    ], None),
    c("Warrenbuck", "Garenbouquin", &[
        ("Coup de patte", ""),
        ("Fuite au terrier", "Échangez cette créature contre 1 des créatures de votre Banc."),
    ], None),
    c("Warrenking", "Garenroi", &[
        ("Débandade", ""),
        ("Séisme de garenne", ""),
    ], Some(("Bienvenue au terrier", "Quand vous mettez cette créature en jeu en la faisant évoluer, piochez 2 cartes."))),
    c("Pecklet", "Picorin", &[
        ("Coup de bec", ""),
        ("Battement d'ailes", "Votre adversaire échange la créature en défense contre 1 des créatures de son Banc."),
    ], None),
    c("Galeheron", "Rafaléron", &[
        ("Aile tranchante", "Lancez une pièce. Si c'est face, cette attaque inflige 20 dégâts de plus."),
        ("Tourbillon", "Échangez la créature en défense contre 1 des créatures du Banc adverse, de votre choix."),
    ], None),
    c("Echopup", "Échouaf", &[
        ("Jappement", ""),
        ("Écho", "Choisissez 1 attaque de la créature en défense et utilisez-la comme cette attaque."),
    ], None),
    c("Haulox", "Halebœuf", &[
        ("Coup d'épaule", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à cette créature."),
        ("Piétinement lourd", ""),
    ], None),
    c("Pocketmouse", "Souripoche", &[
        ("Grignote-miette", ""),
    ], Some(("Détalage", "Tant que cette créature est sur votre Banc, le coût de Retraite de votre créature Active diminue de 1."))),
    c("Mendlark", "Pansalouette", &[
        ("Gazouillis soignant", "Soigne 20 dégâts de cette créature."),
        ("Air apaisant", "Guérit les deux créatures Actives de toutes leurs altérations."),
    ], None),
    // ----- Trainers -----
    t("Dowsing Rod", "Baguette sourcière", "Cherchez une carte Énergie de base dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."),
    t("Field Guide", "Guide nature", "Cherchez une carte créature dans votre deck, ajoutez-la à votre main, puis mélangez votre deck."),
    t("Ash Sifter", "Tamis à cendres", "Ajoutez une carte Énergie de base de votre défausse à votre main."),
    t("Scrap Ledger", "Carnet de récup", "Ajoutez une carte Dresseur de votre défausse à votre main."),
    t("Clean Slate", "Table rase", "Défaussez votre main, puis piochez 7 cartes."),
    t("Grove Balm", "Baume du bosquet", "Soigne jusqu'à 20 dégâts de chacune de vos créatures."),
    t("Charge Coil", "Bobine de charge", "Cherchez une carte Énergie de base dans votre deck et attachez-la à votre créature Active, puis mélangez votre deck."),
    t("Jolt Wire", "Fil de choc", "Infligez 20 dégâts à 1 des créatures de votre adversaire."),
    t("Static Net", "Filet statique", "La créature Active de votre adversaire ne peut pas battre en retraite pendant son prochain tour."),
    t("Smoke Pellet", "Fumigène", "Pendant le prochain tour de votre adversaire, sa créature Active doit obtenir face à pile ou face pour attaquer."),
    t("Venom Dart", "Dard venimeux", "Lancez une pièce. Si c'est face, la créature Active de votre adversaire est maintenant Empoisonnée."),
    t("Wide Satchel", "Grande besace", "Piochez 3 cartes."),
    t("Hearty Broth", "Bouillon tonique", "Soigne jusqu'à 50 dégâts de votre créature Active."),
    // ----- Promos -----
    c("Thistlewick", "Chardomèche", &[
        ("Piqûre", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Empoisonnée."),
        ("Fouet de ronces", ""),
    ], None),
    c("Brinemoth", "Saumite", &[
        ("Contre-courant", "Échangez la créature en défense contre 1 des créatures du Banc adverse, de votre choix."),
        ("Gerbe saline", ""),
    ], None),
    c("Kilnfox", "Fournard", &[
        ("Attisement", "Cherchez une carte Énergie Feu dans votre deck et attachez-la à cette créature, puis mélangez votre deck."),
        ("Flambée", "Défaussez 1 carte Énergie Feu attachée à cette créature."),
    ], None),
    c("Squallkite", "Milanorage", &[
        ("Rafale statique", "Lancez une pièce. Si c'est face, la créature en défense est maintenant Paralysée."),
        ("Piqué tonnerre", "Cette créature s'inflige 10 dégâts."),
    ], None),
    c("Slatebrow", "Ardoisfront", &[
        ("Rempart rocheux", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à cette créature."),
        ("Éboulement", ""),
    ], None),
    c("Moonmoth", "Phalune", &[
        ("Ailes feutrées", "La créature en défense est maintenant Endormie."),
        ("Rayon de lune", "Lancez 2 pièces. Cette attaque inflige 20 dégâts de plus pour chaque face."),
    ], None),
    c("Hexcrest", "Sorcrête", &[
        ("Garde des Six", "Pendant le prochain tour de votre adversaire, les attaques infligent 20 dégâts de moins à cette créature."),
        ("Frappe couronnée", ""),
    ], Some(("Calme du champion", "Cette créature est immunisée contre les altérations."))),
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
