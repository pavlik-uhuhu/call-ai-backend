use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entity::speech_recog::EmotionKind;

#[derive(Clone, Debug, Default, PartialEq, Serialize, sqlx::FromRow, ToSchema)]
pub struct CallMetrics {
    pub task_id: Uuid,
    pub call_duration: f32,
    pub time_to_answer: f32,

    pub total_employee_speech: f32,
    pub total_client_speech: f32,

    pub employee_client_speech_ratio: f32,
    pub employee_speech_ratio: f32,
    pub client_speech_ratio: f32,

    pub call_holds_count: i32,

    pub silence_pause_count: i32,
    pub total_employee_silence: f32,

    pub client_interruptions_count: i32,
    pub total_client_interruptions_duration: f32,

    pub avg_employee_words_per_min: f32,
    pub avg_client_words_per_min: f32,

    pub script_score: i32,
    pub employee_quality_score: i32,

    pub emotion_mode: Option<EmotionKind>,
    pub emotion_start_mode: Option<EmotionKind>,
    pub emotion_end_mode: Option<EmotionKind>,
}

impl CallMetrics {
    pub async fn insert(metrics: Self, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                INSERT INTO task_call_metrics (
                    task_id,
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
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 
                    $12, $13, $14, $15, $16, $17, 
                    $18::call_metrics_emotion_type, 
                    $19::call_metrics_emotion_type,
                    $20::call_metrics_emotion_type
                )
            "#,
            metrics.task_id,
            metrics.call_duration,
            metrics.time_to_answer,
            metrics.total_employee_speech,
            metrics.total_client_speech,
            metrics.employee_client_speech_ratio,
            metrics.employee_speech_ratio,
            metrics.client_speech_ratio,
            metrics.call_holds_count,
            metrics.silence_pause_count,
            metrics.total_employee_silence,
            metrics.client_interruptions_count,
            metrics.total_client_interruptions_duration,
            metrics.avg_employee_words_per_min,
            metrics.avg_client_words_per_min,
            metrics.script_score,
            metrics.employee_quality_score,
            metrics.emotion_mode as Option<EmotionKind>,
            metrics.emotion_start_mode as Option<EmotionKind>,
            metrics.emotion_end_mode as Option<EmotionKind>
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn fetch_by_task_id(id: Uuid, conn: &mut sqlx::PgConnection) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Self,
            r#"
                SELECT 
                    task_id,
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
                    emotion_mode as "emotion_mode: EmotionKind",
                    emotion_start_mode as "emotion_start_mode: EmotionKind",
                    emotion_end_mode as "emotion_end_mode: EmotionKind"
                FROM task_call_metrics
                WHERE task_id = $1
            "#,
            id
        )
        .fetch_one(conn)
        .await
    }
}
