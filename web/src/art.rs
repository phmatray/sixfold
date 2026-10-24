//! Original procedural card art: SVG strings built from a hash of the card
//! name, so each card always gets the same picture. No external images.

use std::fmt::Write;

use pkmntcg_engine::{CardDef, CardKind, Color, Creature, EnergyCard, Stage};

/// CSS class suffix for a colour (`t-fire` etc. in style.css).
pub fn color_key(c: Color) -> &'static str {
    match c {
        Color::Grass => "grass",
        Color::Fire => "fire",
        Color::Water => "water",
        Color::Lightning => "lightning",
        Color::Fighting => "fighting",
        Color::Psychic => "psychic",
        Color::Colorless => "colorless",
    }
}

/// The colour a card is framed in.
pub fn card_color(def: &CardDef) -> Option<Color> {
    match &def.kind {
        CardKind::Creature(c) | CardKind::TrainerBasic(c) => Some(c.color),
        CardKind::Energy(EnergyCard::Basic(c)) => Some(*c),
        CardKind::Energy(EnergyCard::DoubleColorless) => Some(Color::Colorless),
        CardKind::Trainer(_) => None,
    }
}

/// One 24×24 path per colour, filled with `currentColor`.
fn glyph_path(c: Color) -> &'static str {
    match c {
        Color::Grass => "M12 2.5C5 7 4.5 15 12 21.5C19.5 15 19 7 12 2.5ZM12 7.5V18.5L11 18.5V7.5Z",
        Color::Fire => "M12 2C14.5 6.5 19 9 18.5 15A6.5 6.5 0 0 1 5.5 15C5.5 11.5 8 10 8.5 6.5C10.5 8.5 12 6.5 12 2Z",
        Color::Water => "M12 2.5C16.5 9 18.5 12 18.5 15A6.5 6.5 0 0 1 5.5 15C5.5 12 7.5 9 12 2.5Z",
        Color::Lightning => "M13.5 1.5L4.5 13.5H11L9.5 22.5L19.5 9H13Z",
        Color::Fighting => "M12 2.5L20.5 8.5L17.5 20.5H6.5L3.5 8.5ZM12 7L8 10L9.5 16H14.5L16 10Z",
        Color::Psychic => "M1.5 12C6 4.5 18 4.5 22.5 12C18 19.5 6 19.5 1.5 12ZM12 8A4 4 0 1 0 12 16A4 4 0 1 0 12 8Z",
        Color::Colorless => "M12 1.5L14.6 9.4L22.5 12L14.6 14.6L12 22.5L9.4 14.6L1.5 12L9.4 9.4Z",
    }
}

/// A standalone icon for a colour.
pub fn glyph_svg(c: Color) -> String {
    format!(
        r#"<svg viewBox="0 0 24 24" class="glyph" aria-hidden="true"><path fill-rule="evenodd" d="{}"/></svg>"#,
        glyph_path(c)
    )
}

/// Tiny deterministic sequence (splitmix64) seeded from the card name.
struct Seq(u64);

impl Seq {
    fn of(name: &str) -> Seq {
        // FNV-1a, so the art follows the name, not the position in the set.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in name.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
        Seq(h)
    }

    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}

/// The picture window of any card.
pub fn card_art(def: &CardDef) -> String {
    match &def.kind {
        CardKind::Creature(c) | CardKind::TrainerBasic(c) => creature_art(def.name, c),
        CardKind::Energy(e) => energy_art(*e),
        CardKind::Trainer(_) => trainer_art(def.name),
    }
}

fn motifs(s: &mut String, r: &mut Seq, c: Color, count: usize) {
    for _ in 0..count {
        let (x, y) = (r.range(-4.0, 100.0), r.range(-4.0, 66.0));
        let (rot, sc) = (r.range(0.0, 360.0), r.range(0.45, 1.25));
        let _ = write!(
            s,
            r#"<path class="motif" fill-rule="evenodd" transform="translate({x:.1} {y:.1}) rotate({rot:.0}) scale({sc:.2}) translate(-12 -12)" d="{}"/>"#,
            glyph_path(c)
        );
    }
}

