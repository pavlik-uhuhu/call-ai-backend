use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Hash, Eq, sqlx::Type, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "participant_type", rename_all = "snake_case")]
pub enum ParticipantKind {
    Employee,
    Client,
}

impl fmt::Display for ParticipantKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub mod settings_metrics;
pub mod speech_recog;
