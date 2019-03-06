# fachinfo_ai
Doing NLTK and AI on Swiss Fachinfos with Python. Parsing all the important words from all FIs in Switzerland.
#### Requirements:
* List of stopwords in folder input (filename: stopwords.txt)
* Amiko sqlite DB in folder dbs (filename: amiko_db_full_idx_de.db)

#### Setup:
* Create `dbs` dir and put the files `amiko_db_full_idx_de.db` and `amiko_db_full_idx_fr.db` generated with [cpp2sqlite](https://github.com/zdavatz/cpp2sqlite) there.
* From `$SRC_DIR` run with `/usr/local/bin/python3 smartinfo.py --lang=de`

#### Output:
* Frequency csv file in folder output (filename: frequency.csv)
* Auto-generated stopwords file in folder output (filename: auto_stopwords.csv)

#### Requirements
* pip install nltk
* python 3.5.2
* nltk.download('stopwords','punkt')

#### sqlite Database to download under the GPLv3.0 License
* http://pillbox.oddb.org/amiko_frequency.db
