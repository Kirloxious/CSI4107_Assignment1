use crate::preprocessing::*;
use std::io::BufRead;
use std::io::BufReader;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
};

use serde::{Deserialize, Serialize};

// Type alias to define inverted index
// {token: {doc_id, frequency}, ...}
pub type InvertedIndex = HashMap<String, HashMap<u32, u16>>;

pub fn save<T: Serialize>(container: T, file_path: &str) {
    let mut file = File::create(file_path).expect("Failed to create file at specified path.");
    let json_data = serde_json::to_string(&container).expect("Failed to serialize data.");
    file.write_all(json_data.as_bytes())
        .expect("Failed to write to file.");
}

pub fn load<T: for<'de> Deserialize<'de>>(
    file_path: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut buf: Vec<u8> = vec![];
    file.read_to_end(&mut buf)?;
    let data = serde_json::from_slice::<T>(&buf)?;
    Ok(data)
}

pub fn build_inverted_index(documents: Vec<TokenizedDocument>) -> InvertedIndex {
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

pub fn initial_inverted_index_setup() {
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

pub fn initial_query_setup() {
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
