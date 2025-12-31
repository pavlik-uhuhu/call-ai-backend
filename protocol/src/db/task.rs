use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "task_result_status", rename_all = "snake_case")]
pub enum TaskResultKind {
    Processing,
    Ready,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Task {
    pub id: Uuid,
    pub call_metadata_id: Uuid,
    pub status: TaskResultKind,
    pub failed_reason: Option<String>,
    #[serde(skip_deserializing)]
    pub project_id: Uuid,
}

impl Task {
    pub async fn insert(&self, conn: &mut sqlx::PgConnection) -> sqlx::Result<Task> {
        sqlx::query_as!(
            Task,
            r#"
                INSERT INTO task
                    (call_metadata_id, status, project_id)
                VALUES ($1, $2::task_result_status, $3)
                RETURNING
                    id,
                    call_metadata_id,
                    status as "status: TaskResultKind",
                    failed_reason,
                    project_id
            "#,
            self.call_metadata_id,
            self.status as TaskResultKind,
            self.project_id
        )
        .fetch_one(conn)
        .await
    }

    pub async fn get(id: &Uuid, conn: &mut sqlx::PgConnection) -> sqlx::Result<Task> {
        sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                call_metadata_id,
                status as "status: TaskResultKind",
                failed_reason,
                project_id
            FROM task
            WHERE id = $1
            "#,
            id,
        )
        .fetch_one(conn)
        .await
    }

    pub async fn update(&self, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        sqlx::query_as!(
            Task,
            r#"
                UPDATE task
                SET 
                    status = $2, 
                    failed_reason = $3
                WHERE 
                    id = $1
            "#,
            self.id,
            self.status as TaskResultKind,
            self.failed_reason
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskToDict {
    pub task_id: Uuid,
    pub dictionary_id: i32,
    pub contains: bool,
}

impl TaskToDict {
    pub async fn insert(this: Self, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO task_to_dict
                    (task_id, dictionary_id, contains)
                VALUES ($1, $2, $3)
            "#,
            this.task_id,
            this.dictionary_id,
            this.contains
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn bulk_insert(this: Vec<Self>, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        let mut task_ids = Vec::new();
        let mut dict_ids = Vec::new();
        let mut contains = Vec::new();
        this.into_iter().for_each(|item| {
            task_ids.push(item.task_id);
            dict_ids.push(item.dictionary_id);
            contains.push(item.contains);
        });

        sqlx::query!(
            r#"
                INSERT INTO task_to_dict
                    (task_id, dictionary_id, contains)
                SELECT task_id, dictionary_id, contains
                FROM UNNEST($1::uuid[], $2::int[], $3::bool[]) as a(task_id, dictionary_id, contains)
            "#,
            &task_ids,
            &dict_ids,
            &contains
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn list_by_task_id(
        task_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            TaskToDict,
            r#"
                SELECT task_id, dictionary_id, contains 
                FROM task_to_dict
                WHERE task_id = $1
            "#,
            task_id,
        )
        .fetch_all(conn)
        .await
    }
}
