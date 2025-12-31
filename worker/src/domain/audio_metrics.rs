use std::collections::HashMap;

use protocol::{
    db::metrics::CallMetrics,
    entity::{
        speech_recog::{CallHolds, EmotionKind, Interval, RecognitionData, SpeechRecognition},
        ParticipantKind,
    },
};
use uuid::Uuid;

const OVERLAP_DURATION_EPS: f32 = 1.0;
const PAUSE_DURATION: f32 = 5.0;

fn intervals_overlap(first_interval: &Interval, seconds_interval: &Interval) -> bool {
    first_interval.start < seconds_interval.end && seconds_interval.start < first_interval.end
}

fn is_interruption(employee_interval: &Interval, client_interval: &Interval) -> bool {
    let overlap_start = employee_interval.start.max(client_interval.start);
    let overlap_end = employee_interval.end.min(client_interval.end);

    let overlap_duration = overlap_end - overlap_start;

    employee_interval.start > client_interval.start
        && employee_interval.start < client_interval.end
        && overlap_duration >= OVERLAP_DURATION_EPS
}

fn find_interruptions(
    employee_intervals: &Vec<Interval>,
    client_intervals: &Vec<Interval>,
) -> (f32, i32) {
    let mut interruptions_count: i32 = 0;
    let mut total_interruption_time = 0.0;

    for employee_interval in employee_intervals {
        for client_interval in client_intervals {
            if is_interruption(employee_interval, client_interval) {
                interruptions_count += 1;
                total_interruption_time += employee_interval.end - employee_interval.start;
                break;
            }
        }
    }
    (total_interruption_time, interruptions_count)
}

fn time_to_answer(employee_intervals: &[Interval]) -> Option<f32> {
    if employee_intervals.is_empty() {
        return None;
    }

    employee_intervals.first().map(|interval| interval.start)
}

fn total_speech_duration(intervals: &[Interval]) -> f32 {
    intervals
        .iter()
        .map(|interval| interval.end - interval.start)
        .sum()
}

fn speech_percentage(total_speech: f32, total_call_duration: f32) -> f32 {
    if total_call_duration == 0.0 {
        return 0.0;
    }
    (total_speech / total_call_duration) * 100.0
}

fn count_pauses(
    employee_intervals: &[Interval],
    client_intervals: &[Interval],
    holds: &CallHolds,
) -> (i32, f32) {
    if employee_intervals.is_empty() || client_intervals.is_empty() {
        return (0, 0.0);
    }

    let mut hold_intervals = vec![];
    hold_intervals.extend(holds.music.iter().cloned());
    hold_intervals.extend(holds.silent.iter().cloned());
    let mut hold_intervals: Vec<Interval> = hold_intervals
        .iter()
        .map(|hold| Interval {
            start: hold.start - PAUSE_DURATION,
            end: hold.end + PAUSE_DURATION,
        })
        .collect();
    hold_intervals.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());

    let mut intervals = employee_intervals
        .iter()
        .map(|interval| (ParticipantKind::Employee, interval))
        .chain(
            client_intervals
                .iter()
                .map(|interval| (ParticipantKind::Client, interval)),
        )
        .collect::<Vec<_>>();

    intervals.sort_by(|a, b| a.1.start.partial_cmp(&b.1.start).unwrap());

    let mut previous_end: Option<f32> = None;
    let mut pause_count = 0;
    let mut pause_sum = 0.0;
    for interval in intervals {
        if let Some(ref end) = previous_end {
            if interval.0 == ParticipantKind::Employee
                && *end < interval.1.start
                && interval.1.start - *end >= PAUSE_DURATION
                && !hold_intervals
                    .iter()
                    .any(|hold| intervals_overlap(hold, interval.1))
            {
                pause_count += 1;
                pause_sum += interval.1.start - *end;
            }
        }
        if interval.0 == ParticipantKind::Employee {
            previous_end = Some(interval.1.end);
        } else {
            previous_end = None;
        }
    }

    (pause_count, pause_sum)
}

fn calculate_words_per_minute(
    transcriptions: &[SpeechRecognition],
    speech_time: f32,
    speaker: ParticipantKind,
) -> f32 {
    let total_words = transcriptions
        .iter()
        .filter(|transcription| transcription.speaker == speaker)
        .fold(0, |words, transcription| {
            words + transcription.text.split_whitespace().count()
        });

    let speech_time_min = speech_time / 60.0;

    total_words as f32 / speech_time_min
}

fn call_emotional_mode(emotions: &Vec<EmotionKind>) -> Option<EmotionKind> {
    let mut occurrence: HashMap<EmotionKind, i32> = HashMap::new();

    for emotion in emotions {
        *occurrence.entry(*emotion).or_insert(0) += 1;
    }

    occurrence
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(emotion, _)| emotion)
}

