use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

use super::ParticipantKind;

#[derive(
    Debug, Default, Serialize, Deserialize, PartialEq, Clone, Hash, Eq, Copy, sqlx::Type, ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "call_metrics_emotion_type", rename_all = "snake_case")]
pub enum EmotionKind {
    #[default]
    Neutral,
    Positive,
    Angry,
    Sad,
    Other,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RecognitionData {
    pub call_holds: CallHolds,
    pub emotion_recognition_result: Vec<EmotionKind>,
    pub phrase_timestamps: PhraseTimestamps,
    pub speech_recognition_result: Vec<SpeechRecognition>,
}

#[derive(Serialize, Default, PartialEq, Deserialize, Debug, ToSchema)]
pub struct CallHolds {
    #[serde(
        deserialize_with = "vec_ts_tuple_de",
        serialize_with = "vec_ts_tuple_serialize"
    )]
    #[schema(value_type = Vec<[f32; 2]>)]
    pub music: Vec<Interval>,
    #[serde(
        deserialize_with = "vec_ts_tuple_de",
        serialize_with = "vec_ts_tuple_serialize"
    )]
    #[schema(value_type = Vec<[f32; 2]>)]
    pub silent: Vec<Interval>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SpeechRecognition {
    pub text: String,
    #[serde(
        deserialize_with = "ts_tuple_de",
        serialize_with = "ts_tuple_serialize"
    )]
    #[schema(value_type = [f32; 2])]
    pub timestamps: Interval,
    pub speaker: ParticipantKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interval {
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PhraseTimestamps {
    #[serde(
        deserialize_with = "vec_ts_tuple_de",
        serialize_with = "vec_ts_tuple_serialize"
    )]
    #[schema(value_type = Vec<[f32; 2]>)]
    pub client: Vec<Interval>,
    #[serde(
        deserialize_with = "vec_ts_tuple_de",
        serialize_with = "vec_ts_tuple_serialize"
    )]
    #[schema(value_type = Vec<[f32; 2]>)]
    pub employee: Vec<Interval>,
}

fn vec_ts_tuple_de<'de, D>(deserializer: D) -> Result<Vec<Interval>, D::Error>
where
    D: Deserializer<'de>,
{
    let intervals: Vec<[f32; 2]> = Deserialize::deserialize(deserializer)?;
    let res = intervals
        .into_iter()
        .map(|interval| Interval {
            start: interval[0],
            end: interval[1],
        })
        .collect();

    Ok(res)
}

fn ts_tuple_de<'de, D>(deserializer: D) -> Result<Interval, D::Error>
where
    D: Deserializer<'de>,
{
    let interval: [f32; 2] = Deserialize::deserialize(deserializer)?;

    Ok(Interval {
        start: interval[0],
        end: interval[1],
    })
}

fn ts_tuple_serialize<S>(interval: &Interval, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    [interval.start, interval.end].serialize(s)
}

fn vec_ts_tuple_serialize<S>(intervals: &[Interval], s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let intervals: Vec<[f32; 2]> = intervals
        .iter()
        .map(|interval| [interval.start, interval.end])
        .collect();
    intervals.serialize(s)
}

#[cfg(test)]
mod tests {
    use crate::entity::{
        speech_recog::{Interval, PhraseTimestamps, SpeechRecognition},
        ParticipantKind,
    };

    #[test]
    fn serialize_test() {
        let serialized = serde_json::to_vec(&PhraseTimestamps {
            client: vec![Interval {
                start: 0.0,
                end: 1.0,
            }],
            employee: vec![],
        })
        .unwrap();

        let timestamps: PhraseTimestamps = serde_json::from_slice(&serialized).unwrap();
        assert!(!timestamps.client.is_empty());

        let serialized = serde_json::to_vec(&SpeechRecognition {
            text: "hello".to_string(),
            timestamps: Interval {
                start: 0.0,
                end: 1.0,
            },
            speaker: ParticipantKind::Client,
        })
        .unwrap();

        let _: SpeechRecognition = serde_json::from_slice(&serialized).unwrap();
    }
}
