use axum::extract::{Path, State};
use axum::Json;
use http::StatusCode;
use protocol::db::dictionary::{Dictionary, Phrase};
use protocol::entity::ParticipantKind;
use serde::Deserialize;
use sqlx::Acquire;
use utoipa::{OpenApi, ToSchema};

use crate::context::{AppContext, Context};
use crate::error::{Error, ErrorKind};

use super::utils::{AppResponse, RequestResult};

#[derive(OpenApi)]
#[openapi(
    paths(list_dicts, dict_by_id, create, update, delete),
    components(schemas(Dictionary, Phrase, DictCreateRequest, DictUpdateRequest)),
    tags(
        (name = "Dictionaries", description = "API for handling dictionaries operations")
    )
)]
pub(super) struct ApiDictionaries;

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = OK, description = "List of dictionaries", body = Vec<Dictionary>),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to retrieve dictionaries")
    ),
    tags = ["Dictionaries"]
)]
pub async fn list_dicts(State(cx): State<AppContext>) -> RequestResult<Vec<Dictionary>> {
    do_list_dicts(cx).await
}

async fn do_list_dicts<C: Context>(cx: C) -> RequestResult<Vec<Dictionary>> {
    let mut conn = cx.get_db_conn().await?;
    let dicts = Dictionary::list(&mut conn).await?;

    Ok(AppResponse::new(StatusCode::OK, dicts))
}

#[utoipa::path(
    get,
    path = "/{dict_id}",
    responses(
        (status = OK, description = "Get a dictionary", body = Vec<Phrase>),
        (status = NOT_FOUND, description = "Dictionary not found"),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to retrieve dictionary phrases")
    ),
    params(
        ("dict_id" = i32, Path, description = "dictionary's id"),
    ),
    tags = ["Dictionaries"]
)]
pub async fn dict_by_id(
    State(cx): State<AppContext>,
    Path(dict_id): Path<i32>,
) -> RequestResult<Vec<Phrase>> {
    do_dict_by_id(cx, dict_id).await
}

async fn do_dict_by_id<C: Context>(cx: C, dict_id: i32) -> RequestResult<Vec<Phrase>> {
    let mut conn = cx.get_db_conn().await?;
    let dict = Dictionary::fetch_by_id(dict_id, &mut conn).await?;
    dict.ok_or(Error::new(
        ErrorKind::EntityNotFound,
        anyhow::anyhow!("dictionary by {dict_id} not found"),
    ))?;

    let phrases = Phrase::list_by_dict_id(dict_id, &mut conn).await?;

    Ok(AppResponse::new(StatusCode::OK, phrases))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DictCreateRequest {
    name: String,
    participant: ParticipantKind,
    phrases: Vec<String>,
}

#[utoipa::path(
    post,
    operation_id = "dict_create",
    path = "",
    request_body = DictCreateRequest,
    responses(
        (status = CREATED, description = "Dictionary created", body = Dictionary),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to create dictionary")
    ),
    tags = ["Dictionaries"]
)]
pub async fn create(
    State(cx): State<AppContext>,
    Json(request): Json<DictCreateRequest>,
) -> RequestResult<Dictionary> {
    do_create(cx, request).await
}

async fn do_create<C: Context>(cx: C, request: DictCreateRequest) -> RequestResult<Dictionary> {
    let mut conn = cx.get_db_conn().await?;
    let mut txn = conn.begin().await?;

    let dict = Dictionary::insert(request.name, request.participant, &mut txn).await?;
    let phrases = request
        .phrases
        .into_iter()
        .map(|phrase| Phrase {
            id: 0,
            dictionary_id: dict.id,
            text: phrase,
        })
        .collect();
    Phrase::bulk_insert(phrases, &mut txn).await?;

    txn.commit().await?;

    Ok(AppResponse::new(StatusCode::CREATED, dict))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DictUpdateRequest {
    delete_phrases: Vec<i64>,
    create_phrases: Vec<String>,
}

#[utoipa::path(
    put,
    operation_id = "dict_update",
    path = "/{dict_id}",
    request_body = DictUpdateRequest,
    responses(
        (status = OK, description = "Dictionary updated"),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to update dictionary"),
        (status = NOT_FOUND, description = "Dictionary not found")
    ),
    params(
        ("dict_id" = i32, Path, description = "Dictionary ID to update")
    ),
    tags = ["Dictionaries"]
)]
pub async fn update(
    State(cx): State<AppContext>,
    Path(dict_id): Path<i32>,
    Json(request): Json<DictUpdateRequest>,
) -> RequestResult<()> {
    do_update(cx, dict_id, request).await
}

async fn do_update<C: Context>(
    cx: C,
    dict_id: i32,
    request: DictUpdateRequest,
) -> RequestResult<()> {
    let mut conn = cx.get_db_conn().await?;
    let mut txn = conn.begin().await?;

    let dict = Dictionary::fetch_by_id(dict_id, &mut txn).await?;
    dict.ok_or(Error::new(
        ErrorKind::EntityNotFound,
        anyhow::anyhow!("dictionary by {dict_id} not found"),
    ))?;

    let create_phrases = request
        .create_phrases
        .into_iter()
        .map(|phrase| Phrase {
            id: 0,
            dictionary_id: dict_id,
            text: phrase,
        })
        .collect();
    Phrase::bulk_delete(request.delete_phrases, &mut txn).await?;
    Phrase::bulk_insert(create_phrases, &mut txn).await?;

    txn.commit().await?;

    Ok(AppResponse::new(StatusCode::OK, ()))
}