pub fn process_metrics(recog_data: &RecognitionData) -> CallMetrics {
    let (silence_pause_count, total_employee_silence) = count_pauses(
        &recog_data.phrase_timestamps.employee,
        &recog_data.phrase_timestamps.client,
        &recog_data.call_holds,
    );

    let (total_client_interruptions_duration, client_interruptions_count) = find_interruptions(
        &recog_data.phrase_timestamps.employee,
        &recog_data.phrase_timestamps.client,
    );

    let total_employee_speech = total_speech_duration(&recog_data.phrase_timestamps.employee);
    let total_client_speech = total_speech_duration(&recog_data.phrase_timestamps.client);

    let avg_employee_words_per_min = calculate_words_per_minute(
        &recog_data.speech_recognition_result,
        total_employee_speech,
        ParticipantKind::Employee,
    );
    let avg_client_words_per_min = calculate_words_per_minute(
        &recog_data.speech_recognition_result,
        total_client_speech,
        ParticipantKind::Client,
    );

    let call_duration = recog_data
        .phrase_timestamps
        .client
        .iter()
        .chain(recog_data.phrase_timestamps.employee.iter())
        .map(|interval| interval.end)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0f32);

    let holds_count = recog_data.call_holds.silent.len() + recog_data.call_holds.music.len();

    CallMetrics {
        task_id: Uuid::default(),
        call_duration,
        time_to_answer: time_to_answer(&recog_data.phrase_timestamps.employee).unwrap_or(0.0),
        total_employee_speech,
        total_client_speech,
        employee_client_speech_ratio: speech_percentage(total_employee_speech, total_client_speech),
        employee_speech_ratio: speech_percentage(total_employee_speech, call_duration),
        client_speech_ratio: speech_percentage(total_client_speech, call_duration),
        call_holds_count: holds_count as i32,
        silence_pause_count,
        total_employee_silence,
        client_interruptions_count,
        total_client_interruptions_duration,
        avg_employee_words_per_min: avg_employee_words_per_min.round(),
        avg_client_words_per_min: avg_client_words_per_min.round(),
        employee_quality_score: 0,
        script_score: 0,
        emotion_mode: call_emotional_mode(&recog_data.emotion_recognition_result),
        emotion_start_mode: recog_data.emotion_recognition_result.first().cloned(),
        emotion_end_mode: recog_data.emotion_recognition_result.last().cloned(),
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use protocol::entity::{
        speech_recog::{CallHolds, EmotionKind, Interval, SpeechRecognition},
        ParticipantKind,
    };

    use crate::domain::audio_metrics::{
        calculate_words_per_minute, call_emotional_mode, count_pauses, find_interruptions,
        intervals_overlap, is_interruption, speech_percentage, time_to_answer,
        total_speech_duration,
    };

    #[test]
    fn test_intervals_overlap() {
        let interval1 = Interval {
            start: 0.0,
            end: 5.0,
        };
        let interval2 = Interval {
            start: 5.0,
            end: 10.0,
        };
        assert!(!intervals_overlap(&interval1, &interval2));

        let interval1 = Interval {
            start: 0.0,
            end: 10.0,
        };
        let interval2 = Interval {
            start: 2.0,
            end: 8.0,
        };
        assert!(intervals_overlap(&interval1, &interval2));
    }

    #[test]
    fn test_is_interruption() {
        let employee = Interval {
            start: 7.0,
            end: 10.0,
        };
        let client = Interval {
            start: 5.0,
            end: 15.0,
        };
        assert!(is_interruption(&employee, &client));

        let employee = Interval {
            start: 5.0,
            end: 10.0,
        };
        let client = Interval {
            start: 6.0,
            end: 12.0,
        };
        assert!(!is_interruption(&employee, &client));

        let employee = Interval {
            start: 0.0,
            end: 5.0,
        };
        let client = Interval {
            start: 6.0,
            end: 10.0,
        };
        assert!(!is_interruption(&employee, &client));
    }

    #[test]
    fn test_find_interruptions() {
        let employee_intervals = vec![
            Interval {
                start: 2.0,
                end: 4.0,
            },
            Interval {
                start: 9.0,
                end: 12.0,
            },
            Interval {
                start: 18.0,
                end: 22.0,
            },
        ];
        let client_intervals = vec![
            Interval {
                start: 5.0,
                end: 10.0,
            },
            Interval {
                start: 15.0,
                end: 20.0,
            },
        ];
        let interruptions = find_interruptions(&employee_intervals, &client_intervals);
        assert_eq!(interruptions, (7.0, 2));
    }

    #[test]
    fn test_time_to_answer() {
        let employee_intervals = vec![Interval {
            start: 10.0,
            end: 15.0,
        }];
        let result = time_to_answer(&employee_intervals);
        assert_eq!(result, Some(10.0));

        let employee_intervals = vec![];
        let result = time_to_answer(&employee_intervals);
        assert_eq!(result, None);
    }

    #[test]
    fn test_total_speech_duration() {
        let intervals = vec![];
        let result = total_speech_duration(&intervals);
        assert_eq!(result, 0.0);

        let intervals = vec![
            Interval {
                start: 0.0,
                end: 5.0,
            },
            Interval {
                start: 10.0,
                end: 15.0,
            },
        ];
        let result = total_speech_duration(&intervals);
        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_speech_percentage() {
        let total_speech = 10.0;
        let total_call_duration = 50.0;
        assert_eq!(speech_percentage(total_speech, total_call_duration), 20.0);
    }

    #[test]
    fn test_count_pauses() {
        let employee_intervals = vec![
            Interval {
                start: 0.0,
                end: 2.0,
            },
            Interval {
                start: 15.0,
                end: 17.0,
            },
        ];
        let client_intervals = vec![Interval {
            start: 2.0,
            end: 15.0,
        }];
        let result = count_pauses(
            &employee_intervals,
            &client_intervals,
            &CallHolds {
                music: vec![],
                silent: vec![],
            },
        );
        assert_eq!(result, (0, 0.0));

        let employee_intervals = vec![
            Interval {
                start: 0.0,
                end: 2.0,
            },
            Interval {
                start: 8.0,
                end: 15.0,
            },
            Interval {
                start: 25.0,
                end: 30.0,
            },
            Interval {
                start: 50.0,
                end: 60.0,
            },
        ];
        let client_intervals = vec![Interval {
            start: 30.0,
            end: 40.0,
        }];
        let result = count_pauses(
            &employee_intervals,
            &client_intervals,
            &CallHolds {
                music: vec![],
                silent: vec![],
            },
        );
        assert_eq!(result, (2, 16.0));

        let employee_intervals = vec![];
        let client_intervals = vec![Interval {
            start: 0.0,
            end: 5.0,
        }];
        let result = count_pauses(
            &employee_intervals,
            &client_intervals,
            &CallHolds {
                music: vec![],
                silent: vec![],
            },
        );
        assert_eq!(result, (0, 0.0));

        let employee_intervals = vec![
            Interval {
                start: 0.0,
                end: 2.0,
            },
            Interval {
                start: 12.0,
                end: 22.0,
            },
        ];
        let client_intervals = vec![];
        let result = count_pauses(
            &employee_intervals,
            &client_intervals,
            &CallHolds {
                music: vec![],
                silent: vec![],
            },
        );
        assert_eq!(result, (0, 0.0));

        // Not pause, but call hold
        let employee_intervals = vec![
            Interval {
                start: 0.0,
                end: 2.0,
            },
            Interval {
                start: 10.0,
                end: 15.0,
            },
        ];
        let client_intervals = vec![Interval {
            start: 5.0,
            end: 7.0,
        }];
        let holds = CallHolds {
            music: vec![Interval {
                start: 8.0,
                end: 12.0,
            }],
            silent: vec![],
        };
        let result = count_pauses(&employee_intervals, &client_intervals, &holds);
        assert_eq!(result, (0, 0.0));

        // No pauses
        let employee_intervals = vec![
            Interval {
                start: 0.0,
                end: 2.0,
            },
            Interval {
                start: 3.0,
                end: 4.0,
            },
        ];
        let client_intervals = vec![Interval {
            start: 2.0,
            end: 3.0,
        }];
        let result = count_pauses(
            &employee_intervals,
            &client_intervals,
            &CallHolds {
                music: vec![],
                silent: vec![],
            },
        );
        assert_eq!(result, (0, 0.0));
    }

    #[test]
    fn test_calculate_wpm() {
        let transcriptions = vec![
            SpeechRecognition {
                text: String::from("Hello this is a test."),
                speaker: ParticipantKind::Employee,
                timestamps: Interval {
                    start: 0.0,
                    end: 20.0,
                },
            },
            SpeechRecognition {
                text: String::from("This is another test."),
                speaker: ParticipantKind::Employee,
                timestamps: Interval {
                    start: 25.0,
                    end: 55.0,
                },
            },
            SpeechRecognition {
                text: String::from("And another one."),
                speaker: ParticipantKind::Employee,
                timestamps: Interval {
                    start: 60.0,
                    end: 70.0,
                },
            },
        ];

        let wpm = calculate_words_per_minute(&transcriptions, 60.0, ParticipantKind::Employee);
        assert_eq!(wpm, 12.0);
    }

    #[test]
    fn test_call_emotional_mode() {
        let emotions = vec![];
        assert_eq!(call_emotional_mode(&emotions), None);

        let emotions = vec![
            EmotionKind::Positive,
            EmotionKind::Neutral,
            EmotionKind::Positive,
            EmotionKind::Positive,
            EmotionKind::Neutral,
        ];
        assert_eq!(call_emotional_mode(&emotions), Some(EmotionKind::Positive));

        // Equal frequency of amount of emotions
        let emotions = vec![
            EmotionKind::Positive,
            EmotionKind::Neutral,
            EmotionKind::Positive,
            EmotionKind::Neutral,
        ];
        let result = call_emotional_mode(&emotions);
        assert!(result == Some(EmotionKind::Positive) || result == Some(EmotionKind::Neutral));
    }
}
