use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "settings_type", rename_all = "snake_case")]
pub enum SettingsKind {
    Quality,
    Script,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Settings {
    pub id: Uuid,
    pub project_id: Uuid,
    pub r#type: SettingsKind,
}

impl Settings {
    pub async fn list_by_project_id(
        project_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Settings,
            r#"
                SELECT id, project_id, type as "type: SettingsKind" 
                FROM settings
                WHERE project_id = $1
            "#,
            project_id,
        )
        .fetch_all(conn)
        .await
    }

    pub async fn fetch_by_id(id: Uuid, conn: &mut sqlx::PgConnection) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Settings,
            r#"
                SELECT id, project_id, type as "type: SettingsKind" 
                FROM settings
                WHERE id = $1
            "#,
            id,
        )
        .fetch_one(conn)
        .await
    }

    #[cfg(feature = "test")]
    pub async fn insert(settings: Self, conn: &mut sqlx::PgConnection) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Settings,
            r#"
                INSERT INTO settings 
                    (project_id, type)
                VALUES ($1, $2::settings_type)
                RETURNING
                    id, project_id, type as "type: SettingsKind"
            "#,
            settings.project_id,
            settings.r#type as SettingsKind
        )
        .fetch_one(conn)
        .await
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "settings_item_type", rename_all = "snake_case")]
pub enum SettingsItemKind {
    SpeechRateRatio,
    CallHolds,
    SilencePauses,
    Interruptions,
    LackingInfoDict,
    FillerWordsDict,
    SlurredSpeechDict,
    ProfanitySpeechDict,
    Dictionary,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SettingsItem {
    pub id: Uuid,
    pub settings_id: Uuid,
    pub settings_immutable: bool,
    pub r#type: SettingsItemKind,
    pub name: String,
    pub score_weight: i32,
}

impl SettingsItem {
    pub async fn list_by_project_id(
        project_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            SettingsItem,
            r#"
                SELECT si.id, si.settings_id, si.settings_immutable, 
                    si.type as "type: SettingsItemKind", si.name, si.score_weight
                FROM settings_item si
                JOIN settings on si.settings_id = settings.id
                WHERE project_id = $1
            "#,
            project_id,
        )
        .fetch_all(conn)
        .await
    }

    pub async fn insert(this: Self, conn: &mut sqlx::PgConnection) -> sqlx::Result<Self> {
        sqlx::query_as!(
            SettingsItem,
            r#"
                INSERT INTO settings_item
                    (settings_id, settings_immutable, type, name, score_weight)
                VALUES ($1, $2, $3::settings_item_type, $4, $5)
                RETURNING
                    id, settings_id, settings_immutable, type as "type: SettingsItemKind", name, score_weight
            "#,
            this.settings_id,
            this.settings_immutable,
            this.r#type as SettingsItemKind,
            this.name,
            this.score_weight
        )
        .fetch_one(conn)
        .await
    }

    pub async fn fetch_by_id(
        id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            SettingsItem,
            r#"
                SELECT 
                    id, settings_id, settings_immutable, type as "type: SettingsItemKind", name, score_weight
                FROM settings_item
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(conn)
        .await
    }

    pub async fn update_by_id(
        id: Uuid,
        name: String,
        score_weight: i32,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                UPDATE settings_item
                SET
                    name = $2,
                    score_weight = $3
                WHERE id = $1
            "#,
            id,
            name,
            score_weight
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn delete_by_id(id: Uuid, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM settings_item
                WHERE id = $1
            "#,
            id,
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct SettingsDictItem {
    #[serde(skip_deserializing)]
    pub id: Uuid,
    pub settings_item_id: Uuid,
    pub dictionary_id: i32,
    pub contains: bool,
}

impl SettingsDictItem {
    pub async fn list_by_project_id(
        project_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            SettingsDictItem,
            r#"
                SELECT sdi.id, sdi.settings_item_id, sdi.dictionary_id, sdi.contains
                FROM settings_dict_item sdi 
                JOIN settings_item on settings_item.id = sdi.settings_item_id 
                JOIN settings on settings.id = settings_item.settings_id
                WHERE project_id = $1
            "#,
            project_id,
        )
        .fetch_all(conn)
        .await
    }

    pub async fn bulk_insert(this: Vec<Self>, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        let mut item_ids = Vec::new();
        let mut dict_ids = Vec::new();
        let mut contains = Vec::new();
        this.into_iter().for_each(|item| {
            item_ids.push(item.settings_item_id);
            dict_ids.push(item.dictionary_id);
            contains.push(item.contains);
        });

        sqlx::query!(
            r#"
                INSERT INTO settings_dict_item
                    (settings_item_id, dictionary_id, contains)
                SELECT settings_item_id, dictionary_id, contains
                FROM UNNEST($1::uuid[], $2::int[], $3::bool[]) as a(settings_item_id, dictionary_id, contains)
            "#,
            &item_ids,
            &dict_ids,
            &contains
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn delete_by_item_id(
        settings_item_id: Uuid,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM settings_dict_item
                WHERE settings_item_id = $1
            "#,
            settings_item_id,
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}
