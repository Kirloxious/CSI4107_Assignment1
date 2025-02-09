use assignment1::{indexing::*, preprocessing::*, ranking::*};
use std::collections::HashMap;
use std::time::Instant;
use std::{fs::File, io::Write}; //import functions

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
    let rank = Ranking::init(&doc_lengths, &inverted_index, 1.2, 0.75);

    let start = Instant::now();
    let results = rank.rank_documents(&queries);
    let duration = start.elapsed();
    println!("{:?}", duration);

    println!("Vocab lengths: {:?}", inverted_index.keys().len());

    save_results_to_file(results, "saved/results.tsv");
}
