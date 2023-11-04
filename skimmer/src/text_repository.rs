use anyhow::Result;
use tantivy::{
    directory::MmapDirectory,
    schema::{Field, NumericOptions, Schema, TEXT},
    Index,
};

pub(crate) struct TextRepository {
    id: Field,
    sentence: Field,
    index: Index,
}

impl TextRepository {
    pub(crate) fn new() -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let id = schema_builder.add_u64_field("id", NumericOptions::default().set_fast());
        let sentence = schema_builder.add_text_field("sentence", TEXT);
        let schema = schema_builder.build();
        let index = Index::open_or_create(MmapDirectory::open("/var/lib/tantivy/storage")?, schema)?;
        Ok(Self { id, sentence, index })
    }

    pub(crate) fn add_documents(&self, documents: Vec<(i32, String)>) -> Result<()> {
        let mut index_writer = self.index.writer(100_000_000)?;
        for (id, sentence) in documents {
            index_writer.add_document(doc!(
                self.id => id as u64,
                self.sentence => sentence,
            ))?;
        }
        index_writer.commit()?;
        Ok(())
    }
}
