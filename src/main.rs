use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;
use std::{
    collections::{BTreeSet, HashMap},
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    vec,
};

lazy_static! {
    static ref WORD_REGEX: Regex = Regex::new(r"\w+(?:'\w+)?|[^\w\s]").unwrap();
}

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
    return WORD_REGEX
        .find_iter(str.as_str())
        .map(|m| m.as_str())
        .filter(|w| w.chars().all(|c| !c.is_digit(10))) //remove numbers
        .filter(|w| w.chars().all(|c| !c.is_ascii_punctuation())) //remove punctuation
        .collect();
}

fn remove_stopwords(words: &mut Vec<&str>, stopwords: &HashSet<String>) {
    words.retain(|e| !stopwords.contains(*e));
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

fn preprocess_text(str: String, stopwords: &HashSet<String>) -> HashMap<String, u16> {
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

fn load_stopwords() -> HashSet<String> {
    let file = File::open("scifact/stopwords.txt").unwrap();
    BufReader::new(file)
        .lines()
        .map(|line| line.unwrap())
        .collect()
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

fn load<T: for<'de> Deserialize<'de>>(file_path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut buf: Vec<u8> = vec![];
    file.read_to_end(&mut buf)?;
    let data = serde_json::from_slice::<T>(&buf)?;
    Ok(data)
}

fn initial_inverted_index_setup() {
    let stopwords = load_stopwords();
    let mut documents: Vec<TokenizedDocument> = vec![];
    let file = File::open("scifact/corpus.jsonl").unwrap();
    let buffered_reader = BufReader::new(file);
    let mut document_lengths = HashMap::new();
    for line in buffered_reader.lines() {
        let d: Document = serde_json::from_str(line.unwrap().as_str()).expect("msg");
        // let mut text_tokens = preprocess_text(d.text, &stopwords);
        let title_tokens = preprocess_text(d.title, &stopwords);
        // text_tokens.extend(title_tokens); // combine title token with text tokens
        document_lengths.insert(d._id.clone(), title_tokens.len().clone() as u32);
        documents.push(TokenizedDocument {
            _id: d._id.parse::<u32>().unwrap(),
            tokens: title_tokens,
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

struct Ranking<'a> {
    k1: f32,
    b: f32,
    avgdl: u32,
    num_doc: u32,
    inv_index: &'a InvertedIndex,
    doc_lengths: &'a HashMap<u32, u32>,
}

impl<'a> Ranking<'a> {
    fn init(
        doc_lengths: &'a HashMap<u32, u32>,
        inverted_index: &'a InvertedIndex,
        k1: f32,
        b: f32,
    ) -> Ranking<'a> {
        let num_doc = doc_lengths.len() as u32;
        let avgdl = doc_lengths.clone().into_values().sum::<u32>() / num_doc;

        Ranking {
            k1,
            b,
            avgdl,
            num_doc,
            inv_index: inverted_index,
            doc_lengths,
        }
    }
    fn idf(&self, term: &str) -> f32 {
        // if inv_index doesnt contain term, idf is 0
        let df = self.inv_index.get(term).map_or(0, |map| map.len());
        if df == 0 {
            return 0.0;
        }
        return ((self.num_doc as f32 - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();
    }

    fn bm25_weight(&self, doc_id: &u32, term: &str) -> f32 {
        let doc_length = *self.doc_lengths.get(doc_id).unwrap_or(&0);
        if let Some(term_map) = self.inv_index.get(term) {
            if let Some(&tf) = term_map.get(doc_id) {
                let idf = self.idf(term);
                return idf * tf as f32
                    / (self.k1
                        * ((1.0 - self.b) + self.b * (doc_length as f32 / self.avgdl as f32))
                        + tf as f32);
            }
        }
        0.0
    }

    fn vector_length(&self, weights: &[f32]) -> f32 {
        weights
            .iter()
            .map(|weight| weight.powi(2))
            .sum::<f32>()
            .sqrt()
    }

    fn cosine_similarity(&self, doc_id: &u32, query_terms: &TokenizedQuery) -> f32 {
        let mut sum = 0.0;
        let mut doc_weights = vec![];
        let mut q_weights = vec![];

        for (term, freq) in &query_terms.tokens {
            let doc_term_weight = self.bm25_weight(doc_id, term);
            let query_term_weight = self.idf(term) * (*freq as f32);

            sum += query_term_weight * doc_term_weight;

            q_weights.push(query_term_weight);
            doc_weights.push(doc_term_weight);
        }

        let doc_len = self.vector_length(&doc_weights);
        let q_len = self.vector_length(&q_weights);

        if doc_len > 0.0 && q_len > 0.0 {
            sum / (doc_len * q_len)
        } else {
            0.0
        }
    }

    fn rank_documents(&self, queries: &[TokenizedQuery]) -> BTreeMap<u32, BTreeSet<RankingResult>> {
        let mut results: BTreeMap<u32, BTreeSet<RankingResult>> = BTreeMap::new();
        const MAX_TREE_SIZE: usize = 100;

        for query in queries.iter() {
            for term in query.tokens.keys() {
                if let Some(doc_map) = self.inv_index.get(term) {
                    for (doc_id, _) in doc_map.iter() {
                        let q_id = query._id.parse::<u32>().unwrap();
                        let tag = (doc_id + q_id) % 2_u32.pow(23);

                        let score = self.cosine_similarity(doc_id, query);
                        let q_entry = results.entry(q_id).or_insert(BTreeSet::new());
                        q_entry.insert(RankingResult {
                            query_id: q_id,
                            doc_id: *doc_id,
                            score,
                            tag,
                        });

                        // Remove the smallest result if the new score is bigger and more than 100 values in tree.
                        if q_entry.len() > MAX_TREE_SIZE {
                            q_entry.pop_first();
                        }
                    }
                }
            }
        }

        return results;
    }
}

fn save_results_to_file(results: BTreeMap<u32, BTreeSet<RankingResult>>, file_path: &str) {
    let mut file = File::create(file_path).expect("Failed to create file.");
    for result in results.iter() {
        let mut rank = 0;
        for query_ranking in result.1.iter().rev() {
            rank += 1;
            file.write_fmt(format_args!(
                "{}  {}  {}  {}  {}  {}\n",
                query_ranking.query_id,
                "Q0",
                query_ranking.doc_id,
                rank,
                query_ranking.score,
                query_ranking.tag
            ))
            .expect("Failed to write to file.");
        }
    }
}

// query_id Q0 doc_id rank score tag
#[derive(Debug)]
struct RankingResult {
    query_id: u32,
    doc_id: u32,
    score: f32,
    tag: u32,
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

fn save_vocab(inverted_index: &InvertedIndex) {
    let mut f = File::create("saved/vocab_sample.txt").unwrap();
    inverted_index
        .keys()
        .collect::<Vec<&String>>()
        .iter()
        .enumerate()
        .for_each(|(n, w)| {
            if n < 100 {
                f.write_fmt(format_args!("{w}\n")).unwrap();
            }
        });
}

fn main() {
    // To run the setup code, compile with cargo run --features setup

    // Created the inverted index & doc_length and saved to file
    #[cfg(feature = "setup")]
    initial_inverted_index_setup();

    // Tokenized the queries and saved to file
    #[cfg(feature = "setup")]
    initial_query_setup();

    let inverted_index: InvertedIndex = load("saved/inverted_index.json").expect("Failed to load");
    let queries: Vec<TokenizedQuery> = load("saved/query_tokens.json").expect("Failed to load");
    let doc_lengths: HashMap<u32, u32> = load("saved/doc_lengths.json").expect("Failed to load");
    let rank = Ranking::init(&doc_lengths, &inverted_index, 1.75, 0.75);

    let start = Instant::now();
    let results = rank.rank_documents(&queries);
    let duration = start.elapsed();
    println!("{:?}", duration);

    println!("Vocab lengths: {:?}", inverted_index.keys().len());

    save_results_to_file(results, "saved/results.tsv");
}
