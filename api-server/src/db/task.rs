use protocol::db::{metadata::CallMetadata, task::Task};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct TaskWithMetadata {
    #[sqlx(flatten)]
    pub task: Task,
    #[sqlx(flatten)]
    pub metadata: CallMetadata,
}

impl TaskWithMetadata {
    pub async fn total_count(project_id: Uuid, conn: &mut sqlx::PgConnection) -> sqlx::Result<i64> {
        sqlx::query!(
            r#"
                SELECT COUNT(1) as total
                FROM task
                WHERE project_id = $1
            "#,
            project_id
        )
        .fetch_one(conn)
        .await
        .map(|r| r.total.unwrap_or(0))
    }

    pub async fn tasks_list(
        offset: i64,
        limit: i64,
        order_by: &str, // TODO: possible SQL injection, fix it
        desc: bool,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<TaskWithMetadata>> {
        let desc = if desc { "DESC" } else { "ASC" };

        let query = format!(
            r#"
            SELECT
                call_metadata.id as metadata_id,
                task.id as id,
                call_metadata_id,
                status,
                failed_reason,
                project_id,
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
                inbound
            FROM task
            JOIN call_metadata ON task.call_metadata_id = call_metadata.id
            ORDER BY {order_by} {desc}
            OFFSET {offset}
            LIMIT {limit}
            "#
        );

        sqlx::query_as(&query).fetch_all(conn).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use protocol::{db::task::TaskResultKind, entity::ParticipantKind};
    use uuid::Uuid;

    #[sqlx::test]
    async fn test_tasks_list(pool: sqlx::PgPool) {
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
            status: TaskResultKind::Processing,
        };
        task.insert(&mut conn).await.expect("failed to insert task");

        let tasks = TaskWithMetadata::tasks_list(0, 10, "file_name", false, &mut conn)
            .await
            .expect("failed to retrieve tasks list");
        let count = TaskWithMetadata::total_count(Uuid::default(), &mut conn)
            .await
            .expect("failed to retrieve total count");
        assert!(tasks.len() == count as usize);
    }
}
