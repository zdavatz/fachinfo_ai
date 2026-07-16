//! Port of nltk.tokenize.destructive.NLTKWordTokenizer (nltk 3.10), the word
//! tokenizer invoked by `nltk.word_tokenize` on each sentence.

use std::sync::LazyLock;

struct Rules {
    // STARTING_QUOTES
    sq0: regex::Regex,
    sq1: regex::Regex,
    sq2: regex::Regex,
    sq3: regex::Regex,
    sq4: fancy_regex::Regex, // clitic rule with negative lookahead
    // PUNCTUATION
    p0: regex::Regex,
    p1: regex::Regex,
    p2: regex::Regex,
    p3: regex::Regex,
    p4: regex::Regex,
    p5: regex::Regex,
    p6: regex::Regex,
    p7: regex::Regex,
    p8: regex::Regex,
    p9: regex::Regex,
    parens: regex::Regex,
    dashes: regex::Regex,
    // ENDING_QUOTES
    e0: regex::Regex,
    e1: regex::Regex,
    e2: regex::Regex,
    e3: regex::Regex,
    e4: regex::Regex,
    e5: regex::Regex,
    // CONTRACTIONS2 / CONTRACTIONS3
    c2: Vec<regex::Regex>,
    c2_wanna: fancy_regex::Regex, // (?i)\b(wan)(na)(?=\s)
    c3: Vec<regex::Regex>,
}

static RULES: LazyLock<Rules> = LazyLock::new(|| Rules {
    sq0: regex::Regex::new("([«“‘„]|[`]+)").unwrap(),
    sq1: regex::Regex::new(r#"^""#).unwrap(),
    sq2: regex::Regex::new(r"(``)").unwrap(),
    sq3: regex::Regex::new(r#"([ \(\[{<])("|'{2})"#).unwrap(),
    // (?i)(\')(?!re|ve|ll|m|t|s|d|n)(\w)\b — Python's \w/\b exclude combining
    // marks (unlike Rust's \w which includes \p{M}), so Python's word class
    // [letter|number|_] is spelled out and \b after the single word char is
    // expressed as a negative lookahead.
    sq4: fancy_regex::Regex::new(
        r"(')(?!(?i:re|ve|ll|m|t|s|d|n))([\p{L}\p{N}_])(?![\p{L}\p{N}_])",
    )
    .unwrap(),
    p0: regex::Regex::new(r#"([^\.])(\.)([\]\)}>"'»”’ ]*)\s*$"#).unwrap(),
    p1: regex::Regex::new(r"([:,])([^\d])").unwrap(),
    p2: regex::Regex::new(r"([:,])$").unwrap(),
    p3: regex::Regex::new(r"\.{2,}").unwrap(),
    p4: regex::Regex::new(r"[;@#$%&]").unwrap(),
    p5: regex::Regex::new(r"[\x{2012}-\x{2015}]").unwrap(),
    p6: regex::Regex::new(r#"([^\.])(\.)([\]\)}>"']*)\s*$"#).unwrap(),
    p7: regex::Regex::new(r"[?!]").unwrap(),
    p8: regex::Regex::new(r"([^'])' ").unwrap(),
    p9: regex::Regex::new(r"[*]").unwrap(),
    parens: regex::Regex::new(r"[\]\[\(\)\{\}<>]").unwrap(),
    dashes: regex::Regex::new(r"--").unwrap(),
    e0: regex::Regex::new("([»”’])").unwrap(),
    e1: regex::Regex::new(r"''").unwrap(),
    e2: regex::Regex::new(r#"""#).unwrap(),
    e3: regex::Regex::new(r"\s+").unwrap(),
    e4: regex::Regex::new(r"([^' ])('[sS]|'[mM]|'[dD]|') ").unwrap(),
    e5: regex::Regex::new(r"([^' ])('ll|'LL|'re|'RE|'ve|'VE|n't|N'T) ").unwrap(),
    c2: vec![
        regex::Regex::new(r"(?i)\b(can)(not)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(d)('ye)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(gim)(me)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(gon)(na)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(got)(ta)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(lem)(me)\b").unwrap(),
        regex::Regex::new(r"(?i)\b(more)('n)\b").unwrap(),
    ],
    c2_wanna: fancy_regex::Regex::new(r"(?i)\b(wan)(na)(?=\s)").unwrap(),
    c3: vec![
        regex::Regex::new(r"(?i) ('t)(is)\b").unwrap(),
        regex::Regex::new(r"(?i) ('t)(was)\b").unwrap(),
    ],
});

/// NLTKWordTokenizer().tokenize(sentence)
pub fn tokenize(text: &str) -> Vec<String> {
    let r = &*RULES;
    let mut t: String;

    // STARTING_QUOTES
    t = r.sq0.replace_all(text, " $1 ").into_owned();
    t = r.sq1.replace_all(&t, "``").into_owned();
    t = r.sq2.replace_all(&t, " $1 ").into_owned();
    t = r.sq3.replace_all(&t, "$1 `` ").into_owned();
    t = r.sq4.replace_all(&t, "$1 $2").into_owned();

    // PUNCTUATION
    t = r.p0.replace_all(&t, "$1 $2 $3 ").into_owned();
    t = r.p1.replace_all(&t, " $1 $2").into_owned();
    t = r.p2.replace_all(&t, " $1 ").into_owned();
    t = r.p3.replace_all(&t, " ${0} ").into_owned();
    t = r.p4.replace_all(&t, " ${0} ").into_owned();
    t = r.p5.replace_all(&t, " ${0} ").into_owned();
    t = r.p6.replace_all(&t, "$1 ${2}${3} ").into_owned();
    t = r.p7.replace_all(&t, " ${0} ").into_owned();
    t = r.p8.replace_all(&t, "$1 ' ").into_owned();
    t = r.p9.replace_all(&t, " ${0} ").into_owned();

    // PARENS_BRACKETS, DOUBLE_DASHES
    t = r.parens.replace_all(&t, " ${0} ").into_owned();
    t = r.dashes.replace_all(&t, " -- ").into_owned();

    // add extra space to make things easier
    t = format!(" {} ", t);

    // ENDING_QUOTES
    t = r.e0.replace_all(&t, " $1 ").into_owned();
    t = r.e1.replace_all(&t, " '' ").into_owned();
    t = r.e2.replace_all(&t, " '' ").into_owned();
    t = r.e3.replace_all(&t, " ").into_owned();
    t = r.e4.replace_all(&t, "$1 $2 ").into_owned();
    t = r.e5.replace_all(&t, "$1 $2 ").into_owned();

    // CONTRACTIONS2 / CONTRACTIONS3
    for re in &r.c2 {
        t = re.replace_all(&t, " $1 $2 ").into_owned();
    }
    t = r.c2_wanna.replace_all(&t, " $1 $2 ").into_owned();
    for re in &r.c3 {
        t = re.replace_all(&t, " $1 $2 ").into_owned();
    }

    t.split_whitespace().map(str::to_string).collect()
}
