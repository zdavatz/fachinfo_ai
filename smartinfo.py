# File name: smartinfo.py
# Author: Max Lungarella <cybrmx@gmail.com>
# Date created: 10/02/2017
# Date last modified: 13/02/2017
# Python Version: 3.4
#
# Requirements:
#   List of stopwords in folder input (filename: stopwords.txt)
#   Amiko sqlite DB in folder dbs (filename: amiko_db_full_idx_de.db)
# Output:
#   Frequency csv file in folder output (filename: frequency.csv)
#   Auto-generated stopwords file in folder output (filename: auto_stopwords.csv)
#

import sys
import sqlite3 as sql
import nltk
import string
import csv
import time
import re

from collections import Counter
from bs4 import BeautifulSoup
from nltk.corpus import stopwords

con = None
rows = []


def is_integer(s):
    """
    Checkes if string is an integer
    :param s: string
    :return: bool
    """
    return s.isdigit() or (s[0] == '-' and s[1:].isdigit())


def remove_html_tags(text):
    """
    Removes html tags
    :param text: html string
    :return: clean string
    """
    if text is not None:
        # lowers = text.lower()
        soup = BeautifulSoup(text, "lxml")
        # Remove sections 17-final section, e.g. <div class="paragraph" id="section19">
        soup.find("div", {"class": "MonTitle"}).decompose()
        soup.find("div", {"class": "ownerCompany"}).decompose()
        divs = ["section17",
                "section18",
                "section19",
                "section20",
                "section21",
                "Section7750",
                "Section7800",
                "Section7850",
                "Section8000"]

        for d in divs:
            div = soup.find("div", {"id": d})
            if div is not None:
                div.decompose()

        footer = soup.find("p", {"class": "footer"})
        if footer is not None:
            footer.decompose()

        return soup.get_text()
    return []


def get_tokens(text):
    """
    Tokenizes given string
    :param text: string
    :return: list of tokens
    """
    if text is not None:
        # Remove the punctuation using the character deletion step of translate
        tokens = nltk.word_tokenize(text)
        filtered = [w for w in tokens if w not in all_stopwords]
        filtered = [w for w in filtered if w not in string.punctuation]
        filtered = [item for item in filtered if not is_integer(item)]
        filtered = [word for word in filtered if len(word) > 3]
        return filtered
    return []


def clean_up_string(s):
    """
    Cleans input string
    :param s: string
    :return: clean string
    """
    if s is not None:
        #
        chars = "\\`♠↔↓↑«»„“”®×'¹³’§‘≡✶•≙≤≥,·†‡‹›ˆ¶*"
        for c in chars:
            if c in s:
                s = s.replace(c, "")
        # Remove all -fachen, -faches, etc
        s = re.sub(r"[-]?[0-9]?(Fach(en|es|e)|fac(h|he|hen)|stündig(e|en)|wöchige(r|n)|monatig(e|en)|jährig(e|en))", "", s)
        # Remove numbers and dots before letters, e.g. 1.Drehen -> Drehen
        s = re.sub(r"^([0-9]+.|-|−)([a-zA-ZäöüèéàÜÖÄ]+)$", "\2", s)
        # Remove time with format hh:mm:ss
        s = re.sub(r"^[0-9]{2}:[0-9]{2}:[0-9]{2}$", "", s)
        # Replace , in numbers, e.g. 0,001 -> 0.001
        s = re.sub(r"([+-]?[0-9]+),([0-9]+)", r"\1.\2", s)
        # Replace ' in numbers, e.g. 10'000 -> 10000
        s = re.sub(r"([+-]?[0-9]+)'([0-9]+)", r"\1\2", s)
        # Remove all numbers
        s = re.sub(r"[+-]?[0-9]+.[0-9]+?", "", s)
        # Remove all corpses from previous operation
        s = re.sub(r"^[-|–]?(.)?[0-9]+$", "", s)
        # Replace all alpha only strings which start with '-'
        s = re.sub(r"^[-–./*+,](\D+)$", r"\1", s)
        # Remove all n=46
        s = re.sub(r"(\*|-[0-9]+|[0-9]+)n=[0-9]+", "", s)
        # Remove all ...xxx kind of strings
        s = re.sub(r"^...[0-9]+$|^-[0-9]+", "", s)
        # Remove all strings that start with / or start with ,
        if s.startswith("/"):
            s = ""
        if s.startswith("‚"):
            s = s[1:]
        # Remove all strings with this format (+/-)60**
        s = re.sub(r"^[+-−.]?[0-9]+\*+$", "", s)
        # Remove special strings
        s = s.replace("o.ä.", "").replace("z.B.", "").replace("’’", "")
        # Remove all strings that are smaller than 3 chars
        if len(s) <= 3:
            s = ""
    return s

try:
    # Open connection to database
    con = sql.connect("./dbs/amiko_db_full_idx_de.db")
    cur = con.cursor()
    # Retrieve all articles
    query = "SELECT * FROM amikodb"
    cur.execute(query)
    rows = cur.fetchall()

except sql.Error:
    print("Error %s:" % sql.Error.args[0])
    sys.exit(1)

finally:
    if con:
        con.close()

# Read our stop words
stop_words = []
with open("./input/stopwords.txt", encoding="utf-8") as file:
    for line in file:
        line = line.strip()
        stop_words.append(line)

# All stop words
all_stopwords = set(stopwords.words('german')) | set(stop_words)

# Open file for write
csvfile = open("./output/frequency.csv", "w", newline="", encoding="utf-8")
wr = csv.writer(csvfile, quoting=csv.QUOTE_NONE, delimiter=';')

# Open stop_word file for write
auto_stopwords_file = open("./output/auto_stopwords.csv", "w", newline="", encoding="utf-8")
auto_wr = csv.writer(auto_stopwords_file, quoting=csv.QUOTE_NONE, delimiter=";")

# Column 5: swissmedic number 5
# Column 15: html content
start = time.time()
word_dict = {}  # Empty dictionary

for i in range(0, len(rows)):
    title = rows[i][1]
    title = title.replace(";", " ")
    html_content = rows[i][15]

    regnr = rows[i][5]
    if regnr:
        regnr = regnr.split(",")[0]

    if regnr:
        clean_text = remove_html_tags(html_content)
        if clean_text:
            tokens = get_tokens(clean_text)
            tokens = [clean_up_string(t) for t in tokens]
            count = Counter(tokens)
            size = len(count)
            frequency_list = sorted(list(count.most_common(size)))

            # Add to map
            for word in frequency_list:
                w = word[0]
                if w:
                    if w not in word_dict:
                        word_dict[w] = regnr
                    else:
                        updated_entry = word_dict[w] + ", " + regnr
                        word_dict[w] = updated_entry

            print(title, frequency_list)

for k in sorted(word_dict):
    r = word_dict[k]
    if len(r.split(",")) > 400:
        auto_wr.writerow([k])
    else:
        line = (k, r)
        wr.writerow(line)

end = time.time()

print("\nElapsed time = %.3fs" % (end-start))