/// A seeded blob creature: smooth body, a crest in its colour's glyph, eyes.
fn creature_art(name: &str, c: &Creature) -> String {
    let mut r = Seq::of(name);
    let mut s = String::from(r#"<svg viewBox="0 0 100 70" aria-hidden="true">"#);
    motifs(&mut s, &mut r, c.color, 7);
    let stage = match c.stage {
        Stage::Basic => 0.0,
        Stage::Stage1 => 1.0,
        Stage::Stage2 => 2.0,
    };
    let (cx, cy) = (50.0, 39.0 - stage);
    let base = 13.5 + stage * 3.2;
    let _ = write!(
        s,
        r#"<ellipse class="shadow" cx="50" cy="{:.1}" rx="{:.1}" ry="3.6"/>"#,
        cy + base + 5.0,
        base * 1.25
    );
    if c.stage == Stage::Stage2 {
        let _ = write!(
            s,
            r#"<circle class="aura" cx="{cx}" cy="{cy}" r="{:.1}"/>"#,
            base + 9.0
        );
    }
    // Body: n jittered points, smoothed with quadratic curves through midpoints.
    let n = 6 + (r.next() * 4.0) as usize;
    let spin = r.range(0.0, std::f32::consts::TAU);
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let a = spin + i as f32 / n as f32 * std::f32::consts::TAU + r.range(-0.25, 0.25);
            let rad = base * r.range(0.82, 1.18);
            (cx + a.cos() * rad * 1.15, cy + a.sin() * rad)
        })
        .collect();
    let mid = |a: (f32, f32), b: (f32, f32)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    let start = mid(pts[n - 1], pts[0]);
    let _ = write!(s, r#"<path class="body" d="M{:.1} {:.1}"#, start.0, start.1);
    for i in 0..n {
        let m = mid(pts[i], pts[(i + 1) % n]);
        let _ = write!(s, " Q{:.1} {:.1} {:.1} {:.1}", pts[i].0, pts[i].1, m.0, m.1);
    }
    s.push_str(r#"Z"/>"#);
    let _ = write!(
        s,
        r#"<ellipse class="belly" cx="{cx}" cy="{:.1}" rx="{:.1}" ry="{:.1}"/>"#,
        cy + base * 0.35,
        base * 0.62,
        base * 0.42
    );
    // Crest: the colour glyph sitting on the head, tilted.
    let tilt = r.range(-25.0, 25.0);
    let _ = write!(
        s,
        r#"<path class="crest" fill-rule="evenodd" transform="translate({cx} {:.1}) rotate({tilt:.0}) scale({:.2}) translate(-12 -20)" d="{}"/>"#,
        cy - base * 0.78,
        0.55 + stage * 0.12,
        glyph_path(c.color)
    );
    // Eyes, looking somewhere.
    let gap = r.range(4.5, 8.0) + stage;
    let ey = cy - r.range(0.0, 4.0);
    let (lx, ly) = (r.range(-1.2, 1.2), r.range(-0.8, 1.0));
    for side in [-1.0f32, 1.0] {
        let ex = cx + side * gap;
        let _ = write!(
            s,
            r#"<ellipse class="eye" cx="{ex:.1}" cy="{ey:.1}" rx="3" ry="3.6"/><circle class="pupil" cx="{:.1}" cy="{:.1}" r="1.7"/>"#,
            ex + lx,
            ey + ly
        );
    }
    let my = ey + 6.0;
    let _ = write!(
        s,
        r#"<path class="mouth" d="M{:.1} {my:.1} Q{cx} {:.1} {:.1} {my:.1}"/>"#,
        cx - 3.0,
        my + r.range(1.0, 3.0),
        cx + 3.0
    );
    s.push_str("</svg>");
    s
}

fn energy_art(e: EnergyCard) -> String {
    let c = match e {
        EnergyCard::Basic(c) => c,
        EnergyCard::DoubleColorless => Color::Colorless,
    };
    let mut s = String::from(r#"<svg viewBox="0 0 100 70" aria-hidden="true">"#);
    s.push_str(r#"<circle class="ring" cx="50" cy="35" r="27"/><circle class="ring" cx="50" cy="35" r="19"/>"#);
    let marks: &[f32] = if e == EnergyCard::DoubleColorless {
        &[38.0, 62.0]
    } else {
        &[50.0]
    };
    for &x in marks {
        let _ = write!(
            s,
            r#"<path class="crest" fill-rule="evenodd" transform="translate({x} 35) scale(1.25) translate(-12 -12)" d="{}"/>"#,
            glyph_path(c)
        );
    }
    s.push_str("</svg>");
    s
}

/// Trainers: a seeded arrangement of arcs and bars in slate and brass.
fn trainer_art(name: &str) -> String {
    let mut r = Seq::of(name);
    let mut s = String::from(r#"<svg viewBox="0 0 100 70" aria-hidden="true">"#);
    for i in 0..4 {
        let rad = 10.0 + i as f32 * 8.0 + r.range(-2.0, 2.0);
        let _ = write!(
            s,
            r#"<circle class="arc" cx="{:.1}" cy="{:.1}" r="{rad:.1}" stroke-dasharray="{:.0} {:.0}"/>"#,
            r.range(40.0, 60.0),
            r.range(30.0, 40.0),
            r.range(10.0, 40.0),
            r.range(6.0, 18.0)
        );
    }
    let rot = r.range(-40.0, 40.0);
    let _ = write!(
        s,
        r#"<g transform="rotate({rot:.0} 50 35)"><rect class="bar" x="20" y="32" width="60" height="6" rx="3"/><rect class="bar2" x="44" y="14" width="12" height="42" rx="6"/></g>"#
    );
    s.push_str("</svg>");
    s
}
