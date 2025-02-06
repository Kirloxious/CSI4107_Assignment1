use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
};

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref WORD_REGEX: Regex = Regex::new(r"\w+(?:'\w+)?|[^\w\s]").unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Query {
    pub _id: String,
    pub text: String,
    pub metadata: HashMap<String, Vec<InnerMetadata>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InnerMetadata {
    pub sentences: Vec<u8>,
    pub label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenizedQuery {
    pub _id: String,
    pub tokens: HashMap<String, u16>,
    pub metadata: HashMap<String, Vec<InnerMetadata>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Document {
    pub _id: String,
    pub title: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenizedDocument {
    pub _id: u32,
    pub tokens: HashMap<String, u16>,
}

pub fn extract_words(str: &String) -> Vec<&str> {
    return WORD_REGEX
        .find_iter(str.as_str())
        .map(|m| m.as_str())
        .filter(|w| w.chars().all(|c| !c.is_digit(10))) //remove numbers
        .filter(|w| w.chars().all(|c| !c.is_ascii_punctuation())) //remove punctuation
        .collect();
}

pub fn remove_stopwords(words: &mut Vec<&str>, stopwords: &HashSet<String>) {
    words.retain(|e| !stopwords.contains(*e));
}

pub fn stem_words(words: Vec<&str>) -> Vec<String> {
    return words
        .iter()
        .map(|w| match stem::get(&w) {
            Ok(stemmed) => stemmed.to_lowercase(),
            Err(_e) => String::from(""),
        })
        .collect();
}

pub fn preprocess_text(str: String, stopwords: &HashSet<String>) -> HashMap<String, u16> {
    let mut words = extract_words(&str);
    remove_stopwords(&mut words, stopwords);
    let mut stemmed_words = stem_words(words);
    stemmed_words.retain(|w| w.len() > 1); // remove words that ended up being 1 letter or less
    let mut frequency: HashMap<String, u16> = HashMap::new();
    stemmed_words
        .into_iter()
        .for_each(|word| *frequency.entry(word).or_insert(0) += 1);
    return frequency;
}

pub fn load_stopwords() -> HashSet<String> {
    let file = File::open("scifact/stopwords.txt").unwrap();
    BufReader::new(file)
        .lines()
        .map(|line| line.unwrap())
        .collect()
}

pub fn process_queries(queries: Vec<Query>) -> Vec<TokenizedQuery> {
    //extract words, remove stopwords, stem
    let mut tokenized: Vec<TokenizedQuery> = vec![];
    let stopwords = load_stopwords();
    for query in queries {
        let mut words = extract_words(&query.text);
        remove_stopwords(&mut words, &stopwords);
        let mut stemmed_words = stem_words(words);
        stemmed_words.retain(|w| w.len() > 1); // remove words that ended up being 2 letter or less
        let mut frequency: HashMap<String, u16> = HashMap::new();
        for word in stemmed_words {
            *frequency.entry(word).or_insert(0) += 1;
        }
        tokenized.push(TokenizedQuery {
            _id: query._id,
            tokens: frequency,
            metadata: query.metadata,
        });
    }

    return tokenized;
}
