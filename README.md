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
* [python 3.5.2](https://www.python.org/ftp/python/3.5.2/Python-3.5.2.tgz)
* nltk.download('stopwords','punkt')

####For Mac
* install [zlib](https://www.zlib.net/zlib-1.2.11.tar.gz) from source
* configure python with `./configure --with-zlib-dir=/usr/local`
* grab [git-pip.py](https://bootstrap.pypa.io/get-pip.py)
* do `/usr/local/bin/python3.5 get-pip.py`

#### sqlite Database to download under the GPLv3.0 License
* http://pillbox.oddb.org/amiko_frequency.db
