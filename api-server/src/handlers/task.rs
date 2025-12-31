use axum::extract::{Path, Query};
use axum::{extract::State, Json};
use http::StatusCode;
use protocol::db::{
    metadata::CallMetadata,
    settings::{Settings, SettingsDictItem, SettingsItem},
    task::{Task, TaskResultKind, TaskToDict},
};
use protocol::entity::settings_metrics::{self, TaskSettingsMetrics};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::context::{AppContext, Context, TaskPublisher};
use crate::db::{metrics::MetricsWithMetadata, task::TaskWithMetadata};
use crate::error::{Error, ErrorExt, ErrorKind};
use crate::handlers::utils::{AppResponse, RequestResult};

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TaskCreateRequest {
    metadata: CallMetadata,
    #[serde(skip_deserializing)]
    _project_id: Uuid,
}

#[derive(OpenApi)]
#[openapi(
    paths(create, reprocess, list, metrics_list, detailed_metrics),
    components(schemas(TaskCreateRequest, TaskListResponse, MetricsListResponse, TaskDetailedMetrics)),
    tags(
        (name = "Tasks", description = "API to handle tasks and metrics")
    )
)]
pub(super) struct ApiTasks;

#[utoipa::path(
    post,
    operation_id = "task_create",
    path = "",
    request_body = TaskCreateRequest,
    responses(
        (status = CREATED, description = "Task created successfully", body = Task),
        (status = BAD_REQUEST, description = "File with the same hash already exists"),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to create task")
    ),
    tags = ["Tasks"]
)]
pub async fn create(
    State(cx): State<AppContext>,
    Json(request): Json<TaskCreateRequest>,
) -> RequestResult<Task> {
    do_create(cx, request).await
}

async fn do_create<C: Context>(cx: C, request: TaskCreateRequest) -> RequestResult<Task> {
    let stored_metadata = {
        let mut conn = cx.get_db_conn().await?;
        let res = request.metadata.insert(&mut conn).await;
        match res {
            Err(sqlx::Error::Database(db_err))
                if db_err.kind() == sqlx::error::ErrorKind::UniqueViolation =>
            {
                return Err(Error::new(
                    ErrorKind::FileAlredyExists,
                    anyhow::anyhow!(
                        "file {} with hash {} already exists",
                        request.metadata.file_name,
                        request.metadata.file_hash
                    ),
                ))
            }
            Err(err) => return Err(err.into()),
            Ok(res) => res,
        }
    };

    let stored_task = {
        let mut conn = cx.get_db_conn().await?;
        let task = Task {
            id: Uuid::default(),
            call_metadata_id: stored_metadata.metadata_id,
            status: TaskResultKind::Processing,
            failed_reason: None,
            project_id: request._project_id,
        };

        task.insert(&mut conn).await?
    };

    cx.publisher().publish(&stored_task.id).await?;

    Ok(AppResponse::new(StatusCode::CREATED, stored_task))
}

#[utoipa::path(
    put,
    operation_id = "task_recreate",
    path = "/{task_id}",
    responses(
        (status = OK, description = "Task reprocessed successfully", body = Task),
        (status = NOT_FOUND, description = "Task not found"),
        (status = BAD_REQUEST, description = "Task is already processing")
    ),
    params(
        ("task_id" = Uuid, Path, description = "Unique identifier for the task")
    ),
    tags = ["Tasks"]
)]
pub async fn reprocess(
    State(cx): State<AppContext>,
    Path(task_id): Path<Uuid>,
) -> RequestResult<Task> {
    do_reprocess(cx, task_id).await
}

