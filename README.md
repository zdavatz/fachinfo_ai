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

#### Requirements for Linux
* pip install nltk
* [python 3.5.2](https://www.python.org/ftp/python/3.5.2/Python-3.5.2.tgz)
* nltk.download('stopwords','punkt')

#### For Mac
* https://github.com/sashkab/homebrew-python
```
brew tap sashkab/python
brew install python35
cd $HOME/software
wget https://bootstrap.pypa.io/get-pip.py
sudo /usr/local/opt/python35/bin/python3.5 $HOME/software/get-pip.py
sudo /usr/local/Cellar/python35/3.5.6_2/Frameworks/Python.framework/Versions/3.5/bin/pip3.5  install nltk
sudo /usr/local/Cellar/python35/3.5.6_2/Frameworks/Python.framework/Versions/3.5/bin/pip3.5  install bs4
sudo /usr/local/Cellar/python35/3.5.6_2/Frameworks/Python.framework/Versions/3.5/bin/pip3.5  install lxml
/usr/local/opt/python35/bin/python3.5
cd $SRC
```
in the Python interactive shell do `import nltk` and then do `nltk.download('stopwords','punkt')`
then run `/usr/local/bin/python3 smartinfo.py --lang=de`


#### sqlite Database to download under the GPLv3.0 License
* http://pillbox.oddb.org/amiko_frequency.db
