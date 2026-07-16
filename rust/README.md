# smartinfo (Rust port)

A 1:1 port of `../smartinfo.py` producing **byte-identical output** (same
`frequency_<lang>.csv`, `auto_stopwords_<lang>.csv` and
`amiko_frequency_<lang>.db`), roughly 4× faster with about half the memory.

It re-implements the exact NLTK tokenization chain used by the Python
version: the Punkt sentence tokenizer (tokenization-time algorithm, using the
pre-trained English `punkt_tab` tables), `NLTKWordTokenizer` (nltk 3.10) and
`MWETokenizer` — plus lxml-compatible handling of self-closed `<div/>`
elements found in the Fachinfo HTML.

## Build

```
cargo build --release
```

## Run

From the repository root (same data layout as the Python version:
`./dbs/amiko_db_full_idx_<lang>.db`, `./input/*.txt`, writes `./output/`):

```
./rust/target/release/smartinfo --lang=de
./rust/target/release/smartinfo --lang=fr
```

Requires the NLTK `punkt_tab` English tables (also needed by the Python
version), found via `$NLTK_DATA` or `~/nltk_data`
(`tokenizers/punkt_tab/english/`). Install them once with:

```python
import nltk; nltk.download('punkt_tab')
```

## Notes

- Known quirks of smartinfo.py are replicated intentionally for parity, e.g.
  the `"\2"` replacement in `clean_up_string` (Python parses it as the
  control character U+0002, not a backreference, so `1.Drehen`-style tokens
  are dropped instead of being un-prefixed), and the auto-stopword threshold
  counting chapter ids as well as documents via `len(r.split(","))`.
- Like the Python version, all rows are loaded up front; streaming them would
  reduce peak memory further at no cost to the result.
