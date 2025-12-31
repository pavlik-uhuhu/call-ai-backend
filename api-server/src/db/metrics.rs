use protocol::db::{metadata::CallMetadata, metrics::CallMetrics};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, sqlx::FromRow, ToSchema)]
pub struct MetricsWithMetadata {
    #[sqlx(flatten)]
    pub metadata: CallMetadata,
    #[sqlx(flatten)]
    pub metrics: CallMetrics,
}

impl MetricsWithMetadata {
    pub async fn total_count(project_id: Uuid, conn: &mut sqlx::PgConnection) -> sqlx::Result<i64> {
        sqlx::query!(
            r#"
                SELECT COUNT(1) as total
                FROM task_call_metrics
                JOIN task ON task.id = task_call_metrics.task_id
                WHERE project_id = $1
            "#,
            project_id
        )
        .fetch_one(conn)
        .await
        .map(|r| r.total.unwrap_or(0))
    }

    pub async fn metrics_list(
        offset: i64,
        limit: i64,
        order_by: &str, // TODO: possible SQL injection, fix it
        desc: bool,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<MetricsWithMetadata>> {
        let desc = if desc { "DESC" } else { "ASC" };

        let query = format!(
            r#"
            SELECT
               call_metadata.id as metadata_id,
               task_id,
               call_id,
               performed_at,
               uploaded_at,
               file_hash,
               file_url,
               file_name,
               duration,
               left_channel,
               right_channel,
               client_name,
               employee_name,
               inbound,
               call_duration,
               time_to_answer,
               total_employee_speech,
               total_client_speech,
               employee_client_speech_ratio,
               employee_speech_ratio,
               client_speech_ratio,
               call_holds_count,
               silence_pause_count,
               total_employee_silence,
               client_interruptions_count,
               total_client_interruptions_duration,
               avg_employee_words_per_min,
               avg_client_words_per_min,
               script_score,
               employee_quality_score,
               emotion_mode,
               emotion_start_mode,
               emotion_end_mode
            FROM call_metadata
            JOIN task ON task.call_metadata_id = call_metadata.id
            JOIN task_call_metrics ON task.id = task_call_metrics.task_id
            ORDER BY {order_by} {desc}
            OFFSET {offset}
            LIMIT {limit}
            "#
        );

        sqlx::query_as(&query).fetch_all(conn).await
    }

    pub async fn fetch_by_task_id(
        task_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Option<MetricsWithMetadata>> {
        let query = format!(
            r#"
            SELECT
               call_metadata.id as metadata_id,
               task_id,
               call_id,
               performed_at,
               uploaded_at,
               file_hash,
               file_url,
               file_name,
               duration,
               left_channel,
               right_channel,
               client_name,
               employee_name,
               inbound,
               call_duration,
               time_to_answer,
               total_employee_speech,
               total_client_speech,
               employee_client_speech_ratio,
               employee_speech_ratio,
               client_speech_ratio,
               call_holds_count,
               silence_pause_count,
               total_employee_silence,
               client_interruptions_count,
               total_client_interruptions_duration,
               avg_employee_words_per_min,
               avg_client_words_per_min,
               script_score,
               employee_quality_score,
               emotion_mode,
               emotion_start_mode,
               emotion_end_mode
            FROM call_metadata
            JOIN task ON task.call_metadata_id = call_metadata.id
            JOIN task_call_metrics ON task.id = task_call_metrics.task_id
            WHERE task_call_metrics.task_id = '{task_id}'
            "#
        );

        sqlx::query_as(&query).fetch_optional(conn).await
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use protocol::{
        db::{
            metadata::CallMetadata,
            metrics::CallMetrics,
            task::{Task, TaskResultKind},
        },
        entity::{speech_recog::EmotionKind, ParticipantKind},
    };
    use uuid::Uuid;

    use super::*;

    #[sqlx::test]
    async fn test_metrics_fetch(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let metadata = CallMetadata {
            metadata_id: Uuid::default(),
            call_id: 11i64,
            performed_at: Utc::now(),
            uploaded_at: Utc::now(),
            file_hash: Uuid::new_v4().hyphenated().to_string(),
            file_url: "s3://test_bucket/test.mp3".to_string(),
            file_name: "test.mp3".to_string(),
            duration: 15.0,
            left_channel: ParticipantKind::Client,
            right_channel: ParticipantKind::Employee,
            client_name: "test_client".to_string(),
            employee_name: "test_agent".to_string(),
            inbound: true,
        };
        let metadata_id = metadata
            .insert(&mut conn)
            .await
            .expect("failed to insert metadata")
            .metadata_id;

        let task = Task {
            id: Uuid::default(),
            call_metadata_id: metadata_id,
            failed_reason: None,
            project_id: Uuid::default(),
            status: TaskResultKind::Ready,
        };
        let task_id = task
            .insert(&mut conn)
            .await
            .expect("failed to insert task")
            .id;

        let mut metrics = CallMetrics::default();
        metrics.task_id = task_id;
        metrics.emotion_mode = Some(EmotionKind::Sad);
        let _ = CallMetrics::insert(metrics, &mut conn).await.unwrap();

        let metrics = MetricsWithMetadata::metrics_list(0, 10, "file_name", false, &mut conn)
            .await
            .expect("failed to retrieve tasks list");
        let count = MetricsWithMetadata::total_count(Uuid::default(), &mut conn)
            .await
            .expect("failed to retrieve total count");
        assert!(metrics.len() == count as usize);

        let metrics = MetricsWithMetadata::fetch_by_task_id(task_id, &mut conn)
            .await
            .expect("failed to retrieve single row");
        assert!(metrics.is_some());
    }
}