async fn do_reprocess<C: Context>(cx: C, task_id: Uuid) -> RequestResult<Task> {
    let mut stored_task = {
        let mut conn = cx.get_db_conn().await?;
        Task::get(&task_id, &mut conn)
            .await
            .error(ErrorKind::EntityNotFound)?
    };
    if stored_task.status == TaskResultKind::Processing {
        return Err(Error::new(
            ErrorKind::TaskAlreadyProcessing,
            anyhow::anyhow!("task {task_id} already processing"),
        ));
    }

    stored_task.status = TaskResultKind::Processing;

    let stored_task = {
        let mut conn = cx.get_db_conn().await?;

        stored_task.insert(&mut conn).await?
    };

    cx.publisher().publish(&stored_task.id).await?;

    Ok(AppResponse::new(StatusCode::OK, stored_task))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TaskListRequest {
    #[serde(skip_deserializing)]
    _project_id: Uuid,
    offset: i64,
    limit: i64,
    order_by: String,
    desc: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TaskListResponse {
    items: Vec<TaskWithMetadata>,
    total_count: i64,
}

#[utoipa::path(
    get,
    operation_id = "task_list",
    path = "",
    params(
        TaskListRequest
    ),
    responses(
        (status = OK, description = "List of tasks with metadata", body = TaskListResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to retrieve tasks list")
    ),
    tags = ["Tasks"]
)]
pub async fn list(
    State(cx): State<AppContext>,
    Query(request): Query<TaskListRequest>,
) -> RequestResult<TaskListResponse> {
    do_list(cx, request).await
}

async fn do_list<C: Context>(cx: C, request: TaskListRequest) -> RequestResult<TaskListResponse> {
    let mut conn = cx.get_db_conn().await?;
    let items = TaskWithMetadata::tasks_list(
        request.offset,
        request.limit,
        &request.order_by,
        request.desc,
        &mut conn,
    )
    .await?;
    let total_count = TaskWithMetadata::total_count(Uuid::default(), &mut conn).await?;

    Ok(AppResponse::new(
        StatusCode::OK,
        TaskListResponse { items, total_count },
    ))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MetricsListResponse {
    items: Vec<MetricsWithMetadata>,
    total_count: i64,
}

#[utoipa::path(
    get,
    operation_id = "metrics_list",
    path = "/metrics",
    params(
        TaskListRequest
    ),
    responses(
        (status = OK, description = "List of metrics with metadata", body = MetricsListResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to retrieve metrics list")
    ),
    tags = ["Tasks"]
)]
pub async fn metrics_list(
    State(cx): State<AppContext>,
    Query(request): Query<TaskListRequest>,
) -> RequestResult<MetricsListResponse> {
    do_metrics_list(cx, request).await
}

async fn do_metrics_list<C: Context>(
    cx: C,
    request: TaskListRequest,
) -> RequestResult<MetricsListResponse> {
    let mut conn = cx.get_db_conn().await?;
    let items = MetricsWithMetadata::metrics_list(
        request.offset,
        request.limit,
        &request.order_by,
        request.desc,
        &mut conn,
    )
    .await?;
    let total_count = MetricsWithMetadata::total_count(Uuid::default(), &mut conn).await?;

    Ok(AppResponse::new(
        StatusCode::OK,
        MetricsListResponse { items, total_count },
    ))
}

#[derive(Debug, PartialEq, Serialize, ToSchema)]
pub struct TaskDetailedMetrics {
    #[serde(flatten)]
    nested: MetricsWithMetadata,
    efficiency_metrics: Vec<TaskSettingsMetrics>,
}

#[utoipa::path(
    get,
    path = "/{task_id}/detailed_metrics",
    responses(
        (status = OK, description = "Detailed metrics for the specified task", body = TaskDetailedMetrics),
        (status = NOT_FOUND, description = "Metrics not found"),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to retrieve detailed metrics")
    ),
    params(
        ("task_id" = Uuid, Path, description = "Unique identifier for the task")
    ),
    tags = ["Tasks"]
)]
pub async fn detailed_metrics(
    State(cx): State<AppContext>,
    Path(task_id): Path<Uuid>,
) -> RequestResult<TaskDetailedMetrics> {
    do_detailed_metrics(cx, task_id, Uuid::default()).await
}

