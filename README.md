
# Name & Tasks

Name: Alexandre Ringuette
Student Number: 300251252

Name: 
Student Number: 

Name: 
Student Number: 

# Functionality

|File Name  | Functionality  |
|---|---|
| indexing.rs  | Contains the functions for index the corpus  |   |
| preprocessing.rs | Contains the functions for preprocessing the text in the corpus and queries|
| ranking.rs | contains the functions for ranking the queries againsts the corpus |

The program was initialy run with the setup of inverted index and queries to build the inverted index and the tokens from the queries. 
Once those have been saved, we no longer needed to run this setup as we just load them in the program while doing the ranking. 
Once the necessary files are loaded, the ranking system can begin running the BM25 score on the inverted index and calculating the cosine similarity with the query. 

# Instructions

- Requires Rust programming language to be installed (see here for installation: https://www.rust-lang.org/tools/install)
- Once installed and in the root directory run the command: `cargo run --release`
- The program will then execute and output the results.tsv file.

# Explanation of Algorithms, Data Structures, and Optimizations

### Preprocessing Stage
The queries and docements are defined as struct in the preprocessing module so it is easier to manipulate them.  
The preprocessing algorithm works as such:
1. Extract the words from the text using a Regex and outpus a list of strings.
2. Remove the stopwords from the list.
3. Stem the words using the Porter Stemmer. 
4. Removes any outlier which only have 1 letter
5. Calculate the frequecy for all the words
6. Returns a map with the words as keys and their frequency as values.

The same process is applied to the queries.

### Indexing Stage
The inverted index is represent as a map of string as keys and another map of int as keys and int as values (Map<String, Map<u32, u16>>)
This allows us to store for a given token, each document with the frequency of that token for more accurate scores.
The indexing algorithm works as such: 
1. After loading the corpus and stopwords, iterate through the corpus line by line and use the preprocessing algorithm on the text and title.
2. Combine the tokens from the text and title and pack them in a struct called `TokenizedDocument` whichs holds the document id and the tokens for that document and store in a list for now.
3. After all documents have been processed, pass the list to the function `build_inverted_index` which iterates through the list and builds the map storing the tokens as keys and the inserting documents that contain the token with the frequency. 

The queries are also stored in a file as a map of query id as keys and their tokens as values.


### Ranking Stage
The ranking struct holds values for the BM25 formula; k1, b, avgdl & number of documents. It also stores a reference to the inverted index and a map of document lengths. This struct contains functions including calculating the idc, bm25 weight, vector length, cosine similarity and the ranking algorithm. 
The ranking algorithm works as such:
1. Iterate through the queries, then iterate through each term in that query
2. if the term is in the inverted index, fetch the documents map
3. iterate through the documents map and calculate the cosine similarity between the document and the query
4. store the result in a BTreeSet and insert into a BTreeMap to link query id to the set. We use a BTree here to keep it sorted on insert and remove the first element once we exceed 100 since first element is alwasy the smallest.
5. return the BtreeMap

### Query Test Results

Here are the results of the top 10 answers of the first 2 queries id 0 & 1.

0  Q0  13231899  1  0.7750401  4843291
0  Q0  43385013  2  0.5003812  1441973
0  Q0  27049238  3  0.5003811  1883414
0  Q0  6550579  4  0.48893997  6550579
0  Q0  13903052  5  0.479567  5514444
0  Q0  26083387  6  0.47956696  917563
0  Q0  26071782  7  0.47616866  905958
0  Q0  34386619  8  0.46710527  832187
0  Q0  1203035  9  0.41149545  1203035
0  Q0  20155713  10  0.41149542  3378497
0  Q0  6327940  11  0.4114954  6327940

1  Q0  13231899  1  0.8019935  4843292
1  Q0  27049238  2  0.5177828  1883415
1  Q0  40212412  3  0.51778275  6657981
1  Q0  6550579  4  0.5059437  6550580
1  Q0  34386619  5  0.48334968  832188
1  Q0  18953920  6  0.42580593  2176705
1  Q0  20155713  7  0.4258059  3378498
1  Q0  21257564  8  0.4246468  4480349
1  Q0  18276599  9  0.41892213  1499384
1  Q0  41928290  10  0.4189221  8373859
1  Q0  14275671  11  0.41892207  5887064




The vocabulary consistes of 20028 tokens. See vocab_sample.txt for a sample of 100 tokens.

# Mean Average Precision
Running the follow command to calculate the Mean Average Precision (MAP) using the trec_eval script: </br>
`./trec_eval -m map ../../Assignment1/scifact/qrels/test.tsv ../../Assignment1/saved/results.tsv`

Outputs: 
`map                     all     0.5162`
</br>
The MAP is 0.5162 compared to the test.tsv file.

</br>

Running the system on the inverted index only collecting the title tokens, the Mean Average Presicions of the queires is: 

`map                     all     0.3785`

We can conclude that including the text tokens greatly increases the performance of the system.



