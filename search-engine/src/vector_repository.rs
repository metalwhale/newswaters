use std::env;

use anyhow::Result;
use qdrant_client::{
    prelude::{Payload, QdrantClient},
    qdrant::{
        point_id::PointIdOptions, vectors_config::Config, CreateCollection, Distance, PointId, PointStruct,
        SearchPoints, VectorParams, VectorsConfig,
    },
};

pub(crate) struct VectorRepository {
    client: QdrantClient,
}

impl VectorRepository {
    pub(crate) async fn new() -> Result<Self> {
        let url = format!(
            "http://{}:{}",
            env::var("SEARCH_ENGINE_VECTOR_HOST")?,
            env::var("SEARCH_ENGINE_VECTOR_PORT")?
        );
        let client = QdrantClient::from_url(&url).build()?;
        let collection_names = env::var("SEARCH_ENGINE_VECTOR_COLLECTION_NAMES")?;
        for name in collection_names.split(",") {
            if !client.has_collection(name).await? {
                client
                    .create_collection(&CreateCollection {
                        collection_name: name.to_string(),
                        vectors_config: Some(VectorsConfig {
                            config: Some(Config::Params(VectorParams {
                                size: 768, // See: https://huggingface.co/jinaai/jina-embeddings-v2-base-en/blob/d411fe9/config.json#L18
                                distance: Distance::Cosine.into(),
                                ..Default::default()
                            })),
                        }),
                        ..Default::default()
                    })
                    .await?;
            }
        }
        return Ok(Self { client });
    }

    pub(crate) async fn find_missing(&self, collection_name: String, ids: Vec<i32>) -> Result<Vec<i32>> {
        let points = self
            .client
            .get_points(
                collection_name,
                &ids.iter().map(|i| (*i as u64).into()).collect::<Vec<PointId>>(),
                Some(false),
                Some(false),
                None,
            )
            .await?
            .result;
        let mut existing_ids = vec![];
        for point in points {
            if let Some(PointId {
                point_id_options: Some(PointIdOptions::Num(id)),
            }) = point.id
            {
                existing_ids.push(id as i32)
            }
        }
        let mut missing_ids = vec![];
        for id in ids {
            if existing_ids.contains(&id) {
                continue;
            } else {
                missing_ids.push(id);
            }
        }
        Ok(missing_ids)
    }

    pub(crate) async fn upsert(&self, collection_name: String, id: i32, embedding: Vec<f32>) -> Result<()> {
        let points = vec![PointStruct::new(id as u64, embedding, Payload::new())];
        self.client
            .upsert_points_blocking(collection_name, points, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn search_similar(
        &self,
        collection_name: String,
        embedding: Vec<f32>,
        limit: u64,
    ) -> Result<Vec<(i32, f32)>> {
        let points = self
            .client
            .search_points(&SearchPoints {
                collection_name,
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
