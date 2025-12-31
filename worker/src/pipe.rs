use anyhow::Context as _;
use futures::{Stream, StreamExt};
use lapin::{
    message::Delivery,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions,
        ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
    },
    types::FieldTable,
    Connection, ConnectionProperties,
};
use protocol::db::{
    metadata::CallMetadata,
    metrics::CallMetrics,
    task::{Task, TaskResultKind, TaskToDict},
};
use sqlx::Acquire;
use tracing::{debug, error};
use uuid::Uuid;

use crate::context::Context;
use crate::indexer::Indexer;
use crate::{clients::speech_recognition::SpeechRecognitionClient, domain};

async fn create_broker_connection() -> anyhow::Result<lapin::Connection> {
    let url = std::env::var("RABBITMQ_URL")?;
    let options = ConnectionProperties::default();
    let connection = Connection::connect(&url, options).await?;

    Ok(connection)
}

pub(crate) async fn run_broker_pipe<C>(cx: C, prefetch_count: u16) -> anyhow::Result<()>
where
    C: Context + Clone + Send + Sync + 'static,
{
    let connection = create_broker_connection().await?;
    let channel = connection.create_channel().await?;
    channel
        .basic_qos(prefetch_count, BasicQosOptions::default())
        .await?;
    channel
        .exchange_declare(
            "task_exchanger",
            lapin::ExchangeKind::Direct,
            ExchangeDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            "task_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            "task_queue",
            "task_exchanger",
            "task",
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let consumer = channel
        .basic_consume(
            "task_queue",
            "task_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    run_pipe(consumer, cx).await
}

async fn run_pipe<S, C>(mut stream: S, cx: C) -> anyhow::Result<()>
where
    S: Stream<Item = Result<Delivery, lapin::Error>> + Unpin,
    C: Context + Clone + Send + Sync + 'static,
{
    while let Some(message) = stream.next().await {
        match message {
            Ok(delivery) => {
                let cx = cx.clone();
                tokio::spawn(async move {
                    let delivery_res = match process(&delivery, &cx).await {
                        Ok(_) => delivery.ack(BasicAckOptions::default()).await,
                        Err(err) => {
                            error!("task processing failed: {:?}", err);
                            delivery.nack(BasicNackOptions::default()).await
                        }
                    };

                    if let Err(err) = delivery_res {
                        error!("RabbitMQ ack/nack failed: {:?}", err);
                    }
                });
            }
            Err(err) => {
                anyhow::bail!("error consuming tasks queue: {:?}", err);
            }
        }
    }

    Ok(())
}

async fn process<C: Context>(delivery: &Delivery, cx: &C) -> anyhow::Result<()> {
    let task_id: Uuid = serde_json::from_slice(&delivery.data)?;
    debug!("Handle Task with UUID: {task_id}");

    let mut task = {
        let mut conn = cx.get_db_conn().await?;
        Task::get(&task_id, &mut conn).await?
    };
    match process_task(&mut task, cx).await {
        Ok(_) => Ok(()),
        Err(err) => {
            task.status = TaskResultKind::Failed;
            task.failed_reason = Some(err.to_string());
            let mut conn = cx.get_db_conn().await?;
            Task::update(&task, &mut conn).await?;
            Err(err)
        }
    }
}

async fn process_task<C: Context>(task: &mut Task, cx: &C) -> anyhow::Result<()> {
    let task_id: Uuid = task.id;

    let metadata = {
        let mut conn = cx.get_db_conn().await?;
        CallMetadata::get_by_task_id(task_id, &mut conn).await?
    };

    let recog_data = cx
        .speech_recognition()
        .transcribe((&metadata).into())
        .await?;

    cx.indexer()
        .index_speech_recog(task_id, &recog_data)
        .await?;

    let mut metrics = domain::audio_metrics::process_metrics(&recog_data);
    metrics.task_id = task_id;
    let task_to_dicts =
        domain::keywords::process_metrics(cx, task_id, task.project_id, &mut metrics).await?;

    let mut conn = cx.get_db_conn().await?;
    let mut txn = conn
        .begin()
        .await
        .context("Failed to acquire transaction")?;

    task.status = TaskResultKind::Ready;
    task.failed_reason = None;

    CallMetrics::insert(metrics, &mut txn).await?;
    TaskToDict::bulk_insert(task_to_dicts, &mut txn).await?;
    Task::update(task, &mut txn).await?;

    txn.commit().await.context("Transaction failed")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use protocol::{
        db::{
            dictionary::{Dictionary, Phrase},
            settings::{Settings, SettingsDictItem, SettingsItem, SettingsItemKind, SettingsKind},
        },
        entity::{
            speech_recog::{
                CallHolds, Interval, PhraseTimestamps, RecognitionData, SpeechRecognition,
            },
            ParticipantKind,
        },
    };

    use crate::test_helpers::context::TestContext;

    use super::*;

    #[sqlx::test(migrations = "../api-server/migrations")]
    async fn task_processing(pool: sqlx::PgPool) {
        let mut cx = TestContext::new(pool.clone()).await;
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

        let mut conn = cx.get_db_conn().await.unwrap();
        let res = metadata.insert(&mut conn).await.unwrap();

        let mut task = {
            let task = Task {
                id: Uuid::default(),
                call_metadata_id: res.metadata_id,
                status: TaskResultKind::Processing,
                failed_reason: None,
                project_id,
            };

            task.insert(&mut conn).await.unwrap()
        };
        metadata.metadata_id = task.call_metadata_id;

        let dict_to_create = {
            let dict =
                Dictionary::insert("test_dict".to_owned(), ParticipantKind::Employee, &mut conn)
                    .await
                    .unwrap();
            let phrases = vec![Phrase {
                id: 0,
                dictionary_id: dict.id,
                text: "test phrase".to_owned(),
            }];

            Phrase::bulk_insert(phrases, &mut conn).await.unwrap();
            dict
        };

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
                name: "dict_test".to_string(),
                r#type: SettingsItemKind::Dictionary,
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
                contains: true,
            }],
            &mut conn,
        )
        .await
        .unwrap();

        cx.speech_recog_client_mock()
            .expect_transcribe()
            .with(mockall::predicate::always())
            .returning(|_| {
                Ok(RecognitionData {
                    call_holds: CallHolds::default(),
                    emotion_recognition_result: vec![],
                    phrase_timestamps: PhraseTimestamps::default(),
                    speech_recognition_result: vec![SpeechRecognition {
                        text: "test phrase".to_string(),
                        timestamps: Interval {
                            start: 0f32,
                            end: 10f32,
                        },
                        speaker: ParticipantKind::Employee,
                    }],
                })
            });

        let _ = process_task(&mut task, &cx)
            .await
            .expect("failed to process task");
        let task = Task::get(&task.id, &mut conn).await.unwrap();
        assert_eq!(task.status, TaskResultKind::Ready);

        let metrics = CallMetrics::fetch_by_task_id(task.id, &mut conn)
            .await
            .unwrap();
        assert_eq!(metrics.script_score, 100);
    }
}
