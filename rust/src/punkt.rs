//! Port of NLTK's PunktSentenceTokenizer (nltk/tokenize/punkt.py) restricted
//! to tokenization time (no training), loading the pre-trained English tables
//! from punkt_tab (abbrev_types.txt, collocations.tab, ortho_context.tab,
//! sent_starters.txt) so that sentence-boundary decisions match
//! `nltk.sent_tokenize(text, language="english")`.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

const ORTHO_BEG_UC: u32 = 1 << 1;
const ORTHO_MID_UC: u32 = 1 << 2;
const ORTHO_UNK_UC: u32 = 1 << 3;
const ORTHO_BEG_LC: u32 = 1 << 4;
const ORTHO_MID_LC: u32 = 1 << 5;
const ORTHO_UNK_LC: u32 = 1 << 6;
const ORTHO_UC: u32 = ORTHO_BEG_UC + ORTHO_MID_UC + ORTHO_UNK_UC;
const ORTHO_LC: u32 = ORTHO_BEG_LC + ORTHO_MID_LC + ORTHO_UNK_LC;

// _RE_NUMERIC = ^-?[\.,]?\d[\d,\.-]*\.?$
static RE_NUMERIC: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^-?[\.,]?\d[\d,\.-]*\.?$").unwrap());

// Punkt word tokenizer (PunktLanguageVars._word_tokenize_fmt with the default
// sent_end_chars ('.', '!', '?')). Uses lookaheads, hence fancy-regex.
static RE_WORD_TOKENIZE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(
        r#"(?:\-{2,}|\.{2,}|(?:\.\s){2,}\.)|(?=[^\(\"\`\{\[:;&\#\*@\)\}\]\-,])\S+?(?=\s|$|(?:[\)\";\}\]\*:@\'\(\{\[!\?])|(?:\-{2,}|\.{2,}|(?:\.\s){2,}\.)|,(?=$|\s|(?:[\)\";\}\]\*:@\'\(\{\[!\?])|(?:\-{2,}|\.{2,}|(?:\.\s){2,}\.)))|\S"#,
    )
    .unwrap()
});

// re_boundary_realignment = re.compile(r'["\')\]}]+?(?:\s+|(?=--)|$)', re.MULTILINE)
// used with .match() (anchored at the start), hence \A.
static RE_REALIGN: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(r#"(?m)\A["'\)\]\}]+?(?:\s+|(?=--)|$)"#).unwrap()
});

// _re_non_word_chars for default sent_end_chars: (?:[)\";}\]\*:@\'\({\[!?])
fn is_non_word_char(c: char) -> bool {
    matches!(
        c,
        ')' | '"' | ';' | '}' | ']' | '*' | ':' | '@' | '\'' | '(' | '{' | '[' | '!' | '?'
    )
}

// string.whitespace (ASCII), used by _get_last_whitespace_index
fn is_ascii_py_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c')
}

/// One token as annotated by punkt (PunktToken).
struct Tok {
    tok: String,
    typ: String,
    period_final: bool,
    abbr: bool,
    ellipsis: bool,
    sentbreak: bool,
}

impl Tok {
    fn new(tok: &str) -> Tok {
        let lower = tok.to_lowercase();
        let typ = if RE_NUMERIC.is_match(&lower) {
            "##number##".to_string()
        } else {
            lower
        };
        Tok {
            period_final: tok.ends_with('.'),
            tok: tok.to_string(),
            typ,
            abbr: false,
            ellipsis: false,
            sentbreak: false,
        }
    }

    fn type_no_period(&self) -> &str {
        if self.typ.len() > 1 && self.typ.ends_with('.') {
            &self.typ[..self.typ.len() - 1]
        } else {
            &self.typ
        }
    }

    fn type_no_sentperiod(&self) -> &str {
        if self.sentbreak {
            self.type_no_period()
        } else {
            &self.typ
        }
    }

    fn first_upper(&self) -> bool {
        self.tok.chars().next().is_some_and(|c| c.is_uppercase())
    }

    fn first_lower(&self) -> bool {
        self.tok.chars().next().is_some_and(|c| c.is_lowercase())
    }

    // _RE_ELLIPSIS = re.match(r"\.\.+$") => whole token is 2+ periods
    fn is_ellipsis(&self) -> bool {
        self.tok.len() >= 2 && self.tok.bytes().all(|b| b == b'.')
    }

    // _RE_INITIAL = re.match(r"[^\W\d]\.$") => single word-char (non-digit) + "."
    fn is_initial(&self) -> bool {
        let mut it = self.tok.chars();
        match (it.next(), it.next(), it.next()) {
            (Some(a), Some('.'), None) => (a.is_alphabetic() || a == '_') && !a.is_numeric(),
            _ => false,
        }
    }
}

struct PMatch {
    start: usize,          // byte offset of the [.!?] char
    end: usize,            // start + 1
    after_end: usize,      // byte end of the after_tok group
    next_tok_start: Option<usize>, // byte start of next_tok, if the \s+\S+ branch matched
}

