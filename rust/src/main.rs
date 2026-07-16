//! Rust port of smartinfo.py — extracts keyword frequency indexes from the
//! AmiKo Fachinfo database. CLI-compatible: `smartinfo --lang=de` (or fr).
//! Run from the repository root (expects ./dbs, ./input, writes ./output).
//!
//! Requires the NLTK punkt_tab English tables (same files the Python version
//! uses), found via $NLTK_DATA or ~/nltk_data.

mod cleanup;
mod htmltext;
mod mwe;
mod punkt;
mod treebank;

use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

struct Pipeline {
    punkt: punkt::Punkt,
    mwe: mwe::MweTokenizer,
    stopwords: HashSet<String>,
}

/// nltk.word_tokenize(text): punkt sentence split + NLTKWordTokenizer per sentence
fn word_tokenize(text: &str, p: &Pipeline) -> Vec<String> {
    let mut tokens = Vec::new();
    for (a, b) in p.punkt.sentence_spans(text) {
        if a < b {
            tokens.extend(treebank::tokenize(&text[a..b]));
        }
    }
    tokens
}

/// get_tokens() from smartinfo.py
fn get_tokens(text: &str, p: &Pipeline) -> Vec<String> {
    let tokens = word_tokenize(text, p);
    let tokens = p.mwe.tokenize(tokens);
    cleanup::filter_tokens(tokens, &p.stopwords)
}

/// find_chapters_with_tokens() from smartinfo.py
fn find_chapters_with_tokens(
    sections: &[(String, String)],
    tokens: &HashSet<String>,
    mw_set: &HashSet<String>,
    p: &Pipeline,
) -> HashMap<String, String> {
    let mut word_to_chapter: HashMap<String, String> = HashMap::new();
    for (id, div_text) in sections {
        if div_text.is_empty() {
            continue;
        }
        let div_list: HashSet<String> = get_tokens(div_text, p)
            .into_iter()
            .map(|s| s.replace('_', " "))
            .collect();
        let clean_id = id.replace("section", "").replace("Section", "");
        for w in &div_list {
            if tokens.contains(w) || mw_set.contains(w) {
                word_to_chapter
                    .entry(w.clone())
                    .and_modify(|e| {
                        e.push(',');
                        e.push_str(&clean_id);
                    })
                    .or_insert_with(|| clean_id.clone());
            }
        }
    }
    word_to_chapter
}

fn punkt_tab_dir() -> PathBuf {
    if let Ok(d) = std::env::var("NLTK_DATA") {
        return PathBuf::from(d).join("tokenizers/punkt_tab/english");
    }
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join("nltk_data/tokenizers/punkt_tab/english")
}

fn read_lines(path: &str, trim: fn(&str) -> &str) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {}", path, e))
        .lines()
        .map(|l| trim(l).to_string())
        .collect()
}

