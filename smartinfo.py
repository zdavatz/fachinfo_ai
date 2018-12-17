# File name: smartinfo.py
# Author: Max Lungarella <cybrmx@gmail.com>
# Date created: 10/02/2017
# Date last modified: 13/02/2017
# Python Version: 3.5.2
#
# Requirements:
#   List of stopwords in folder input (filename: stopwords.txt)
#   Amiko sqlite DB in folder dbs (filename: amiko_db_full_idx_de.db)
# Output:
#   Frequency csv file in folder output (filename: frequency.csv)
#   Auto-generated stopwords file in folder output (filename: auto_stopwords.csv)
#

import sys
import getopt
import sqlite3 as sql
import nltk
import string
import csv
import time
import re
import os
import hashlib

from collections import Counter
from bs4 import BeautifulSoup
# from nltk.corpus import stopwords
from nltk.tokenize.mwe import MWETokenizer

multi_word_tokenizer = MWETokenizer()
# multi_word_tokenizer.add_mwe(("Multiple", "Sklerose"))

chapter_ids = ["section1", "section2", "section3", "section4", "section5", "section6", "section7", "section8",
               "section9", "section10", "section11", "section12", "section13", "section14", "section15", "section16",
               "Section7000", "Section7050", "Section7100", "Section7150", "Section7200", "Section7250", "Section7350",
               "Section7400", "Section7450", "Section7500", "Section7550", "Section7600", "Section7650", "Section7700"]

