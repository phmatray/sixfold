//! The world's items: the name, the model the item preview shows, and the blurb of each.

use sixfold_engine::tr;

/// Display name, model and blurb of an item.
pub(super) fn item_info(id: &str) -> (String, &'static str, &'static str) {
    match id {
        "running_shoes" => (
            tr("Running Shoes", "Chaussures de course").into(),
            "shoes",
            tr(
                "Light soles, quick feet. Hold {run}, or switch on Run, to cover two tiles in the time of one.",
                "Semelles légères, pas rapides. Maintenez {run}, ou activez Courir, pour franchir deux cases le temps d'une.",
            ),
        ),
        "field_lens" => (
            tr("Field Lens", "Longue-vue").into(),
            "lens",
            tr(
                "A brass birdwatcher's lens with a scratched cap. Somebody on the woodland trail will want it back.",
                "Une lunette d'ornithologue en laiton, au capuchon rayé. Quelqu'un, sur le sentier des bois, voudra la récupérer.",
            ),
        ),
        "letter" => (
            tr("Letter for Hob", "Lettre pour Hob").into(),
            "letter",
            tr(
                "Sealed with a wax hexagon and smelling faintly of the sea. Hob bakes in Brightwell Square.",
                "Scellée d'un hexagone de cire, elle sent vaguement la mer. Hob fait son pain sur la Place de Clairpuits.",
            ),
        ),
        // The six 1.2 curios (WORLD_DESIGN §5.2): Odile in the square counts them.
        "curio_brass_thimble" => (
            tr("Brass Thimble", "Dé à coudre en laiton").into(),
            "brass_thimble",
            tr(
                "Dented on one side, polished bright on the other. Somebody sewed a great deal with it. A curio: Odile in Brightwell Square collects them.",
                "Cabossé d'un côté, poli à force de servir de l'autre. Quelqu'un a beaucoup cousu avec. Une curiosité\u{a0}: Odile, sur la Place de Clairpuits, les collectionne.",
            ),
        ),
        "curio_heron_feather" => (
            tr("Heron Feather", "Plume de héron").into(),
            "heron_feather",
            tr(
                "Long, grey and perfect. The herons don't miss it. A curio for Odile's cabinet.",
                "Longue, grise, parfaite. Les hérons ne s'en plaindront pas. Une curiosité pour la vitrine d'Odile.",
            ),
        ),
        "curio_message_bottle" => (
            tr("Message in a Bottle", "Bouteille à la mer").into(),
            "message_bottle",
            tr(
                "The cork is waxed shut and the paper inside says, as far as you can read it, \"Wish you were here.\" A curio for Odile.",
                "Le bouchon est scellé à la cire et le papier dit, autant qu'on puisse lire\u{a0}: « J'aurais aimé que tu sois là. » Une curiosité pour Odile.",
            ),
        ),
        "curio_fossil_fern" => (
            tr("Fossil Fern", "Fougère fossile").into(),
            "fossil_fern",
            tr(
                "A fern pressed into stone long before the quarry had a name. A curio for Odile.",
                "Une fougère pressée dans la pierre bien avant que la carrière ait un nom. Une curiosité pour Odile.",
            ),
        ),
        "curio_tin_hex" => (
            tr("Painted Tin Hex", "Hexagone en fer-blanc peint").into(),
            "tin_hex",
            tr(
                "A toy seal, six colours flaking off a tin hexagon. Every child in Brightwell had one once. A curio for Odile.",
                "Un sceau jouet\u{a0}: six couleurs qui s'écaillent sur un hexagone de fer-blanc. Tous les enfants de Clairpuits en ont eu un. Une curiosité pour Odile.",
            ),
        ),
        "curio_lead_type" => (
            tr("Lead Type \"S\"", "Caractère de plomb « S »").into(),
            "lead_type",
            tr(
                "A single letter S, cast backwards in lead. Gus swears it isn't missing from anything. A curio for Odile.",
                "Une lettre S, fondue à l'envers dans le plomb. Gus jure qu'elle ne manque nulle part. Une curiosité pour Odile.",
            ),
        ),
        "seed_packet_1" | "seed_packet_2" | "seed_packet_3" => (
            tr("Seed Packet", "Sachet de graines").into(),
            "curio",
            tr(
                "A twist of paper with Moss's handwriting on it: \"Runner beans. Do NOT eat.\" Moss tends the garden in Sixfold Meadow.",
                "Un cornet de papier, de l'écriture de Moss\u{a0}: « Haricots grimpants. NE PAS MANGER. » Moss soigne le potager du Pré des Six.",
            ),
        ),
        "kite_line" => (
            tr("Kite Line", "Ligne de cerf-volant").into(),
            "curio",
            tr(
                "Skye's spare reel of waxed line, long enough to reach the top of a lighthouse. Dace on the boardwalk is waiting.",
                "La bobine de secours de Skye, une ligne cirée assez longue pour atteindre le haut d'un phare. Dace attend sur la promenade.",
            ),
        ),
        "kiln_ash" => (
            tr("Kiln Ash", "Cendre de four").into(),
            "curio",
            tr(
                "A pouch of fine grey ash from Cole's kiln, still faintly warm. Hob wants it for an ash-crust loaf.",
                "Un sachet de cendre grise et fine, tiré du four de Cole, encore tiède. Hob la veut pour un pain en croûte de cendre.",
            ),
        ),
        "press_voucher" => (
            tr("Press Voucher", "Bon de l'Imprimerie").into(),
            "letter",
            tr(
                "Good for one card, any card that isn't a promo, Rares included. Gus at Brightwell Press prints it while you wait.",
                "Valable pour une carte, n'importe laquelle sauf une promo, Rares comprises. Gus, à l'Imprimerie de Clairpuits, l'imprime pendant que vous attendez.",
            ),
        ),
        // 1.3 curios (Odile still counts them) and the Lantern quest's cullet.
        "curio_glass_marble" => (
            tr("Glass Marble", "Bille de verre").into(),
            "glass_marble",
            tr(
                "As big as a plum, with a twist of ember caught inside. Cinderwick glass. A curio for Odile's cabinet.",
                "Grosse comme une prune, avec une volute de braise prise dedans. Du verre de Mèche-Cendre. Une curiosité pour la vitrine d'Odile.",
            ),
        ),
        "curio_bottled_spark" => (
            tr("Bottled Spark", "Étincelle en bouteille").into(),
            "bottled_spark",
            tr(
                "A stoppered bottle with a thread of lightning crawling about inside. It hums if you shake it. A curio for Odile.",
                "Une bouteille bouchée où rampe un fil d'éclair. Elle bourdonne quand on la secoue. Une curiosité pour Odile.",
            ),
        ),
        "curio_bell_clapper" => (
            tr("Bell Clapper", "Battant de cloche").into(),
            "bell_clapper",
            tr(
                "The drowned bell's tongue, green with the sea, its striking ball worn bright. Still rings if you knock it. A curio for Odile.",
                "La langue de la cloche noyée, verdie par la mer, la boule polie à force de frapper. Elle tinte encore quand on la cogne. Une curiosité pour Odile.",
            ),
        ),
        "curio_moth_brooch" => (
            tr("Moth Brooch", "Broche phalène").into(),
            "moth_brooch",
            tr(
                "A silver moth with two moon-eyes on its wings, which glow faintly in the dark. Someone lost it in the well stair long ago. A curio for Odile.",
                "Une phalène d'argent, deux yeux de lune sur les ailes, qui luisent un peu dans le noir. Quelqu'un l'a perdue dans l'escalier du puits, il y a longtemps. Une curiosité pour Odile.",
            ),
        ),
        "cullet_quarry" | "cullet_town" | "cullet_kiln" => (
            tr("Cullet", "Calcin").into(),
            "cullet",
            tr(
                "A shard of the old green glass, as thick as a thumb. Clio at the Cinderwick Glassworks wants three.",
                "Un éclat de vieux verre vert, épais comme un pouce. Clio, à la Verrerie de Mèche-Cendre, en veut trois.",
            ),
        ),
        // The 1.3 field items (WORLD_DESIGN §7.1): each opens gates in other regions.
        "kiln_lantern" => (
            tr("Kiln Lantern", "Lanterne de four").into(),
            "kiln_lantern",
            tr(
                "Glass blown at Cinderwick round a coal that never quite goes out. Dark tunnels and caves light up around you.",
                "Un verre soufflé à Mèche-Cendre autour d'une braise qui ne s'éteint jamais tout à fait. Les tunnels et les grottes s'éclairent autour de vous.",
            ),
        ),
        "ferry_pass" => (
            tr("Ferry Pass", "Laissez-passer du bac").into(),
            "ferry_pass",
            tr(
                "Stamped by the Gullhaven Harbour Office. Show it on any jetty and the ferry takes you across.",
                "Tamponné par la Capitainerie de Havre-aux-Mouettes. Montrez-le sur n'importe quel embarcadère et le bac vous fait traverser.",
            ),
        ),
        "wading_boots" => (
            tr("Wading Boots", "Bottes de gué").into(),
            "wading_boots",
            tr(
                "Waxed reed and leather, laced to the knee. Fords and shallows are a path now.",
                "Du roseau ciré et du cuir, lacés jusqu'au genou. Gués et hauts-fonds deviennent des chemins.",
            ),
        ),
        "grip_gloves" => (
            tr("Grip Gloves", "Gants d'escalade").into(),
            "grip_gloves",
            tr(
                "Chalked palms, stitched fingertips. Walls with handholds can be climbed.",
                "Paumes à la craie, bouts des doigts cousus. Les parois à prises s'escaladent.",
            ),
        ),
        // 1.3 Grove and Stone: errands and curios.
        "reed_fibre" => (
            tr("Reed Fibre", "Fibres de roseau").into(),
            "reed_fibre",
            tr(
                "Long pale reeds from the Fernhollow stream, stripped and tied. Tobiah the cobbler wants them for wading boots.",
                "De longs roseaux pâles du ruisseau de Creuse-Fougère, effeuillés et liés. Tobiah le cordonnier les veut pour des bottes de gué.",
            ),
        ),
        "honey_jar" => (
            tr("Jar of Honey", "Pot de miel").into(),
            "curio",
            tr(
                "Hazel's new honey, still warm from the comb. For her mother Agnes in Rootholm.",
                "Le miel nouveau de Hazel, encore tiède du rayon. Pour sa mère Agnes, à Racinemont.",
            ),
        ),
        "lunch_pail" => (
            tr("Lunch Pail", "Gamelle").into(),
            "lunch_pail",
            tr(
                "The crew's lunch from the Tally Room: four pasties, two apples, a flask of tea. Counted. For Ros at the Undermine's east cut.",
                "Le déjeuner de l'équipe, de la Salle des Comptes\u{a0}: quatre chaussons, deux pommes, une gourde de thé. Compté. Pour Ros, à la saignée est des Galeries Basses.",
            ),
        ),
        "lucky_pick" => (
            tr("Lucky Pick", "Pioche porte-bonheur").into(),
            "lucky_pick",
            tr(
                "Huw's pick, its handle wrapped in faded red cord, his initials burned into the wood. Forty years lucky.",
                "La pioche de Huw, le manche entouré d'une cordelette rouge passée, ses initiales brûlées dans le bois. Quarante ans de chance.",
            ),
        ),
        "curio_acorn_whistle" => (
            tr("Acorn Whistle", "Sifflet-gland").into(),
            "acorn_whistle",
            tr(
                "An acorn as big as a fist, hollowed and pierced. Blow into it and it hoots like an owl. A curio for Odile.",
                "Un gland gros comme un poing, creusé et percé. On souffle dedans, il hulule comme une chouette. Une curiosité pour Odile.",
            ),
        ),
        "curio_miner_tally" => (
            tr("Miner's Tally", "Jetons de mineur").into(),
            "miner_tally",
            tr(
                "A ring of brass tally tags from the old Undermine, one punched with a hexagon. A curio for Odile.",
                "Un anneau de jetons de laiton des anciennes Galeries, l'un poinçonné d'un hexagone. Une curiosité pour Odile.",
            ),
        ),
        other => (other.replace('_', " "), "", ""),
    }
}