fn main() {
    // Language flag (getopt-style: --lang=de, --lang de, -l de, -h)
    let mut lang = "de".to_string();
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Hidden debug modes for stage-by-stage parity validation against Python
    if args.len() == 2 && args[0] == "--text-from-html" {
        let html = fs::read_to_string(&args[1]).unwrap();
        print!("{}", htmltext::extract(&html).full_text);
        return;
    }
    if args.len() == 2 && args[0] == "--tokens-from-text" {
        let text = fs::read_to_string(&args[1]).unwrap();
        let p = Pipeline {
            punkt: punkt::Punkt::load(&punkt_tab_dir()),
            mwe: mwe::MweTokenizer::default(),
            stopwords: HashSet::new(),
        };
        let mut out = BufWriter::new(std::io::stdout());
        for t in word_tokenize(&text, &p) {
            writeln!(out, "{}", t).unwrap();
        }
        return;
    }

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-h" {
            println!("smartinfo --lang <language>");
            std::process::exit(2);
        } else if let Some(v) = a.strip_prefix("--lang=") {
            lang = v.to_string();
        } else if a == "--lang" || a == "-l" {
            i += 1;
            lang = args.get(i).cloned().unwrap_or_else(|| {
                println!("smartinfo --lang <language>");
                std::process::exit(2);
            });
        }
        i += 1;
    }
    let lang = lang.as_str();
    if lang != "de" && lang != "fr" {
        eprintln!("unsupported --lang: {} (use de or fr)", lang);
        std::process::exit(2);
    }

    if !std::path::Path::new("./output").exists() {
        fs::create_dir_all("./output").expect("cannot create ./output");
    }

    // Open connection to database for reading. Rows are streamed one at a
    // time (unlike the Python version's fetchall) — only the current row's
    // regnrs/content are held in memory.
    let db_path = format!("./dbs/amiko_db_full_idx_{}.db", lang);
    let con = Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap_or_else(|e| panic!("cannot open {}: {}", db_path, e));
    let n: usize = con
        .query_row("SELECT COUNT(*) FROM amikodb", [], |r| r.get(0))
        .unwrap();

    // Input word lists (BOM and empty lines preserved, like the Python version)
    let stop_words = read_lines(&format!("./input/stopwords_{}.txt", lang), str::trim);
    let white_words_list = read_lines(&format!("./input/whitelist_{}.txt", lang), str::trim);
    let multi_words = read_lines(&format!("./input/multiwords_{}.txt", lang), str::trim_end);

    let mut mwe_tok = mwe::MweTokenizer::default();
    for mw in &multi_words {
        let parts: Vec<String> = mw.trim().split(' ').map(str::to_string).collect();
        mwe_tok.add_mwe(&parts);
    }
    let mw_set: HashSet<String> = multi_words.iter().map(|m| m.trim().to_string()).collect();
    let multi_words_set: HashSet<String> = multi_words.iter().cloned().collect();
    let white_words: HashSet<String> = white_words_list.iter().cloned().collect();

    let pipeline = Pipeline {
        punkt: punkt::Punkt::load(&punkt_tab_dir()),
        mwe: mwe_tok,
        stopwords: stop_words.into_iter().collect(),
    };

    // Output files
    let mut freq_wr =
        BufWriter::new(fs::File::create(format!("./output/frequency_{}.csv", lang)).unwrap());
    let mut auto_stop_wr = BufWriter::new(
        fs::File::create(format!("./output/auto_stopwords_{}.csv", lang)).unwrap(),
    );

    // Output database
    let out_con = Connection::open(format!("./output/amiko_frequency_{}.db", lang)).unwrap();
    out_con
        .execute_batch(
            "DROP TABLE IF EXISTS frequency;
             CREATE TABLE frequency (id TEXT PRIMARY_KEY, keyword TEXT, regnr TEXT);",
        )
        .unwrap();

    // Start big loop
    let start = Instant::now();
    let mut word_dict: BTreeMap<String, String> = BTreeMap::new();

    let mut stmt = con.prepare("SELECT regnrs, content FROM amikodb").unwrap();
    let mut db_rows = stmt.query([]).unwrap();
    let mut i: usize = 0;
    while let Some(row) = db_rows.next().unwrap() {
        let i_cur = i;
        i += 1;
        let regnrs: Option<String> = row.get(0).unwrap();
        let content: Option<String> = row.get(1).unwrap();
        let regnr = match &regnrs {
            Some(r) if !r.is_empty() => r.split(',').next().unwrap_or(""),
            _ => "",
        };
        if regnr.is_empty() {
            continue;
        }
        if i_cur % 31 == 0 {
            print!("\r {} % ", (1000.0 * i_cur as f64 / n as f64).round() / 10.0);
            std::io::stdout().flush().ok();
        }
        let Some(html) = &content else { continue };
        let doc = htmltext::extract(html);
        if doc.full_text.is_empty() {
            continue;
        }
        let tokens = get_tokens(&doc.full_text, &pipeline);
        let cleaned: Vec<String> = tokens
            .iter()
            .map(|t| cleanup::clean_up_string(lang, t, &white_words))
            .filter(|s| !s.is_empty())
            .collect();
        let uniq: HashSet<String> = cleaned.into_iter().collect();

        let w_to_c = find_chapters_with_tokens(&doc.sections, &uniq, &mw_set, &pipeline);

        for w in &uniq {
            let ch_ids = match w_to_c.get(w) {
                Some(c) => format!("({})", c),
                None => String::new(),
            };
            let regnr_prime = format!("{}{}", regnr, ch_ids);
            word_dict
                .entry(w.clone())
                .and_modify(|e| {
                    e.push('|');
                    e.push_str(&regnr_prime);
                })
                .or_insert(regnr_prime);
        }
    }

    println!("\n============================================================\n");
    let mut cnt: u64 = 0;
    out_con.execute_batch("BEGIN").unwrap();
    {
        let mut ins = out_con
            .prepare("INSERT INTO frequency VALUES(?1, ?2, ?3)")
            .unwrap();
        for (k, r) in &word_dict {
            // Change this number to increase or decrease the number of
            // auto-generated stopwords
            let word_count = r.split(',').count();

            if !white_words.contains(k) && !multi_words_set.contains(k) && word_count > 600 {
                write!(auto_stop_wr, "{};{}\r\n", k, word_count).unwrap();
            } else {
                cnt += 1;
                write!(freq_wr, "{};{}\r\n", k, r).unwrap();
                if cnt % 100 == 0 {
                    print!("\rSaved: {}", cnt);
                    std::io::stdout().flush().ok();
                    out_con.execute_batch("COMMIT; BEGIN").unwrap();
                }
                // Generate 16 byte hash (first 10 hex chars of sha256)
                let digest = Sha256::digest(k.as_bytes());
                let hash: String = digest.iter().take(5).map(|b| format!("{:02x}", b)).collect();
                ins.execute(rusqlite::params![hash, k, r]).unwrap();
            }
        }
    }
    out_con.execute_batch("COMMIT").unwrap();

    freq_wr.flush().unwrap();
    auto_stop_wr.flush().unwrap();

    println!("\n\n============================================================\n");
    println!("Elapsed time = {:.3}s", start.elapsed().as_secs_f64());
}
