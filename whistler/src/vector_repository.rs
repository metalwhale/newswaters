use std::env;

use anyhow::Result;
use qdrant_client::{
    prelude::QdrantClient,
    qdrant::{point_id::PointIdOptions, PointId, SearchPoints},
};

pub(crate) struct VectorRepository {
    collection_name: String,
    client: QdrantClient,
}

impl VectorRepository {
    pub(crate) async fn new() -> Result<Self> {
        let url = format!(
            "http://{}:{}",
            env::var("VECTOR_DATABASE_HOST")?,
            env::var("VECTOR_DATABASE_PORT")?
        );
        let client = QdrantClient::from_url(&url).build()?;
        let collection_name = env::var("VECTOR_DATABASE_COLLECTION")?;
        return Ok(Self {
            collection_name,
            client,
        });
    }

    pub(crate) async fn search_points(&self, embedding: Vec<f32>, limit: u64) -> Result<Vec<(i32, f32)>> {
        let points = self
            .client
            .search_points(&SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: embedding,
                limit,
                ..Default::default()
            })
            .await?
            .result;
        let mut similar_points = vec![];
        for point in points {
            if let Some(PointId {
                point_id_options: Some(PointIdOptions::Num(id)),
            }) = point.id
            {
                similar_points.push((id as i32, point.score))
            }
        }
        Ok(similar_points)
    }
}
