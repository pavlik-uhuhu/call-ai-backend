use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entity::ParticipantKind;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CallMetadata {
    #[serde(skip_deserializing)]
    #[sqlx(default)]
    pub metadata_id: Uuid,
    pub call_id: i64,

    #[serde(with = "ts_milliseconds")]
    #[schema(value_type = i64)]
    pub performed_at: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    #[schema(value_type = i64)]
    pub uploaded_at: DateTime<Utc>,

    pub file_hash: String,
    pub file_url: String,
    pub file_name: String,

    pub duration: f32,
    pub left_channel: ParticipantKind,
    pub right_channel: ParticipantKind,
    pub client_name: String,
    pub employee_name: String,
    pub inbound: bool,
}

impl CallMetadata {
    pub async fn get_by_task_id(
        task_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<CallMetadata> {
        sqlx::query_as!(
            CallMetadata,
            r#"
            SELECT                 
                call_metadata.id as metadata_id,
                call_id,
                performed_at,
                uploaded_at,
                file_hash,
                file_url,
                file_name,
                duration,
                left_channel as "left_channel: ParticipantKind",
                right_channel as "right_channel: ParticipantKind",
                client_name,
                employee_name,
                inbound
            FROM call_metadata
            JOIN task ON task.call_metadata_id = call_metadata.id
            WHERE task.id = $1
            "#,
            task_id,
        )
        .fetch_one(conn)
        .await
    }

    pub async fn insert(&self, conn: &mut sqlx::PgConnection) -> sqlx::Result<CallMetadata> {
        sqlx::query_as!(
            CallMetadata,
            r#"
            INSERT INTO call_metadata (
                call_id,
                performed_at, uploaded_at, 
                file_hash, file_url, file_name, 
                duration, 
                left_channel, right_channel, 
                client_name, employee_name, 
                inbound
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::participant_type, $9::participant_type, $10, $11, $12)
            RETURNING
                id as metadata_id,
                call_id,
                performed_at,
                uploaded_at,
                file_hash,
                file_url,
                file_name,
                duration,
                left_channel as "left_channel: ParticipantKind",
                right_channel as "right_channel: ParticipantKind",
                client_name,
                employee_name,
                inbound
            "#,
            self.call_id,
            self.performed_at,
            self.uploaded_at,
            self.file_hash,
            self.file_url,
            self.file_name,
            self.duration,
            self.left_channel as ParticipantKind,
            self.right_channel as ParticipantKind,
            self.client_name,
            self.employee_name,
            self.inbound
        )
        .fetch_one(conn)
        .await
    }
}
