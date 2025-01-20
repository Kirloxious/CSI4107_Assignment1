#![allow(dead_code)]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    iter::Sum,
    vec,
};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    _id: String,
    title: String,
    text: String,
}

#[derive(Debug)]
struct TokenizedDocument {
    _id: u32,
    tokens: HashMap<String, u16>,
}

// {token: {doc_id, frequency}, ...}
type InvertedIndex = HashMap<String, HashMap<u32, u16>>;

fn extract_words(str: &String) -> Vec<&str> {
    let regex = Regex::new(r"\w+(?:'\w+)?|[^\w\s]").unwrap();
    return regex
        .find_iter(str.as_str())
        .map(|m| m.as_str())
        .filter(|w| w.chars().all(|c| !c.is_digit(10))) //remove numbers
        .filter(|w| w.chars().all(|c| !c.is_ascii_punctuation())) //remove punctuation
        .collect();
}

fn remove_stopwords(words: &mut Vec<&str>, stopwords: &Vec<String>) {
    let remove = BTreeSet::from_iter(stopwords.to_owned());
    words.retain(|e| !remove.contains(e.to_owned()));
}

fn stem_words(words: Vec<&str>) -> Vec<String> {
    return words
        .iter()
        .map(|w| match stem::get(&w) {
            Ok(stemmed) => stemmed.to_lowercase(),
            Err(_e) => String::from(""),
        })
        .collect();
}

// fn tokenize(str: &String, regex: Regex) -> Vec<&str> {
//     let tokens = regex.find_iter(str.as_str()).map(|m| m.as_str()).collect();
//     return tokens;
// }

// fn remove_extras(tokens: &mut Vec<String>) {
//     let set: HashSet<_> = tokens.drain(..).collect();
//     tokens.extend(set.into_iter());
// }

// fn combine_tokens(mut t1: Vec<String>, mut t2: Vec<String>) -> Vec<String> {
//     let mut combined: Vec<String> = vec![];
//     combined.append(&mut t1);
//     combined.append(&mut t2);
//     remove_extras(&mut combined);
//     return combined;
// }

fn preprocess_text(str: String, stopwords: &Vec<String>) -> HashMap<String, u16> {
    let mut words = extract_words(&str);
    remove_stopwords(&mut words, stopwords);
    let stemmed_words = stem_words(words);
    let mut frequency: HashMap<String, u16> = HashMap::new();
    for word in stemmed_words {
        *frequency.entry(word).or_insert(0) += 1;
    }
    return frequency;
}

fn load_stopwords() -> Vec<String> {
    let mut stopwords = vec![];
    let file = File::open("scifact/stopwords.txt").unwrap();
    let buffered_reader = BufReader::new(file);
    for line in buffered_reader.lines() {
        stopwords.push(line.unwrap());
    }
    return stopwords;
}

fn build_inverted_index(documents: Vec<TokenizedDocument>) -> InvertedIndex {
    let mut inverted_index: InvertedIndex = HashMap::new();
    // Token: {doc_id, freq}
    for doc in documents {
        let tokens = doc.tokens;
        for (token, freq) in tokens {
            // Inserts a key only if it doesnt exist
            // if it does, returns mut reference for updating
            let token_map = inverted_index.entry(token).or_insert(HashMap::new());
            token_map.insert(doc._id, freq);
        }
    }
    return inverted_index;
}

fn save_inverted_index(inverted_index: InvertedIndex, file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    let json_data = serde_json::to_string(&inverted_index).unwrap();
    file.write_all(json_data.as_bytes()).unwrap();
}

fn load_inverted_index(file_path: &str) -> InvertedIndex {
    let mut file = File::open(file_path).unwrap();
    let mut contents: String = String::from("");
    file.read_to_string(&mut contents).unwrap();
    return serde_json::from_str::<InvertedIndex>(&contents).unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct Query {
    _id: String,
    text: String,
    metadata: HashMap<String, Vec<InnerMetadata>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InnerMetadata {
    sentences: Vec<u8>,
    label: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TokenizedQuery {
    _id: String,
    tokens: HashMap<String, u16>,
    metadata: HashMap<String, Vec<InnerMetadata>>,
}

fn process_queries(queries: Vec<Query>) -> Vec<TokenizedQuery> {
    //extract words, remove stopwords, stem
    let mut tokenized: Vec<TokenizedQuery> = vec![];
    let stopwords = load_stopwords();
    for query in queries {
        let mut words = extract_words(&query.text);
        remove_stopwords(&mut words, &stopwords);
        let stemmed_words = stem_words(words);
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

fn save_queries(queries: Vec<TokenizedQuery>, file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    let json_data = serde_json::to_string(&queries).unwrap();
    file.write_all(json_data.as_bytes()).unwrap();
}

fn initial_inverted_index_setup() {
    let stopwords = load_stopwords();
    let mut documents: Vec<TokenizedDocument> = vec![];
    let file = File::open("scifact/corpus.jsonl").unwrap();
    let buffered_reader = BufReader::new(file);
    for line in buffered_reader.lines() {
        let d: Document = serde_json::from_str(line.unwrap().as_str()).expect("msg");
        let text_tokens = preprocess_text(d.text, &stopwords);
        documents.push(TokenizedDocument {
            _id: d._id.parse::<u32>().unwrap(),
            tokens: text_tokens,
        });
    }
    let inverted_index = build_inverted_index(documents);
    save_inverted_index(inverted_index, "saved/inverted_index.json");
}

fn initial_query_setup() {
    let mut queries: Vec<Query> = vec![];
    let file = File::open("scifact/queries.jsonl").unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let q: Query = serde_json::from_str(line.unwrap().as_str()).unwrap();
        queries.push(q);
    }
    let tokenized = process_queries(queries);
    save_queries(tokenized, "saved/query_tokens.json");
}

struct BM25 {
    k1: u8,
    b: u8,
    avgdl: u32,
    num_docs: usize,
    doc_len: HashMap<String, u32>,
    inv_index: InvertedIndex,
}

impl BM25 {
    fn init(doc_len: HashMap<String, u32>, inverted_index: InvertedIndex, k1: u8, b: u8) -> BM25 {
        BM25 {
            k1,
            b,
            avgdl: doc_len.values().sum::<u32>() / (doc_len.len() as u32),
            num_docs: doc_len.len(),
            doc_len,
            inv_index: inverted_index,
        }
    }
    fn idf(self: &BM25, term: String) {}

    fn bm25_score(self: &BM25) {}
}

fn main() {
    // Created the inverted index and saved to file
    // initial_inverted_index_setup();\

    // Tokenized the queries and saved to file
    // initial_query_setup();

    let inverted_index = load_inverted_index("saved/inverted_index.json");
}