pub struct Punkt {
    abbrev_types: HashSet<String>,
    collocations: HashSet<(String, String)>,
    sent_starters: HashSet<String>,
    ortho_context: HashMap<String, u32>,
}

impl Punkt {
    /// Load the punkt_tab tables for a language (e.g. .../tokenizers/punkt_tab/english).
    pub fn load(dir: &Path) -> Punkt {
        let read = |name: &str| -> String {
            fs::read_to_string(dir.join(name))
                .unwrap_or_else(|e| panic!("cannot read {}: {}", dir.join(name).display(), e))
        };
        let abbrev_types: HashSet<String> =
            read("abbrev_types.txt").lines().map(str::to_string).collect();
        let sent_starters: HashSet<String> =
            read("sent_starters.txt").lines().map(str::to_string).collect();
        let collocations: HashSet<(String, String)> = read("collocations.tab")
            .lines()
            .map(|l| {
                let mut it = l.splitn(2, '\t');
                (
                    it.next().unwrap_or("").to_string(),
                    it.next().unwrap_or("").to_string(),
                )
            })
            .collect();
        let ortho_context: HashMap<String, u32> = read("ortho_context.tab")
            .lines()
            .map(|l| {
                let mut it = l.splitn(2, '\t');
                let k = it.next().unwrap_or("").to_string();
                let v = it.next().unwrap_or("0").parse::<u32>().unwrap_or(0);
                (k, v)
            })
            .collect();
        Punkt {
            abbrev_types,
            collocations,
            sent_starters,
            ortho_context,
        }
    }

    fn ortho(&self, typ: &str) -> u32 {
        *self.ortho_context.get(typ).unwrap_or(&0)
    }

    // _first_pass_annotation
    fn first_pass(&self, t: &mut Tok) {
        if t.tok == "." || t.tok == "!" || t.tok == "?" {
            t.sentbreak = true;
        } else if t.is_ellipsis() {
            t.ellipsis = true;
        } else if t.period_final && !t.tok.ends_with("..") {
            let base = t.tok[..t.tok.len() - 1].to_lowercase();
            let last_dash = base.rsplit('-').next().unwrap_or("");
            if self.abbrev_types.contains(&base) || self.abbrev_types.contains(last_dash) {
                t.abbr = true;
            } else {
                t.sentbreak = true;
            }
        }
    }

    // _ortho_heuristic: Some(true) / Some(false) / None (= "unknown")
    fn ortho_heuristic(&self, t: &Tok) -> Option<bool> {
        if matches!(t.tok.as_str(), ";" | ":" | "," | "." | "!" | "?") {
            return Some(false);
        }
        let ctx = self.ortho(t.type_no_sentperiod());
        if t.first_upper() && ctx & ORTHO_LC != 0 && ctx & ORTHO_MID_UC == 0 {
            return Some(true);
        }
        if t.first_lower() && (ctx & ORTHO_UC != 0 || ctx & ORTHO_BEG_LC == 0) {
            return Some(false);
        }
        None
    }

    // _second_pass_annotation over the pair (toks[i], toks[i+1])
    fn second_pass(&self, toks: &mut [Tok], i: usize) {
        if i + 1 >= toks.len() {
            return;
        }
        if !toks[i].period_final {
            return;
        }
        let typ = toks[i].type_no_period().to_string();
        let next_typ = toks[i + 1].type_no_sentperiod().to_string();
        let tok_is_initial = toks[i].is_initial();

        // [4.1.2. Collocation Heuristic]
        if self.collocations.contains(&(typ.clone(), next_typ.clone())) {
            toks[i].sentbreak = false;
            toks[i].abbr = true;
            return;
        }

        // [4.2. Token-Based Reclassification of Abbreviations]
        if (toks[i].abbr || toks[i].ellipsis) && !tok_is_initial {
            match self.ortho_heuristic(&toks[i + 1]) {
                Some(true) => {
                    toks[i].sentbreak = true;
                    return;
                }
                _ => {}
            }
            if toks[i + 1].first_upper() && self.sent_starters.contains(&next_typ) {
                toks[i].sentbreak = true;
                return;
            }
        }

        // [4.3. Token-Based Detection of Initials and Ordinals]
        if tok_is_initial || typ == "##number##" {
            match self.ortho_heuristic(&toks[i + 1]) {
                Some(false) => {
                    toks[i].sentbreak = false;
                    toks[i].abbr = true;
                }
                None => {
                    if tok_is_initial
                        && toks[i + 1].first_upper()
                        && self.ortho(&next_typ) & ORTHO_LC == 0
                    {
                        toks[i].sentbreak = false;
                        toks[i].abbr = true;
                    }
                }
                _ => {}
            }
        }
    }

    // _tokenize_words: split on '\n', skip blank lines, punkt word regex per line
    fn tokenize_words(&self, text: &str) -> Vec<Tok> {
        let mut out = Vec::new();
        for line in text.split('\n') {
            if line.trim().is_empty() {
                continue;
            }
            for m in RE_WORD_TOKENIZE.find_iter(line) {
                out.push(Tok::new(m.unwrap().as_str()));
            }
        }
        out
    }

