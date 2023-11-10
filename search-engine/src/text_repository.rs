use std::env;

use anyhow::Result;
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{Field, Schema, Value, FAST, STORED, TEXT},
    Index,
};

#[derive(Clone)]
pub(crate) struct TextRepository {
    id: Field,
    sentence: Field,
    index: Index,
}

impl TextRepository {
    pub(crate) fn new() -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let id = schema_builder.add_u64_field("id", FAST | STORED);
        let sentence = schema_builder.add_text_field("sentence", TEXT);
        let schema = schema_builder.build();
        let storage_path = env::var("SEARCH_ENGINE_TEXT_STORAGE_PATH")?;
        std::fs::create_dir_all(&storage_path)?;
        let index = Index::open_or_create(MmapDirectory::open(storage_path)?, schema)?;
        Ok(Self { id, sentence, index })
    }

    #[allow(dead_code)]
    pub(crate) fn add(&self, id: i32, sentence: String) -> Result<()> {
        let mut index_writer = self.index.writer(100_000_000)?;
        index_writer.add_document(doc!(
            self.id => id as u64,
            self.sentence => sentence,
        ))?;
        index_writer.commit()?;
        Ok(())
    }

    pub(crate) fn search_similar(&self, sentence: String, limit: usize) -> Result<Vec<(i32, f32)>> {
        let query_parser = QueryParser::for_index(&self.index, vec![self.sentence]);
        let query = query_parser.parse_query(&sentence)?;
        let searcher = self.index.reader()?.searcher();
        let docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        let mut similar_docs = vec![];
        for (score, address) in docs {
            let doc = searcher.doc(address)?;
            if let Some(Value::U64(id)) = doc.get_first(self.id) {
                similar_docs.push((*id as i32, score))
            }
        }
        Ok(similar_docs)
    }
}
