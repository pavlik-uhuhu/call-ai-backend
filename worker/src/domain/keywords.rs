use std::collections::HashMap;

use protocol::auxiliary;
use protocol::db::dictionary::{Dictionary, Phrase};
use protocol::db::metrics::CallMetrics;
use protocol::db::settings::{Settings, SettingsDictItem, SettingsItem};
use protocol::db::task::TaskToDict;
use protocol::entity::settings_metrics::{self};
use tracing::warn;
use uuid::Uuid;

use crate::{context::Context, indexer::Indexer};

pub async fn process_metrics<C: Context>(
    cx: &C,
    id: Uuid,
    project_id: Uuid,
    call_metrics: &mut CallMetrics,
) -> anyhow::Result<Vec<TaskToDict>> {
    let phrases = {
        let mut conn = cx.get_db_conn().await?;
        Phrase::list_all(&mut conn).await?
    };
    let dicts = {
        let mut conn = cx.get_db_conn().await?;
        Dictionary::list(&mut conn).await?
    };

    let grouped: HashMap<i32, Vec<Phrase>> =
        auxiliary::group_by(phrases, |phrase| phrase.dictionary_id, |_| true);

    let mut task_to_dicts: Vec<TaskToDict> = vec![];

    for (dictionary_id, phrases) in grouped {
        let dict = match dicts.iter().find(|dict| dict.id == dictionary_id) {
            None => {
                warn!("skipping non-existing dictionary {dictionary_id}");
                continue;
            }
            Some(dict) => dict,
        };

        let mut contains = false;
        for phrase in phrases {
            contains = cx
                .indexer()
                .search_phrase(id, &phrase.text, &dict.participant)
                .await?;
            if contains {
                break;
            }
        }

        task_to_dicts.push(TaskToDict {
            task_id: id,
            dictionary_id,
            contains,
        })
    }

    let mut conn = cx.get_db_conn().await?;
    let settings = Settings::list_by_project_id(project_id, &mut conn).await?;
    let settings_items = SettingsItem::list_by_project_id(project_id, &mut conn).await?;
    let settings_dict_items = SettingsDictItem::list_by_project_id(project_id, &mut conn).await?;

    settings_metrics::calculate_settings_metrics(
        task_to_dicts.clone(),
        call_metrics,
        settings,
        settings_items,
        settings_dict_items,
    )?;

    Ok(task_to_dicts)
}
