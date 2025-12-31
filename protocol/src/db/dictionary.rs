use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::entity::ParticipantKind;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Dictionary {
    pub id: i32,
    pub name: String,
    pub participant: ParticipantKind,
}

impl Dictionary {
    pub async fn fetch_by_id(id: i32, conn: &mut sqlx::PgConnection) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Dictionary,
            r#"
                SELECT
                    id,
                    name,
                    participant as "participant: ParticipantKind"
                FROM dictionary 
                WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(conn)
        .await
    }

    pub async fn insert(
        name: String,
        participant: ParticipantKind,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Dictionary,
            r#"
                INSERT INTO dictionary
                    (name, participant)
                VALUES ($1, $2::participant_type)
                RETURNING
                    id,
                    name,
                    participant as "participant: ParticipantKind"
            "#,
            name,
            participant as ParticipantKind
        )
        .fetch_one(conn)
        .await
    }

    pub async fn list(conn: &mut sqlx::PgConnection) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Dictionary,
            r#"
                SELECT id, name, participant as "participant: ParticipantKind" 
                FROM dictionary
            "#
        )
        .fetch_all(conn)
        .await
    }

    pub async fn delete_by_id(id: i32, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM dictionary
                WHERE id = $1
            "#,
            &id,
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Phrase {
    pub id: i64,
    pub dictionary_id: i32,
    pub text: String,
}

impl Phrase {
    pub async fn list_all(conn: &mut sqlx::PgConnection) -> sqlx::Result<Vec<Phrase>> {
        sqlx::query_as!(
            Phrase,
            r#"
            SELECT
                id,
                dictionary_id,
                text
            FROM phrase
            "#,
        )
        .fetch_all(conn)
        .await
    }

    pub async fn list_by_dict_id(
        dict_id: i32,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<Vec<Phrase>> {
        sqlx::query_as!(
            Phrase,
            r#"
            SELECT
                id,
                dictionary_id,
                text
            FROM phrase
            WHERE dictionary_id = $1
            "#,
            dict_id
        )
        .fetch_all(conn)
        .await
    }

    pub async fn bulk_insert(this: Vec<Self>, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        let mut dict_ids = Vec::new();
        let mut texts = Vec::new();
        this.into_iter().for_each(|item| {
            dict_ids.push(item.dictionary_id);
            texts.push(item.text.to_lowercase());
        });

        sqlx::query!(
            r#"
                INSERT INTO phrase
                    (dictionary_id, text)
                SELECT dictionary_id, text
                FROM UNNEST($1::int[], $2::text[]) as a(dictionary_id, text)
            "#,
            &dict_ids,
            &texts
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn bulk_delete(ids: Vec<i64>, conn: &mut sqlx::PgConnection) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM phrase
                WHERE id = ANY($1)
            "#,
            &ids,
        )
        .execute(conn)
        .await?;

        Ok(())
    }

    pub async fn delete_by_dict_id(
        dict_id: i32,
        conn: &mut sqlx::PgConnection,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
                DELETE FROM phrase
                WHERE dictionary_id = $1
            "#,
            &dict_id,
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}
