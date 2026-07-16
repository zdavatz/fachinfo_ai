//! Port of smartinfo.py's clean_up_string() and the token filters from
//! get_tokens(). The regexes (and their quirks) are replicated 1:1; note in
//! particular that Python's `re.sub(r"^([0-9]+.|-|−)(...)$", "\2", s)` has a
//! replacement of "\2" in a *non-raw* string, i.e. the control character
//! U+0002 (not a backreference) — replicated here for output parity.

use std::collections::HashSet;
use std::sync::LazyLock;

// string.punctuation; Python's `w not in string.punctuation` is a substring test.
const PUNCT_STR: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

const BAD_CHARS: &[char] = &[
    '\\', '`', '♠', '↔', '↓', '↑', '«', '»', '„', '“', '”', '®', '\'', '¹', '³', '’', '§', '‘',
    '≡', '✶', '•', '≙', '≤', '≥', ',', '·', '†', '‡', '‹', '›', 'ˆ', '¶',
];

struct CleanRules {
    de_fach: regex::Regex,
    num_dot_word: regex::Regex,
    time_hms: regex::Regex,
    comma_in_num: regex::Regex,
    apos_in_num: regex::Regex,
    leading_decimal: regex::Regex,
    corpses: regex::Regex,
    leading_punct_word: regex::Regex,
    n_eq: regex::Regex,
    three_chars_num: regex::Regex,
    num_stars: regex::Regex,
}

static R: LazyLock<CleanRules> = LazyLock::new(|| CleanRules {
    de_fach: regex::Regex::new(
        r"[-]?[0-9]?(Fach(en|es|e)|fac(h|he|hen)|stündig(e|en)|wöchige(r|n)|monatig(e|en)|jährig(e|en))",
    )
    .unwrap(),
    num_dot_word: regex::Regex::new(r"^([0-9]+.|-|−)([a-zA-ZäöüèéàÜÖÄ]+)$").unwrap(),
    time_hms: regex::Regex::new(r"^[0-9]{2}:[0-9]{2}:[0-9]{2}$").unwrap(),
    comma_in_num: regex::Regex::new(r"([+-]?[0-9]+),([0-9]+)").unwrap(),
    apos_in_num: regex::Regex::new(r"([+-]?[0-9]+)'([0-9]+)").unwrap(),
    leading_decimal: regex::Regex::new(r"^[+-]?[0-9]+\.[0-9]+?").unwrap(),
    corpses: regex::Regex::new(r"^[-|–]?(.)?[0-9]+").unwrap(),
    leading_punct_word: regex::Regex::new(r"^[-–./*+,](\D+)$").unwrap(),
    n_eq: regex::Regex::new(r"(\*|-[0-9]+|[0-9]+)n=[0-9]+").unwrap(),
    three_chars_num: regex::Regex::new(r"^...[0-9]+$|^-[0-9]+").unwrap(),
    num_stars: regex::Regex::new(r"^[+-−.]?[0-9]+\*+$").unwrap(),
});

/// clean_up_string(lang, s, white_words)
pub fn clean_up_string(lang: &str, s: &str, white_words: &HashSet<String>) -> String {
    let mut s: String = if s.contains(BAD_CHARS) {
        s.chars().filter(|c| !BAD_CHARS.contains(c)).collect()
    } else {
        s.to_string()
    };

    if lang == "de" {
        // Remove all -fachen, -faches, etc
        s = R.de_fach.replace_all(&s, "").into_owned();
        // Remove special strings
        s = s.replace("o.ä.", "").replace("z.B.", "").replace("’’", "");
    } else if lang == "fr" {
        s = R.de_fach.replace_all(&s, "").into_owned();
    }

    // Remove numbers and dots before letters — NB: Python's replacement "\2"
    // is the control character U+0002, so matching tokens effectively die.
    s = R.num_dot_word.replace_all(&s, "\u{0002}").into_owned();
    // Remove time with format hh:mm:ss
    s = R.time_hms.replace_all(&s, "").into_owned();
    // Replace , in numbers, e.g. 0,001 -> 0.001
    s = R.comma_in_num.replace_all(&s, "${1}.${2}").into_owned();
    // Replace ' in numbers, e.g. 10'000 -> 10000
    s = R.apos_in_num.replace_all(&s, "${1}${2}").into_owned();
    // Remove all numbers
    s = R.leading_decimal.replace_all(&s, "").into_owned();
    // Remove all corpses from previous operation (exclude E-numbers, G6PD, whitelist)
    if !s.starts_with('E') && !s.starts_with('G') && !white_words.contains(&s) {
        s = R.corpses.replace_all(&s, "").into_owned();
    }
    // Replace all alpha only strings which start with '-'
    s = R.leading_punct_word.replace_all(&s, "$1").into_owned();
    // Remove all n=46
    s = R.n_eq.replace_all(&s, "").into_owned();
    // Remove all ...xxx kind of strings (exclude E-numbers, e.g. E218)
    if !s.starts_with('E') {
        s = R.three_chars_num.replace_all(&s, "").into_owned();
    }

    // Remove all strings that start with / or start with ,
    if s.starts_with('/') {
        s = String::new();
    }
    if let Some(rest) = s.strip_prefix('‚') {
        s = rest.to_string();
    }

    // Remove all strings with this format (+/-)60**
    s = R.num_stars.replace_all(&s, "").into_owned();

    // Remove underscores _ from multi words tokenized text
    s = s.replace('_', " ");
    // Remove all strings that are smaller than 3 chars
    if s.chars().count() <= 3 && !white_words.contains(&s) {
        s = String::new();
    }
    s
}

/// is_integer() from smartinfo.py (str.isdigit approximated as ASCII digits)
pub fn is_integer(s: &str) -> bool {
    let all_digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    all_digits(s) || (s.starts_with('-') && all_digits(&s[1..]))
}

/// The post-tokenization filters from get_tokens()
pub fn filter_tokens(tokens: Vec<String>, stopwords: &HashSet<String>) -> Vec<String> {
    tokens
        .into_iter()
        .map(|w| {
            // Handle special case with punctuation (issue #15)
            if w == "c.521T_>_C" {
                "c.521T>C".to_string()
            } else {
                w
            }
        })
        .filter(|w| !stopwords.contains(w) && !stopwords.contains(&w.to_lowercase()))
        .filter(|w| !PUNCT_STR.contains(w.as_str()))
        .filter(|w| !is_integer(w))
        .filter(|w| w.chars().count() >= 3)
        .collect()
}