list_of_stopwords = []


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
    :return: soup object
    """
    if text is not None:
        # Use lxml's HTML parser
        soup = BeautifulSoup(text, "lxml")

        # Remove title and owner sections
        soup.find("div", {"class": "MonTitle"}).decompose()
        soup.find("div", {"class": "ownerCompany"}).decompose()
        # Remove sections 17-final section, e.g. <div class="paragraph" id="section19">
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
        # Remove footer
        footer = soup.find("p", {"class": "footer"})
        if footer is not None:
            footer.decompose()

        # Replace <br /> with " "
        for e in soup.findAll("br"):
            e.replace_with(" ")

        # Get text as html string
        return soup
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
        tokens = multi_word_tokenizer.tokenize(tokens)
        filtered = [w for w in tokens if (w not in list_of_stopwords and w.lower() not in list_of_stopwords)]
        filtered = [w for w in filtered if w not in string.punctuation]
        filtered = [item for item in filtered if not is_integer(item)]
        filtered = [word for word in filtered if len(word) >= 3]
        return filtered
    return []


def clean_up_string(lang, s):
    """
    Cleans input string
    Note: the regexes are not ORed for clearity
    :param lang: language
    :param s: string
    :return: clean string
    """
    if s is not None:
        chars = "\\`♠↔↓↑«»„“”®'¹³’§‘≡✶•≙≤≥,·†‡‹›ˆ¶"
        for c in chars:
            if c in s:
                s = s.replace(c, "")

        if lang == "de":
            # Remove all -fachen, -faches, etc
            s = re.sub(r"[-]?[0-9]?(Fach(en|es|e)|fac(h|he|hen)|stündig(e|en)|wöchige(r|n)|monatig(e|en)|jährig(e|en))", "", s)
            # Remove special strings
            s = s.replace("o.ä.", "").replace("z.B.", "").replace("’’", "")
        elif lang == "fr":
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
        s = re.sub(r"^[+-]?[0-9]+.[0-9]+?", "", s)
        # Remove all corpses from previous operation (exclude E-numbers, e.g. E218, G6PD, G6PD-Mangel, see issue #5)
        if not s.startswith("E") and not s.startswith("G"):
            s = re.sub(r"^[-|–]?(.)?[0-9]+", "", s)
        # Replace all alpha only strings which start with '-'
        s = re.sub(r"^[-–./*+,](\D+)$", r"\1", s)
        # Remove all n=46
        s = re.sub(r"(\*|-[0-9]+|[0-9]+)n=[0-9]+", "", s)
        # Remove all ...xxx kind of strings (exclude E-numbers, e.g. E218)
        if not s.startswith("E"):
            s = re.sub(r"^...[0-9]+$|^-[0-9]+", "", s)

        # Remove all strings that start with / or start with ,
        if s.startswith("/"):
            s = ""
        if s.startswith("‚"):
            s = s[1:]

        # Remove all strings with this format (+/-)60**
        s = re.sub(r"^[+-−.]?[0-9]+\*+$", "", s)
        # Remove underscores _ from multi words tokenized text, e.g. Multiple_Sklerose
        s = s.replace("_", " ")
        # Remove all strings that are smaller than 3 chars
        if len(s) <= 3:
            s = ""
    return s    # s.encode('utf-8')


def find_chapters_with_tokens(soup, tokens, mw_set):
    """
    Given a soup object representing the "Fachinfo" and a list of tokens/words,
    extracts the ids of the chapters containing those words
    :param soup: soup object
    :param tokens: list of words to match
    :return: dictionary of the form word -> string (chapter1,chapter2,...)
    """
    word_to_chapter_dict = {}

    # Extract all chapter ids
    divs = soup.find_all("div", id=lambda x: x and (x.startswith("section") or x.startswith("Section")))
    for div in divs:
        # Get div id
        id = div.get("id")
        # Proceed only if its a section id
        if id.startswith("section") or id.startswith("Section"):   # Sanity check
            if div is not None:
                div_text = div.get_text(separator=" ")
                #
                if div_text:
                    div_list = get_tokens(div_text)
                    div_list = [s.replace("_", " ") for s in div_list]
                    word_set = (set(tokens) | mw_set) & set(div_list)
                    #
                    if word_set:
                        # remove "section" or "Section" from id
                        clean_id = id.replace("section", "").replace("Section", "")
                        for w in word_set:
                            if w not in word_to_chapter_dict:
                                word_to_chapter_dict[w] = clean_id
                            else:
                                _entry = word_to_chapter_dict[w] + "," + clean_id
                                word_to_chapter_dict[w] = _entry

    return word_to_chapter_dict


def main(argv):
    # List of all relevant files
    amiko_db_full_idx = {"de": "amiko_db_full_idx_de.db", "fr": "amiko_db_full_idx_fr.db"}
    stopwords_file = {"de": "stopwords_de.txt", "fr": "stopwords_fr.txt"}
    whitelist_file = {"de": "whitelist_de.txt", "fr": "whitelist_fr.txt"}
    multiwords_file = {"de": "multiwords_de.txt", "fr": "multiwords_fr.txt"}
    frequency_file = {"de": "frequency_de.csv", "fr": "frequency_fr.csv"}
    auto_stopwords_file = {"de": "auto_stopwords_de.csv", "fr": "auto_stopwords_fr.csv"}
    amiko_frequency_db = {"de": "amiko_frequency_de.db", "fr": "amiko_frequency_fr.db"}

    # Check if directories exist, otherwise generate them
    if not os.path.exists("./output"):
        os.makedirs("./output")

    # Language flag
    lang = "de"
    try:
        opts, args = getopt.getopt(argv, "hlang:", ["lang="])
    except getopt.GetoptError:
        print("smartinfo.py --lang <language>")
        sys.exit(2)
    for opt, arg in opts:
        if opt == "-h":
            print("smartinfo.py --lang <language>")
            sys.exit(2)
        if opt in ("-l", "--lang"):
            lang = arg

    # Open connection to database for reading
    con = None
    rows = []
    try:
        con = sql.connect("./dbs/" + amiko_db_full_idx[lang])
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
    with open("./input/" + stopwords_file[lang], encoding="utf-8") as file:
        stop_words = [line.strip() for line in file]

    # Read our whitelist words
    white_words = []
    with open("./input/" + whitelist_file[lang], encoding="utf-8") as file:
        white_words = [line.strip() for line in file]

    # Read our list of multi words
    multi_words = []
    with open("./input/" + multiwords_file[lang], encoding="utf-8") as file:
        multi_words = [line.rstrip() for line in file]

    # Add multiwords to tokenizer
    for mw in multi_words:
        multi_word_tokenizer.add_mwe(tuple(mw.strip().split(" ")))  # Needs a tuple

    # Note to myself: gotta love list comprehensions
    mw_set = set([mw.strip() for mw in multi_words])

    # Open frequency file for write
    csvfile = open("./output/" + frequency_file[lang], "w", newline="", encoding="utf-8")
    wr = csv.writer(csvfile, quoting=csv.QUOTE_NONE, delimiter=';')

    # Open stop_word file for write
    auto_stopwords_file = open("./output/" + auto_stopwords_file[lang], "w", newline="", encoding="utf-8")
    auto_stop_wr = csv.writer(auto_stopwords_file, quoting=csv.QUOTE_NONE, delimiter=";")

    # All stop words
    global list_of_stopwords
    if lang == "de":
        list_of_stopwords = set(stop_words)  # | set(stopwords.words("german"))
    elif lang == "fr":
        list_of_stopwords = set(stop_words)  # | set(stopwords.words("french"))

    # Open connection to database for writing
    # Format with three columns
    #    id (primary key), keyword, regnr (chapter)
    try:
        con = sql.connect("./output/" + amiko_frequency_db[lang])
        cur = con.cursor()
        # Create a table with two columns
        cur.execute("DROP TABLE IF EXISTS frequency")
        cur.execute("CREATE TABLE frequency (id TEXT PRIMARY_KEY, keyword TEXT, regnr TEXT);")
        con.commit()
    except sql.Error:
        sys.exit(1)

    # Start big loop
    start = time.time()
    word_dict = {}  # Empty dictionary

    for i in range(0, len(rows)):
        title = rows[i][1]
        title = title.replace(";", " ")
        html_content = rows[i][15]

        # Column 5: swissmedic number 5
        # Column 15: html content
        regnr = rows[i][5]
        if regnr:
            regnr = regnr.split(",")[0]

        if regnr:   # == "63175":
            soup_object = remove_html_tags(html_content)
            if soup_object:
                clean_text = soup_object.get_text(separator=" ")
                if clean_text:
                    tokens = get_tokens(clean_text)
                    # Note to myself: list comprehensions are cool!
                    tokens = [clean_up_string(lang, t) for t in tokens]
                    # Remove empty strings (note: filter retuns a filter object -> needs to be transformed to list)
                    tokens = list(filter(None, tokens))
                    # Get word count
                    count = Counter(tokens)
                    size = len(count)
                    frequency_list = sorted(list(count.most_common(size)))

                    # Dictionary of the form word -> string (chapter1,chapter2,...)
                    w_to_c_dict = find_chapters_with_tokens(soup_object, tokens, mw_set)

                    # Add to map
                    for word in frequency_list:
                        w = word[0]
                        if w:
                            ch_ids = ""
                            if w in w_to_c_dict:
                                ch_ids = "(" + w_to_c_dict[w] + ")"
                            regnr_prime = regnr + ch_ids
                            if w not in word_dict:
                                word_dict[w] = regnr_prime
                            else:
                                updated_entry = word_dict[w] + "|" + regnr_prime
                                word_dict[w] = updated_entry

                    print(title, frequency_list)

    print("\n============================================================\n")
    cnt = 0
    for k in sorted(word_dict):
        r = word_dict[k]    # registration number swissmedic-5
        # Change this number to increase or decrease the number of auto-generated stopwords
        word_count = len(r.split(","))

        if k not in white_words and k not in multi_words and word_count > 600:
            auto_stop_wr.writerow((k, word_count))
        else:
            cnt += 1
            line = (k, r)       # word, registration numbers
            wr.writerow(line)
            if cnt % 100 == 0:
                print("\rSaved: %d" % cnt, end='', flush=True)
                con.commit()
            # Generate 16 byte hash
            hashed_k = hashlib.sha256(k.encode('utf-8')).hexdigest()[:10]
            query = "INSERT INTO frequency VALUES('%s', '%s', '%s');" % (hashed_k, k, r)
            con.execute(query)

    if con:
        con.commit()
        con.close()

    end = time.time()

    print("\n\n============================================================\n")
    print("Elapsed time = %.3fs" % (end-start))

# MAIN starts here
if __name__ == "__main__":
    main(sys.argv[1:])
