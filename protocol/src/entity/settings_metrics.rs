use std::collections::HashMap;

use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    auxiliary::group_by,
    db::{
        metrics::CallMetrics,
        settings::{Settings, SettingsDictItem, SettingsItem, SettingsItemKind, SettingsKind},
        task::TaskToDict,
    },
};

#[derive(Debug, PartialEq, Serialize, ToSchema)]
pub struct TaskSettingsItemMetric {
    pub settings_item: SettingsItem,
    pub score: i32, // normalized to 100%
}

#[derive(Debug, PartialEq, Serialize, ToSchema)]
pub struct TaskSettingsMetrics {
    pub settings: Settings,
    pub total_score: i32, // normalized to 100%
    pub items: Vec<TaskSettingsItemMetric>,
}

pub fn calculate_settings_metrics(
    task_to_dicts: Vec<TaskToDict>,
    call_metrics: &mut CallMetrics,
    settings: Vec<Settings>,
    settings_items: Vec<SettingsItem>,
    settings_dict_items: Vec<SettingsDictItem>,
) -> anyhow::Result<Vec<TaskSettingsMetrics>> {
    let task_to_dicts: HashMap<i32, bool> = task_to_dicts
        .into_iter()
        .map(|item| (item.dictionary_id, item.contains))
        .collect();
    let mut items_to_dict_items =
        group_by(settings_dict_items, |item| item.settings_item_id, |_| true);
    let mut settings_to_items = group_by(settings_items, |item| item.settings_id, |_| true);

    let mut result = vec![];
    for settings in settings.into_iter() {
        let score_point_normalized = {
            let sum_goal_scores_weights = settings_to_items
                .get(&settings.id)
                .ok_or(anyhow::anyhow!(
                    "can't find related settings {} {:?}",
                    settings.id,
                    settings.r#type
                ))?
                .iter()
                .fold(0, |acc, settings_item| acc + settings_item.score_weight);
            100f32 / sum_goal_scores_weights as f32
        };

        let mut total_score = 0;
        let mut settings_items_metrics = vec![];
        let settings_items = settings_to_items
            .remove(&settings.id)
            .ok_or(anyhow::anyhow!(
                "can't find related settings {} {:?}",
                settings.id,
                settings.r#type
            ))?;
        for settings_item in settings_items.into_iter() {
            let item_match = match settings_item.r#type {
                SettingsItemKind::CallHolds => call_metrics.call_holds_count == 0,
                SettingsItemKind::SilencePauses => call_metrics.silence_pause_count == 0,
                SettingsItemKind::Interruptions => call_metrics.client_interruptions_count == 0,
                SettingsItemKind::SpeechRateRatio => {
                    call_metrics.employee_client_speech_ratio <= 120.0
                        && call_metrics.employee_client_speech_ratio >= 80.0
                }
                _ => {
                    let item_dicts = items_to_dict_items
                        .remove(&settings_item.id)
                        .unwrap_or(vec![]);
                    let all_match = item_dicts.iter().any(|dict_item| !dict_item.contains);

                    let dicts_match = if all_match {
                        item_dicts.iter().all(|dict_item| {
                            task_to_dicts
                                .get(&dict_item.dictionary_id)
                                .map(|dict_contains| *dict_contains == dict_item.contains)
                                .unwrap_or(true)
                        })
                    } else {
                        item_dicts.iter().any(|dict_item| {
                            task_to_dicts
                                .get(&dict_item.dictionary_id)
                                .map(|dict_contains| *dict_contains == dict_item.contains)
                                .unwrap_or(false)
                        })
                    };

                    dicts_match
                }
            };

            let score = if item_match {
                settings_item.score_weight as f32 * score_point_normalized
            } else {
                0f32
            };

            let settings_item_metric = TaskSettingsItemMetric {
                settings_item,
                score: score as i32,
            };
            total_score += score as i32;
            settings_items_metrics.push(settings_item_metric);
        }

        match settings.r#type {
            SettingsKind::Script => {
                if call_metrics.script_score == 0 {
                    call_metrics.script_score = total_score
                }
            }
            SettingsKind::Quality => {
                if call_metrics.employee_quality_score == 0 {
                    call_metrics.employee_quality_score = total_score
                }
            }
        }

        result.push(TaskSettingsMetrics {
            settings,
            total_score,
            items: settings_items_metrics,
        });
    }

    Ok(result)
}
