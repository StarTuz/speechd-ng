use tokenizers::Tokenizer; fn main() { let t = Tokenizer::from_file("tokenizer.json").unwrap(); println!("<END>: {:?}", t.encode("<END>", false).unwrap().get_ids()); }
