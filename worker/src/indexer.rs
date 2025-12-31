use std::sync::Arc;

use async_trait::async_trait;
use axum::{body::Bytes, response::IntoResponse, Json};
use http::StatusCode;
#[cfg(test)]
use mockall::{automock, predicate::*};
use protocol::entity::{speech_recog::RecognitionData, ParticipantKind};
use tantivy::{
    collector::TopDocs,
    directory::{error::OpenDirectoryError, MmapDirectory, RamDirectory},
    doc,
    query::{BooleanQuery, Occur, PhraseQuery, Query, TermQuery},
    schema::{
        document::Value, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, STORED, STRING,
    },
    tokenizer::{LowerCaser, SimpleTokenizer, TextAnalyzer},
    Directory, Index, IndexReader, IndexWriter, TantivyDocument, TantivyError, Term,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Indexer error: {0}")]
    Index(#[source] TantivyError),
    #[error("Failed to open index directory error: {0}")]
    OpenDirectory(#[source] OpenDirectoryError),
    #[error("Indexer serialize error: {0}")]
    Ser(#[source] serde_json::Error),
    #[error("Indexer async block waiting error: {0}")]
    TaskJoin(#[source] tokio::task::JoinError),
    #[error("Requested transcript not found for doc id: {0}")]
    TranscriptNotFound(Uuid),
    #[error("Payload extraction error for doc id: {0}")]
    Payload(Uuid),
}

impl IntoResponse for IndexerError {
    fn into_response(self) -> axum::response::Response {
        error!("Service Error {}", self);

        let body = Json(serde_json::json!({"error": format!("{self}")}));
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Indexer {
    async fn index_speech_recog(
        &self,
        id: Uuid,
        recog_data: &RecognitionData,
    ) -> Result<(), IndexerError>;

    async fn search_phrase(
        &self,
        id: Uuid,
        phrase: &str,
        speaker: &ParticipantKind,
    ) -> Result<bool, IndexerError>;

    async fn load_transcript_payload(&self, id: Uuid) -> Result<Bytes, IndexerError>;
}

#[derive(Clone)]
pub struct TantivyIndexer {
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
}

const CLIENT_TRANSCRIPT_FIELD: &str = "client_trancript";
const EMPLOYEE_TRANSCRIPT_FIELD: &str = "employee_transcript";
const PAYLOAD_FIELD: &str = "payload";
const UUID_FIELD: &str = "uuid";

impl TantivyIndexer {
    pub fn new(index_path: &str) -> Result<Self, IndexerError> {
        let create_dir_res = std::fs::create_dir(index_path);
        info!("crating index dir: {:?}", create_dir_res);

        let mut schema_builder = Schema::builder();

        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("custom_tokenizer")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default().set_indexing_options(text_field_indexing);

        schema_builder.add_text_field(CLIENT_TRANSCRIPT_FIELD, text_options.clone());
        schema_builder.add_text_field(EMPLOYEE_TRANSCRIPT_FIELD, text_options);
        schema_builder.add_text_field(UUID_FIELD, STRING);
        schema_builder.add_bytes_field(PAYLOAD_FIELD, STORED);

        let schema = schema_builder.build();

        let dir = if cfg!(test) {
            Box::new(RamDirectory::create()) as Box<dyn Directory>
        } else {
            Box::new(MmapDirectory::open(index_path).map_err(IndexerError::OpenDirectory)?)
                as Box<dyn Directory>
        };

        let index = Index::open_or_create(dir, schema).map_err(IndexerError::Index)?;

        let tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .build();
        index.tokenizers().register("custom_tokenizer", tokenizer);

        let index_writer: IndexWriter = index.writer(150_000_000).map_err(IndexerError::Index)?;
        let reader_builder = index.reader_builder();
        let reader = reader_builder
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .map_err(IndexerError::Index)?;

        Ok(Self {
            reader,
            writer: Arc::new(Mutex::new(index_writer)),
        })
    }
}

#[async_trait]
impl Indexer for TantivyIndexer {
    async fn index_speech_recog(
        &self,
        id: Uuid,
        recog_data: &RecognitionData,
    ) -> Result<(), IndexerError> {
        let searcher = self.reader.searcher();
        let schema = searcher.schema();

        let client_transcript_field = schema
            .get_field(CLIENT_TRANSCRIPT_FIELD)
            .map_err(IndexerError::Index)?;
        let employee_transcript_field = schema
            .get_field(EMPLOYEE_TRANSCRIPT_FIELD)
            .map_err(IndexerError::Index)?;
        let id_field = schema.get_field(UUID_FIELD).map_err(IndexerError::Index)?;
        let payload_field = schema
            .get_field(PAYLOAD_FIELD)
            .map_err(IndexerError::Index)?;

        let payload_to_bytes = serde_json::to_vec(&recog_data).map_err(IndexerError::Ser)?;
        let client_transcript = recog_data
            .speech_recognition_result
            .iter()
            .filter(|recog| recog.speaker == ParticipantKind::Client)
            .fold("".to_string(), |cur, next| cur + " " + &next.text);
        let employee_transcript = recog_data
            .speech_recognition_result
            .iter()
            .filter(|recog| recog.speaker == ParticipantKind::Employee)
            .fold("".to_string(), |cur, next| cur + " " + &next.text);

        let mut index_writer = self.writer.clone().lock_owned().await;
        let reader = self.reader.clone();

        tokio::task::spawn_blocking(move || {
            index_writer
                .add_document(doc!(
                        client_transcript_field => client_transcript,
                        employee_transcript_field => employee_transcript,
                        id_field => id.to_string(),
                        payload_field => payload_to_bytes,
                ))
                .map_err(IndexerError::Index)?;

            index_writer
                .commit()
                .map(|_| ())
                .map_err(IndexerError::Index)?;

            reader.reload().map_err(IndexerError::Index)
        })
        .await
        .map_err(IndexerError::TaskJoin)?
    }

    async fn search_phrase(
        &self,
        id: Uuid,
        phrase: &str,
        speaker: &ParticipantKind,
    ) -> Result<bool, IndexerError> {
        let searcher = self.reader.searcher();
        let schema = searcher.schema();

        let id_field = schema.get_field(UUID_FIELD).map_err(IndexerError::Index)?;
        let transcript_field = if speaker == &ParticipantKind::Client {
            schema
                .get_field(CLIENT_TRANSCRIPT_FIELD)
                .map_err(IndexerError::Index)?
        } else {
            schema
                .get_field(EMPLOYEE_TRANSCRIPT_FIELD)
                .map_err(IndexerError::Index)?
        };

        let words: Vec<&str> = phrase.split_whitespace().collect();
        let query = if words.len() > 1 {
            let terms = words
                .into_iter()
                .map(|word| Term::from_field_text(transcript_field, word))
                .collect();
            Box::new(PhraseQuery::new(terms)) as Box<dyn Query>
        } else {
            Box::new(TermQuery::new(
                Term::from_field_text(transcript_field, words.first().expect("non empty vec")),
                IndexRecordOption::Basic,
            )) as Box<dyn Query>
        };

        let nested_query = BooleanQuery::new(vec![
            (Occur::Must, query),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(id_field, &id.to_string()),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);

        let top_docs = searcher
            .search(&nested_query, &TopDocs::with_limit(1))
            .map_err(IndexerError::Index)?;

        Ok(!top_docs.is_empty())
    }

    async fn load_transcript_payload(&self, id: Uuid) -> Result<Bytes, IndexerError> {
        let searcher = self.reader.searcher();
        let schema = searcher.schema();

        let id_field = schema.get_field(UUID_FIELD).map_err(IndexerError::Index)?;
        let payload_field = schema
            .get_field(PAYLOAD_FIELD)
            .map_err(IndexerError::Index)?;

        let query = TermQuery::new(
            Term::from_field_text(id_field, &id.to_string()),
            IndexRecordOption::Basic,
        );

        let mut top_docs = searcher
            .search(&query, &TopDocs::with_limit(1))
            .map_err(IndexerError::Index)?;

        let (_, doc_address) = top_docs.pop().ok_or(IndexerError::TranscriptNotFound(id))?;

        let retrieved_doc: TantivyDocument =
            searcher.doc(doc_address).map_err(IndexerError::Index)?;

        let payload = retrieved_doc
            .get_first(payload_field)
            .ok_or(IndexerError::Payload(id))?;

        let payload = payload.as_bytes().ok_or(IndexerError::Payload(id))?;

        Ok(Bytes::copy_from_slice(payload))
    }
}
