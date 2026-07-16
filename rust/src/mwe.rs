//! Port of nltk.tokenize.mwe.MWETokenizer (separator "_").

use std::collections::HashMap;

#[derive(Default)]
struct TrieNode {
    children: HashMap<String, TrieNode>,
    leaf: bool,
}

#[derive(Default)]
pub struct MweTokenizer {
    root: TrieNode,
}

impl MweTokenizer {
    pub fn add_mwe(&mut self, words: &[String]) {
        let mut node = &mut self.root;
        for w in words {
            node = node.children.entry(w.clone()).or_default();
        }
        node.leaf = true;
    }

    pub fn tokenize(&self, text: Vec<String>) -> Vec<String> {
        let n = text.len();
        let mut result = Vec::with_capacity(n);
        let mut i = 0;
        while i < n {
            if self.root.children.contains_key(&text[i]) {
                // possible MWE match
                let mut j = i;
                let mut node = &self.root;
                let mut last_match: isize = -1;
                while j < n {
                    match node.children.get(&text[j]) {
                        Some(next) => {
                            node = next;
                            j += 1;
                            if node.leaf {
                                last_match = j as isize;
                            }
                        }
                        None => break,
                    }
                }
                if last_match > -1 {
                    let j = last_match as usize;
                    result.push(text[i..j].join("_"));
                    i = j;
                } else {
                    result.push(text[i].clone());
                    i += 1;
                }
            } else {
                result.push(text[i].clone());
                i += 1;
            }
        }
        result
    }
}