async fn do_detailed_metrics<C: Context>(
    cx: C,
    task_id: Uuid,
    project_id: Uuid,
) -> RequestResult<TaskDetailedMetrics> {
    let mut conn = cx.get_db_conn().await?;
    let task_to_dicts = TaskToDict::list_by_task_id(task_id, &mut conn).await?;
    let mut call_metrics = MetricsWithMetadata::fetch_by_task_id(task_id, &mut conn)
        .await?
        .ok_or(Error::new(
            ErrorKind::EntityNotFound,
            anyhow::anyhow!("metrics by task id {task_id} not found"),
        ))?;
    let settings = Settings::list_by_project_id(project_id, &mut conn).await?;
    let settings_items = SettingsItem::list_by_project_id(project_id, &mut conn).await?;
    let settings_dict_items = SettingsDictItem::list_by_project_id(project_id, &mut conn).await?;
    let task_settings_metrics = settings_metrics::calculate_settings_metrics(
        task_to_dicts,
        &mut call_metrics.metrics,
        settings,
        settings_items,
        settings_dict_items,
    )
    .error(ErrorKind::CalcMetricsFailed)?;

    Ok(AppResponse::new(
        StatusCode::OK,
        TaskDetailedMetrics {
            nested: call_metrics,
            efficiency_metrics: task_settings_metrics,
        },
    ))
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use protocol::{
        db::{
            dictionary::{Dictionary, Phrase},
            metrics::CallMetrics,
            settings::{SettingsItemKind, SettingsKind},
        },
        entity::ParticipantKind,
    };
    use settings_metrics::TaskSettingsItemMetric;

    use crate::test_helpers::context::TestContext;

    use super::*;

    #[sqlx::test]
    async fn task_create(pool: sqlx::PgPool) {
        let cx = TestContext::new(pool).await;
        let request = TaskCreateRequest {
            metadata: CallMetadata {
                metadata_id: Uuid::default(),
                call_id: 42,
                performed_at: DateTime::default(),
                uploaded_at: DateTime::default(),
                file_hash: "test_hash".to_string(),
                file_url: "s3://test.mp3".to_string(),
                file_name: "test.mp3".to_string(),
                duration: 100.0,
                left_channel: ParticipantKind::Client,
                right_channel: ParticipantKind::Employee,
                client_name: "test_client".to_string(),
                employee_name: "test_operator".to_string(),
                inbound: true,
            },
            _project_id: Uuid::default(),
        };

        let task_resp = do_create(cx.clone(), request.clone())
            .await
            .expect("failed to create task");
        assert_eq!(task_resp.status(), StatusCode::CREATED);
        let task = task_resp.payload();
        assert_eq!(task.status, TaskResultKind::Processing);

        let task_resp = do_create(cx.clone(), request.clone())
            .await
            .expect_err("unexpected success while creating task");
        assert_eq!(task_resp.kind, ErrorKind::FileAlredyExists);

        let published = cx.test_publisher().flush().await;
        assert_eq!(published, vec![serde_json::json!(task.id)]);
    }

    #[sqlx::test]
    async fn task_list(pool: sqlx::PgPool) {
        let cx = TestContext::new(pool).await;
        let mut metadata = CallMetadata {
            metadata_id: Uuid::default(),
            call_id: 42,
            performed_at: DateTime::default(),
            uploaded_at: DateTime::default(),
            file_hash: "test_hash".to_string(),
            file_url: "s3://test.mp3".to_string(),
            file_name: "test.mp3".to_string(),
            duration: 100.0,
            left_channel: ParticipantKind::Client,
            right_channel: ParticipantKind::Employee,
            client_name: "test_client".to_string(),
            employee_name: "test_operator".to_string(),
            inbound: true,
        };
        let request = TaskCreateRequest {
            metadata: metadata.clone(),
            _project_id: Uuid::default(),
        };

        let task_resp = do_create(cx.clone(), request.clone())
            .await
            .expect("failed to create task");
        assert_eq!(task_resp.status(), StatusCode::CREATED);
        let task = task_resp.payload();
        assert_eq!(task.status, TaskResultKind::Processing);

        let list_response = do_list(
            cx,
            TaskListRequest {
                _project_id: Uuid::default(),
                offset: 0,
                limit: 10,
                order_by: "file_name".to_string(),
                desc: true,
            },
        )
        .await
        .expect("failed to retrieve tasks list");

        assert_eq!(list_response.payload().total_count, 1);

        metadata.metadata_id = task.call_metadata_id;
        assert_eq!(
            vec![TaskWithMetadata {
                task: task.clone(),
                metadata
            }],
            list_response.payload().items
        );
    }

    #[sqlx::test]
    async fn detailed_metrics(pool: sqlx::PgPool) {
        let cx = TestContext::new(pool.clone()).await;
        let project_id = Uuid::new_v4();
        let mut metadata = CallMetadata {
            metadata_id: Uuid::default(),
            call_id: 42,
            performed_at: DateTime::default(),
            uploaded_at: DateTime::default(),
            file_hash: "test_hash".to_string(),
            file_url: "s3://test.mp3".to_string(),
            file_name: "test.mp3".to_string(),
            duration: 100.0,
            left_channel: ParticipantKind::Client,
            right_channel: ParticipantKind::Employee,
            client_name: "test_client".to_string(),
            employee_name: "test_operator".to_string(),
            inbound: true,
        };
        let request = TaskCreateRequest {
            metadata: metadata.clone(),
            _project_id: project_id,
        };

        let task_resp = do_create(cx.clone(), request.clone())
            .await
            .expect("failed to create task");
        assert_eq!(task_resp.status(), StatusCode::CREATED);
        let task = task_resp.payload();
        metadata.metadata_id = task.call_metadata_id;

        let mut conn = pool.acquire().await.unwrap();
        let dict_to_create = {
            let dict =
                Dictionary::insert("test_dict".to_owned(), ParticipantKind::Employee, &mut conn)
                    .await
                    .unwrap();
            let phrases = vec![Phrase {
                id: 0,
                dictionary_id: dict.id,
                text: "test_phrase".to_owned(),
            }];

            Phrase::bulk_insert(phrases, &mut conn).await.unwrap();
            dict
        };

        TaskToDict::insert(
            TaskToDict {
                task_id: task.id,
                dictionary_id: dict_to_create.id,
                contains: false,
            },
            &mut conn,
        )
        .await
        .unwrap();

        let settings = Settings::insert(
            Settings {
                id: Uuid::default(),
                project_id,
                r#type: SettingsKind::Script,
            },
            &mut conn,
        )
        .await
        .unwrap();

        let settings_item = SettingsItem::insert(
            SettingsItem {
                id: Uuid::default(),
                settings_id: settings.id,
                settings_immutable: true,
                name: "filler_words_test".to_string(),
                r#type: SettingsItemKind::FillerWordsDict,
                score_weight: 1,
            },
            &mut conn,
        )
        .await
        .unwrap();
        let _ = SettingsDictItem::bulk_insert(
            vec![SettingsDictItem {
                id: Uuid::default(),
                settings_item_id: settings_item.id,
                dictionary_id: dict_to_create.id,
                contains: false,
            }],
            &mut conn,
        )
        .await
        .unwrap();

        let mut metrics = {
            let mut metrics = CallMetrics::default();
            metrics.task_id = task.id;
            CallMetrics::insert(metrics.clone(), &mut conn)
                .await
                .unwrap();
            metrics
        };

        let response = do_detailed_metrics(cx, task.id, project_id)
            .await
            .expect("error while retrieving call metrics");
        assert_eq!(response.status(), StatusCode::OK);

        let detailed_metrics = response.payload();

        metrics.script_score = 100;

        assert_eq!(
            detailed_metrics,
            &TaskDetailedMetrics {
                nested: MetricsWithMetadata { metadata, metrics },
                efficiency_metrics: vec![TaskSettingsMetrics {
                    settings,
                    total_score: 100,
                    items: vec![TaskSettingsItemMetric {
                        settings_item,
                        score: 100
                    }]
                }]
            }
        )
    }
}
