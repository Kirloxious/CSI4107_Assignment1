
# Name & Tasks

Name: Alexandre Ringuette
Student Number: 300251252

Name: 
Student Number: 

Name: 
Student Number: 

# Functionality



# Instructions

- Insure Rust programming language is install (https://www.rust-lang.org/tools/install)
- Once installed and in the root directory run the command: `cargo run --release`
- The program will then execute and output the results.tsv file

# Explanation of Algorithms, Data Structures, and Optimizations


The vocabulary consiste of 6935 tokens. See vocab_sample for a sample of 100 tokens.

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



