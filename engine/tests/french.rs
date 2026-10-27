//! French typography: a no-break space before `:` `;` `?` `!` (U+00A0 before
//! the colon, U+202F before the others), so a wrapped line never starts with
//! the mark alone. A plain space before one, in any player text, fails here:
//! the Rust sources' string literals, the world's `tr(…)` lines, and every
//! map's and quest's `*_fr` text.

use std::fs;
use std::path::Path;

use serde_json::Value;

const MARKS: [char; 4] = [':', ';', '?', '!'];

/// A plain space before a mark that ends a word group (followed by a space,
/// the end of the text, an escaped newline or a closing `»`/`)`).
fn bare(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    (1..c.len()).any(|i| {
        c[i - 1] == ' '
            && MARKS.contains(&c[i])
            && match c.get(i + 1) {
                None => true,
                Some(&n) => {
                    n.is_whitespace()
                        || n == '»'
                        || n == ')'
                        || (n == '\\' && c.get(i + 2) == Some(&'n'))
                }
            }
    })
}

/// The `"…"` (and, in JavaScript, `` `…` ``) literals on one line.
fn literals(line: &str, quotes: &[char]) -> Vec<String> {
    let mut out = vec![];
    let mut it = line.chars().peekable();
    let mut prev = ' ';
    while let Some(ch) = it.next() {
        // A Rust char literal '"' is not a string.
        if quotes.contains(&ch) && !(prev == '\'' && it.peek() == Some(&'\'')) {
            let mut s = String::new();
            let mut closed = false;
            while let Some(c) = it.next() {
                if c == '\\' {
                    s.push(c);
                    if let Some(e) = it.next() {
                        s.push(e);
                    }
                } else if c == ch {
                    closed = true;
                    break;
                } else {
                    s.push(c);
                }
            }
            if closed {
                out.push(s);
            }
            prev = ch;
            continue;
        }
        prev = ch;
    }
    out
}

fn files(dir: &Path, ext: &str) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == ext))
        .collect();
    v.sort();
    v
}

fn fr_texts(v: &Value, key: &str, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => m.iter().for_each(|(k, v)| fr_texts(v, k, out)),
        Value::Array(a) => a.iter().for_each(|v| fr_texts(v, key, out)),
        Value::String(s) if key.ends_with("_fr") => out.push(s.clone()),
        _ => {}
    }
}

#[test]
fn french_marks_keep_their_no_break_space() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut bad = vec![];
    for dir in ["engine/src", "web/src"] {
        for f in files(&root.join(dir), "rs") {
            let src = fs::read_to_string(&f).unwrap();
            for (i, line) in src.lines().enumerate() {
                for s in literals(line, &['"']).into_iter().filter(|s| bare(s)) {
                    bad.push(format!("{}:{}: {s:?}", f.display(), i + 1));
                }
            }
        }
    }
    // The world's scripts: only the lines that hold a `tr(…)` (the rest is code and shaders).
    for f in files(&root.join("web/world"), "js") {
        let src = fs::read_to_string(&f).unwrap();
        for (i, line) in src.lines().enumerate().filter(|(_, l)| l.contains("tr(")) {
            for s in literals(line, &['"', '`']).into_iter().filter(|s| bare(s)) {
                bad.push(format!("{}:{}: {s:?}", f.display(), i + 1));
            }
        }
    }
    let mut json = files(&root.join("web/world/maps"), "json");
    json.push(root.join("web/world/quests.json"));
    for f in json {
        let v: Value = serde_json::from_str(&fs::read_to_string(&f).unwrap()).unwrap();
        let mut texts = vec![];
        fr_texts(&v, "", &mut texts);
        for s in texts.into_iter().filter(|s| bare(s)) {
            bad.push(format!("{}: {s:?}", f.display()));
        }
    }
    assert!(
        bad.is_empty(),
        "a plain space before : ; ? ! (use \\u{{a0}} before ':', \\u{{202f}} before the others):\n{}",
        bad.join("\n")
    );
}

#[test]
fn the_check_sees_a_bare_mark() {
    assert!(bare("Votre main : 3") && bare("Où aller ?") && bare("Victoire !\\n"));
    assert!(
        !bare("Votre main\u{a0}: 3")
            && !bare("Où aller\u{202f}?")
            && !bare("a != b")
            && !bare("12:30")
    );
    assert_eq!(literals(r#"tr("a : b", '"', "c")"#, &['"']), ["a : b", "c"]);
}
