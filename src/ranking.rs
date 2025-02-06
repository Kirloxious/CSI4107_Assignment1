use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs::File,
    io::Write,
};

use crate::{indexing::InvertedIndex, preprocessing::TokenizedQuery};

pub struct Ranking<'a> {
    pub k1: f32,
    pub b: f32,
    pub avgdl: u32,
    pub num_doc: u32,
    pub inv_index: &'a InvertedIndex,
    pub doc_lengths: &'a HashMap<u32, u32>,
}

impl<'a> Ranking<'a> {
    pub fn init(
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
    pub fn idf(&self, term: &str) -> f32 {
        // if inv_index doesnt contain term, idf is 0
        let df = self.inv_index.get(term).map_or(0, |map| map.len());
        if df == 0 {
            return 0.0;
        }
        return ((self.num_doc as f32 - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();
    }

    pub fn bm25_weight(&self, doc_id: &u32, term: &str) -> f32 {
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

    pub fn vector_length(&self, weights: &[f32]) -> f32 {
        weights
            .iter()
            .map(|weight| weight.powi(2))
            .sum::<f32>()
            .sqrt()
    }

    pub fn cosine_similarity(&self, doc_id: &u32, query_terms: &TokenizedQuery) -> f32 {
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

    pub fn rank_documents(
        &self,
        queries: &[TokenizedQuery],
    ) -> BTreeMap<u32, BTreeSet<RankingResult>> {
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

pub fn save_results_to_file(results: BTreeMap<u32, BTreeSet<RankingResult>>, file_path: &str) {
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
pub struct RankingResult {
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
