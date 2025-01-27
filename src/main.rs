#![allow(dead_code)]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    vec,
};

#[derive(Serialize, Deserialize, Debug)]
struct Document {
    _id: String,
    title: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

fn preprocess_text(str: String, stopwords: &Vec<String>) -> HashMap<String, u16> {
    let mut words = extract_words(&str);
    remove_stopwords(&mut words, stopwords);
    let mut stemmed_words = stem_words(words);
    stemmed_words.retain(|w| w.len() > 1); // remove words that ended up being 1 letter or less
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

#[derive(Serialize, Deserialize, Debug)]
struct Query {
    _id: String,
    text: String,
    metadata: HashMap<String, Vec<InnerMetadata>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InnerMetadata {
    sentences: Vec<u8>,
    label: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

fn save<T: Serialize>(container: T, file_path: &str) {
    let mut file = File::create(file_path).expect("Failed to create file at specified path.");
    let json_data = serde_json::to_string(&container).expect("Failed to serialize data.");
    file.write_all(json_data.as_bytes())
        .expect("Failed to write to file.");
}

fn load<T: for<'de> Deserialize<'de>>(file_path: &str) -> T {
    let mut file = File::open(file_path).expect("Failed to open file at specified path.");
    let mut buf: Vec<u8> = vec![];
    file.read_to_end(&mut buf).expect("Failed to read file.");
    return serde_json::from_slice::<T>(&buf).expect("Failed to deserialize data.");
}

fn initial_inverted_index_setup() {
    let stopwords = load_stopwords();
    let mut documents: Vec<TokenizedDocument> = vec![];
    let file = File::open("scifact/corpus.jsonl").unwrap();
    let buffered_reader = BufReader::new(file);
    let mut document_lengths = HashMap::new();
    for line in buffered_reader.lines() {
        let d: Document = serde_json::from_str(line.unwrap().as_str()).expect("msg");
        let mut text_tokens = preprocess_text(d.text, &stopwords);
        let title_tokens = preprocess_text(d.title, &stopwords);
        text_tokens.extend(title_tokens); // combine title token with text tokens
        document_lengths.insert(d._id.clone(), text_tokens.len().clone() as u32);
        documents.push(TokenizedDocument {
            _id: d._id.parse::<u32>().unwrap(),
            tokens: text_tokens,
        });
    }
    let mut documents_map: HashMap<&u32, Vec<String>> = HashMap::new();
    for TokenizedDocument { _id, tokens } in &documents {
        documents_map.insert(_id, tokens.clone().into_keys().collect());
    }
    save(&documents_map, "saved/doc_tokens.json");
    save(&document_lengths, "saved/doc_lengths.json");

    let inverted_index = build_inverted_index(documents);
    save(inverted_index, "saved/inverted_index.json");
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
    save(tokenized, "saved/query_tokens.json");
}

struct Ranking {
    k1: f32,
    b: f32,
    avgdl: u32,
    num_doc: u32,
    doc_lengths: HashMap<u32, u32>,
    inv_index: InvertedIndex,
}

impl Ranking {
    fn init(doc_len: HashMap<u32, u32>, inverted_index: InvertedIndex, k1: f32, b: f32) -> Ranking {
        let num_doc = doc_len.len() as u32;
        let avgdl = doc_len.clone().into_values().sum::<u32>() / num_doc;

        Ranking {
            k1,
            b,
            avgdl,
            num_doc,
            doc_lengths: doc_len,
            inv_index: inverted_index,
        }
    }
    fn idf(self: &Ranking, term: &String) -> f32 {
        let df = self.inv_index.get(term).unwrap().len();
        return ((self.num_doc as f32 - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();
    }

    // fn bm25_score(self: &Ranking, doc_id: u32, query_terms: TokenizedQuery) -> f32 {

    //     let mut score = 0.0;
    //     let doc_length = self.doc_lengths.get(&doc_id).unwrap();
    //     for (term, _) in query_terms.tokens {
    //         if let Some(tf) = self.inv_index.get(&term).unwrap().get(&doc_id) {
    //             let idf_value = self.idf(term);
    //             let term_score = idf_value * (*tf as f32 * (self.k1 + 1.0))
    //                 / (*tf as f32
    //                     + self.k1
    //                         * (1.0 - self.b + self.b * *doc_length as f32 / self.avgdl as f32))
    //                     as f32;
    //             score += term_score;
    //         }
    //     }
    //     return score;
    // }

    fn bm25_weight(self: &mut Ranking, doc_id: u32, term: &String) -> f32 {
        //Calculate bm25 score for each term in document
        let mut term_weight = 0.0;
        let doc_length = self.doc_lengths.get(&doc_id).unwrap();
        if let Some(tf) = self.inv_index.get(term).unwrap().get(&doc_id) {
            // let idf_value = self.idf(term);
            let df = self.inv_index.get(term).unwrap().len() as f32;
            term_weight = (*tf as f32 * ((self.num_doc as f32 - df + 0.5) / df + 0.5).ln())
                / self.k1
                * ((1.0 - self.b) + self.b * *doc_length as f32 / self.avgdl as f32)
                + *tf as f32;
        }
        return term_weight;
    }

    fn vector_length(self: &Ranking, tokens: Vec<&String>) -> f32 {
        let mut sum: f32 = 0.0;
        for term in tokens {
            let idf = self.idf(term);
            sum += idf.powi(2);
        }
        return sum.sqrt();
    }

    fn cosine_similarity(self: &mut Ranking, doc_id: u32, query_terms: TokenizedQuery) -> f32 {
        let mut sum = 0.0;

        for (term, freq) in &query_terms.tokens {
            let doc_term_weight = self.bm25_weight(doc_id, &term);
            let query_term_idf =
                doc_term_weight * ((*freq as f32) / self.inv_index.get(term).unwrap().len() as f32);
            sum += query_term_idf * doc_term_weight
        }

        let q_terms: Vec<&String> = query_terms.tokens.keys().collect();
        let mut d_terms = vec![];
        //extract the documents terms from the inverted index
        for (key, value) in self.inv_index.iter() {
            if value.contains_key(&doc_id) {
                d_terms.push(key);
            }
        }

        let doc_len = self.vector_length(d_terms);
        let q_len = self.vector_length(q_terms);

        let result = sum / (doc_len * q_len);
        return result;
    }
}

// query_id Q0 doc_id rank score tag
#[derive(Debug)]
struct RankingResult {
    query_id: u32,
    doc_id: u32,
    score: f32,
    tag: u8,
}

impl PartialOrd for RankingResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.score.partial_cmp(&other.score)
    }
}

impl PartialEq for RankingResult {
    fn eq(&self, other: &Self) -> bool {
        self.score.eq(&other.score)
    }
}

impl Ord for RankingResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.score.partial_cmp(&other.score) {
            Some(t) => t,
            None => std::cmp::Ordering::Less,
        }
    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::max_by(self, other, Ord::cmp)
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        std::cmp::min_by(self, other, Ord::cmp)
    }
}

impl Eq for RankingResult {}

fn main() {
    // Created the inverted index & doc_length and saved to file
    // initial_inverted_index_setup();

    // Tokenized the queries and saved to file
    // initial_query_setup();

    let inverted_index: InvertedIndex = load("saved/inverted_index.json");
    let queries: Vec<TokenizedQuery> = load("saved/query_tokens.json");
    let doc_lengths: HashMap<u32, u32> = load("saved/doc_lengths.json");
    // let _doc_tokens: HashMap<u32, Vec<String>> = load("saved/doc_tokens.json");
    let mut rank = Ranking::init(doc_lengths.clone(), inverted_index, 1.75, 0.75);

    //Using BTree to auto sort on insert.

    let mut results: BTreeSet<RankingResult> = BTreeSet::new();
    for doc_id in doc_lengths {
        let r = rank.cosine_similarity(doc_id.0, queries[1].clone());
        if r > 0.01 {
            results.insert(RankingResult {
                query_id: queries[1]._id.clone().parse::<u32>().unwrap(),
                doc_id: doc_id.0,
                score: r,
                tag: 1,
            });
        }
    }
    println!("{:?}", results.first());
    println!("{:?}", results);
}