    // text_contains_sentbreak: any non-final token annotated as sentbreak
    fn text_contains_sentbreak(&self, context: &str) -> bool {
        let mut toks = self.tokenize_words(context);
        for t in toks.iter_mut() {
            self.first_pass(t);
        }
        for i in 0..toks.len() {
            self.second_pass(&mut toks, i);
        }
        let n = toks.len();
        for (i, t) in toks.iter().enumerate() {
            if t.sentbreak && i + 1 < n {
                return true;
            }
        }
        false
    }

    // _match_potential_end_contexts (returns matches with their context strings)
    fn match_potential_end_contexts<'a>(&self, text: &'a str) -> Vec<(PMatch, String)> {
        // Find all period-context matches first (the finditer equivalent).
        let mut matches: Vec<PMatch> = Vec::new();
        for (i, ch) in text.char_indices() {
            if ch != '.' && ch != '!' && ch != '?' {
                continue;
            }
            let rest = &text[i + 1..];
            let mut chars = rest.chars();
            match chars.next() {
                None => {} // lookahead needs an after_tok
                Some(c) if is_non_word_char(c) => {
                    matches.push(PMatch {
                        start: i,
                        end: i + 1,
                        after_end: i + 1 + c.len_utf8(),
                        next_tok_start: None,
                    });
                }
                Some(c) if c.is_whitespace() => {
                    // \s+ then \S+ (at least one non-space char required)
                    let mut j = i + 1;
                    for c2 in text[i + 1..].chars() {
                        if c2.is_whitespace() {
                            j += c2.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let mut k = j;
                    for c2 in text[j..].chars() {
                        if !c2.is_whitespace() {
                            k += c2.len_utf8();
                        } else {
                            break;
                        }
                    }
                    if k > j {
                        matches.push(PMatch {
                            start: i,
                            end: i + 1,
                            after_end: k,
                            next_tok_start: Some(j),
                        });
                    }
                }
                _ => {}
            }
        }

        // Overlap-suppression logic from _match_potential_end_contexts.
        let mut out: Vec<(PMatch, String)> = Vec::new();
        let mut prev_slice = (0usize, 0usize);
        let mut prev_match: Option<PMatch> = None;
        for m in matches {
            let before_text = &text.as_bytes()[prev_slice.1..m.start];
            // index of last ASCII whitespace byte, 0 if none (Python returns 0 too)
            let mut last_ws = 0usize;
            for (bi, &b) in before_text.iter().enumerate().rev() {
                if is_ascii_py_whitespace(b) {
                    last_ws = bi;
                    break;
                }
            }
            let index_after_last_space = if last_ws != 0 {
                prev_slice.1 + last_ws + 1
            } else {
                prev_slice.0
            };
            let word_slice = (index_after_last_space, m.start);

            if let Some(pm) = prev_match.take() {
                if prev_slice.1 <= word_slice.0 {
                    let ctx = format!(
                        "{}{}{}",
                        &text[prev_slice.0..prev_slice.1],
                        &text[pm.start..pm.end],
                        &text[pm.end..pm.after_end]
                    );
                    out.push((pm, ctx));
                } else {
                    // overlapping: drop the previous match (as Python does)
                }
            }
            prev_match = Some(m);
            prev_slice = word_slice;
        }
        if let Some(pm) = prev_match {
            let ctx = format!(
                "{}{}{}",
                &text[prev_slice.0..prev_slice.1],
                &text[pm.start..pm.end],
                &text[pm.end..pm.after_end]
            );
            out.push((pm, ctx));
        }
        out
    }

    // _slices_from_text
    fn slices_from_text(&self, text: &str) -> Vec<(usize, usize)> {
        let mut slices = Vec::new();
        let mut last_break = 0usize;
        for (m, ctx) in self.match_potential_end_contexts(text) {
            if self.text_contains_sentbreak(&ctx) {
                slices.push((last_break, m.end));
                last_break = match m.next_tok_start {
                    Some(s) => s,
                    None => m.end,
                };
            }
        }
        slices.push((last_break, text.trim_end().len()));
        slices
    }

    // _realign_boundaries
    fn realign_boundaries(&self, text: &str, slices: &[(usize, usize)]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut realign = 0usize;
        for i in 0..slices.len() {
            let s1 = (slices[i].0 + realign, slices[i].1);
            let s2 = slices.get(i + 1);
            match s2 {
                None => {
                    if s1.0 < s1.1 {
                        out.push(s1);
                    }
                }
                Some(&(s2s, s2e)) => {
                    let sent2 = if s2s < s2e { &text[s2s..s2e] } else { "" };
                    match RE_REALIGN.find(sent2) {
                        Ok(Some(m)) if m.start() == 0 => {
                            out.push((s1.0, s2s + m.as_str().trim_end().len()));
                            realign = m.end();
                        }
                        _ => {
                            realign = 0;
                            if s1.0 < s1.1 {
                                out.push(s1);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// sentences_from_text(text, realign_boundaries=True): byte spans of sentences.
    pub fn sentence_spans(&self, text: &str) -> Vec<(usize, usize)> {
        let slices = self.slices_from_text(text);
        self.realign_boundaries(text, &slices)
    }
}