#[utoipa::path(
    delete,
    operation_id = "dict_delete",
    path = "/{dict_id}",
    responses(
        (status = OK, description = "Dictionary deleted"),
        (status = INTERNAL_SERVER_ERROR, description = "Failed to delete dictionary"),
        (status = NOT_FOUND, description = "Dictionary not found")
    ),
    params(
        ("dict_id" = i32, Path, description = "Dictionary ID to delete")
    ),
    tags = ["Dictionaries"]
)]
pub async fn delete(State(cx): State<AppContext>, Path(dict_id): Path<i32>) -> RequestResult<()> {
    do_delete(cx, dict_id).await
}

async fn do_delete<C: Context>(cx: C, dict_id: i32) -> RequestResult<()> {
    let mut conn = cx.get_db_conn().await?;
    let mut txn = conn.begin().await?;

    let dict = Dictionary::fetch_by_id(dict_id, &mut txn).await?;
    dict.ok_or(Error::new(
        ErrorKind::EntityNotFound,
        anyhow::anyhow!("dictionary by {dict_id} not found"),
    ))?;

    Phrase::delete_by_dict_id(dict_id, &mut txn).await?;
    Dictionary::delete_by_id(dict_id, &mut txn).await?;

    txn.commit().await?;

    Ok(AppResponse::new(StatusCode::OK, ()))
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::context::TestContext;

    use super::*;

    #[sqlx::test]
    async fn list_dicts(pool: sqlx::PgPool) {
        let dict_to_create = {
            let mut conn = pool.acquire().await.unwrap();

            Dictionary::insert("test_dict".to_owned(), ParticipantKind::Employee, &mut conn)
                .await
                .unwrap()
        };

        let cx = TestContext::new(pool).await;
        let dicts_resp = do_list_dicts(cx).await.expect("failed to retrieve dicts");
        assert_eq!(dicts_resp.status(), StatusCode::OK);
        let dict = dicts_resp
            .payload()
            .clone()
            .pop()
            .expect("empty dicts response");
        assert_eq!(dict.name, dict_to_create.name);
    }

    #[sqlx::test]
    async fn fetch_dict_by_id(pool: sqlx::PgPool) {
        let dict_to_create = {
            let mut conn = pool.acquire().await.unwrap();

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

        let cx = TestContext::new(pool).await;
        let dicts_resp = do_dict_by_id(cx, dict_to_create.id)
            .await
            .expect("failed to retrieve dict");
        assert_eq!(dicts_resp.status(), StatusCode::OK);
        let phrases = dicts_resp
            .payload()
            .clone()
            .pop()
            .expect("empty phrases response");
        assert_eq!(phrases.text, "test_phrase");
    }

    #[sqlx::test]
    async fn create_dict(pool: sqlx::PgPool) {
        let cx = TestContext::new(pool.clone()).await;
        let create_request = DictCreateRequest {
            name: "test_dict".to_string(),
            participant: ParticipantKind::Employee,
            phrases: vec!["test_phrase".to_string()],
        };

        let dicts_resp = do_create(cx, create_request)
            .await
            .expect("failed to create dict");

        assert_eq!(dicts_resp.status(), StatusCode::CREATED);
        let mut conn = pool.acquire().await.unwrap();
        let mut phrases = Phrase::list_by_dict_id(dicts_resp.payload().id, &mut conn)
            .await
            .expect("failed to retreive phrases");
        assert_eq!(phrases.pop().unwrap().text, "test_phrase");
    }

    #[sqlx::test]
    async fn update_dict(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let (dict, phrase_to_delete) = {
            let dict =
                Dictionary::insert("test_dict".to_owned(), ParticipantKind::Employee, &mut conn)
                    .await
                    .unwrap();
            let phrases = vec![Phrase {
                id: 0,
                dictionary_id: dict.id,
                text: "test_phrase_to_delete".to_owned(),
            }];

            Phrase::bulk_insert(phrases, &mut conn).await.unwrap();
            let phrase = Phrase::list_by_dict_id(dict.id, &mut conn)
                .await
                .unwrap()
                .pop()
                .unwrap();
            (dict, phrase)
        };

        let cx = TestContext::new(pool).await;
        let update_request = DictUpdateRequest {
            create_phrases: vec!["test_phrase".to_string()],
            delete_phrases: vec![phrase_to_delete.id],
        };

        let dicts_resp = do_update(cx, dict.id, update_request)
            .await
            .expect("failed to update dict");
        assert_eq!(dicts_resp.status(), StatusCode::OK);

        let mut phrases = Phrase::list_by_dict_id(dict.id, &mut conn).await.unwrap();
        assert_eq!(phrases.len(), 1);
        assert_eq!(&phrases.pop().unwrap().text, "test_phrase");
    }

    #[sqlx::test]
    async fn delet_dict(pool: sqlx::PgPool) {
        let dict = {
            let mut conn = pool.acquire().await.unwrap();
            let dict =
                Dictionary::insert("test_dict".to_owned(), ParticipantKind::Employee, &mut conn)
                    .await
                    .unwrap();
            let phrases = vec![Phrase {
                id: 0,
                dictionary_id: dict.id,
                text: "test_phrase_to_delete".to_owned(),
            }];

            Phrase::bulk_insert(phrases, &mut conn).await.unwrap();
            dict
        };

        let cx = TestContext::new(pool).await;

        let delete_resp = do_delete(cx.clone(), dict.id)
            .await
            .expect("failed to update dict");
        assert_eq!(delete_resp.status(), StatusCode::OK);

        let dict_resp = do_dict_by_id(cx, dict.id)
            .await
            .expect_err("unexpected success while retrieving dict");
        assert_eq!(dict_resp.kind, ErrorKind::EntityNotFound);
    }
}
