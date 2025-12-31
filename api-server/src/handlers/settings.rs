use axum::extract::Path;
use axum::{extract::State, Json};
use http::StatusCode;
use protocol::auxiliary;
use protocol::db::settings::SettingsKind;
use protocol::db::{
    dictionary::Dictionary,
    settings::{Settings, SettingsDictItem, SettingsItem},
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::context::{AppContext, Context};
use crate::error::{Error, ErrorKind};
use crate::handlers::utils::{AppResponse, RequestResult};

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingsItemWithDicts {
    item: SettingsItem,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dicts: Vec<Dictionary>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingsResponse {
    quality: Vec<SettingsItemWithDicts>,
    script: Vec<SettingsItemWithDicts>,
}

#[derive(OpenApi)]
#[openapi(
    paths(settings_list, settings_item_create, settings_item_update, settings_item_delete),
    components(schemas(SettingsItemCreateRequest, SettingsItemUpdateRequest, SettingsResponse, SettingsItemWithDicts, Dictionary)),
    tags(
        (name = "Settings", description = "API for handle settings options")
    )
)]
pub struct ApiSettings;

#[utoipa::path(
    get,
    path = "",
    responses(
        (status = OK, description = "List Settings of Project", body = SettingsResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Error while trying to handle list of settings")
    ),
    tags = ["Settings"]
)]
pub async fn settings_list(State(cx): State<AppContext>) -> RequestResult<SettingsResponse> {
    do_settings_list(cx, Uuid::default()).await
}

async fn do_settings_list<C: Context>(cx: C, project_id: Uuid) -> RequestResult<SettingsResponse> {
    let mut conn = cx.get_db_conn().await?;
    let settings = Settings::list_by_project_id(project_id, &mut conn).await?;
    let dictionaries = Dictionary::list(&mut conn).await?;
    let settings_dict_items = SettingsDictItem::list_by_project_id(project_id, &mut conn).await?;
    let mut settings_dict_items = auxiliary::group_by(
        settings_dict_items,
        |dict_item| dict_item.settings_item_id,
        |_| true,
    );
    let settings_items = SettingsItem::list_by_project_id(project_id, &mut conn).await?;
    drop(conn);

    let mut items_with_dicts = vec![];
    for item in settings_items.into_iter() {
        let dict_items = settings_dict_items.remove(&item.id).unwrap_or(vec![]);
        let dict_items = dict_items.into_iter().flat_map(|dict_item| {
            dictionaries
                .iter()
                .find(|dict| dict.id == dict_item.dictionary_id)
                .cloned()
                .into_iter()
        });
        items_with_dicts.push({
            SettingsItemWithDicts {
                item,
                dicts: dict_items.collect(),
            }
        });
    }

    let mut items_with_dicts =
        auxiliary::group_by(items_with_dicts, |item| item.item.settings_id, |_| true);
    let quality_settings = {
        let id = settings
            .iter()
            .find(|settings| settings.r#type == SettingsKind::Quality)
            .map(|settings| settings.id)
            .ok_or(Error::new(
                ErrorKind::EntityNotFound,
                anyhow::anyhow!("related quality settings id not found"),
            ))?;
        items_with_dicts.remove(&id).unwrap_or(vec![])
    };
    let script_settings = {
        let id = settings
            .iter()
            .find(|settings| settings.r#type == SettingsKind::Script)
            .map(|settings| settings.id)
            .ok_or(Error::new(
                ErrorKind::EntityNotFound,
                anyhow::anyhow!("related script settings id not found"),
            ))?;
        items_with_dicts.remove(&id).unwrap_or(vec![])
    };

    Ok(AppResponse::new(
        StatusCode::OK,
        SettingsResponse {
            quality: quality_settings,
            script: script_settings,
        },
    ))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SettingsItemCreateRequest {
    item: SettingsItem,
    dict_items: Vec<SettingsDictItem>,
}

#[utoipa::path(
    post,
    path = "/item",
    request_body = SettingsItemCreateRequest,
    responses(
        (status = CREATED, description = "Create Settings", body = SettingsItem),
        (status = BAD_REQUEST, description = "Trying to create non-script settings item"),
        (status = NOT_FOUND, description = "Related settings not found"),
        (status = INTERNAL_SERVER_ERROR, description = "Server error when creating a settings item")
    ),
    tags = ["Settings"]
)]
pub async fn settings_item_create(
    State(cx): State<AppContext>,
    Json(request): Json<SettingsItemCreateRequest>,
) -> RequestResult<SettingsItem> {
    do_settings_item_create(cx, Uuid::default(), request).await
}

async fn do_settings_item_create<C: Context>(
    cx: C,
    project_id: Uuid,
    request: SettingsItemCreateRequest,
) -> RequestResult<SettingsItem> {
    let mut conn = cx.get_db_conn().await?;
    let settings = Settings::list_by_project_id(project_id, &mut conn).await?;
    let related_settings = settings
        .into_iter()
        .find(|settings| settings.r#type == SettingsKind::Script)
        .ok_or(Error::new(
            ErrorKind::EntityNotFound,
            anyhow::anyhow!("related script settings id not found"),
        ))?;

    if related_settings.id != request.item.settings_id {
        return Err(Error::new(
            ErrorKind::InvalidSettingsRequest,
            anyhow::anyhow!("attempted to create non-script settings item"),
        ));
    }

    let inserted_item = SettingsItem::insert(request.item, &mut conn).await?;

    let dict_items = request
        .dict_items
        .into_iter()
        .map(|dict_item| {
            let mut dict_item = dict_item.clone();
            dict_item.settings_item_id = inserted_item.id;
            dict_item
        })
        .collect();
    SettingsDictItem::bulk_insert(dict_items, &mut conn).await?;

    Ok(AppResponse::new(StatusCode::CREATED, inserted_item))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SettingsItemUpdateRequest {
    item_name: String,
    item_score_weight: i32,
    dict_items: Vec<SettingsDictItem>,
}

#[utoipa::path(
    put,
    path = "/item/{item_id}",
    request_body = SettingsItemUpdateRequest,
    responses(
        (status = OK, description = "Updates the setting item"),
        (status = NOT_FOUND, description = "Setting item not found"),
        (status = INTERNAL_SERVER_ERROR, description = "Server error when updating a settings item")
    ),
    params(
        ("item_id" = Uuid, Path, description = "Unique identifier of the setting item")
    ),
    tags = ["Settings"]
)]
pub async fn settings_item_update(
    State(cx): State<AppContext>,
    Path(item_id): Path<Uuid>,
    Json(request): Json<SettingsItemUpdateRequest>,
) -> RequestResult<()> {
    do_settings_item_update(cx, Uuid::default(), item_id, request).await
}

async fn do_settings_item_update<C: Context>(
    cx: C,
    _project_id: Uuid,
    item_id: Uuid,
    request: SettingsItemUpdateRequest,
) -> RequestResult<()> {
    let mut conn = cx.get_db_conn().await?;
    let _ = SettingsItem::fetch_by_id(item_id, &mut conn)
        .await?
        .ok_or(Error::new(
            ErrorKind::EntityNotFound,
            anyhow::anyhow!("settings item by {item_id} not found"),
        ))?;
    SettingsItem::update_by_id(
        item_id,
        request.item_name,
        request.item_score_weight,
        &mut conn,
    )
    .await?;

    SettingsDictItem::delete_by_item_id(item_id, &mut conn).await?;

    let dict_items = request
        .dict_items
        .into_iter()
        .map(|dict_item| {
            let mut dict_item = dict_item.clone();
            dict_item.settings_item_id = item_id;
            dict_item
        })
        .collect();
    SettingsDictItem::bulk_insert(dict_items, &mut conn).await?;

    Ok(AppResponse::new(StatusCode::OK, ()))
}

#[utoipa::path(
    delete,
    path = "/item/{item_id}",
    responses(
        (status = OK, description = "Deletes a settings item"),
        (status = NOT_FOUND, description = "Setting item not found"),
        (status = BAD_REQUEST, description = "Trying to delete non-script settings item"),
        (status = INTERNAL_SERVER_ERROR, description = "Server error when deleting a settings item")
    ),
    params(
        ("item_id" = Uuid, Path, description = "Unique identifier of the setting item")
    ),
    tags = ["Settings"]
)]
pub async fn settings_item_delete(
    State(cx): State<AppContext>,
    Path(item_id): Path<Uuid>,
) -> RequestResult<()> {
    do_settings_item_delete(cx, Uuid::default(), item_id).await
}

async fn do_settings_item_delete<C: Context>(
    cx: C,
    project_id: Uuid,
    item_id: Uuid,
) -> RequestResult<()> {
    let mut conn = cx.get_db_conn().await?;
    let settings = Settings::list_by_project_id(project_id, &mut conn).await?;
    let related_settings = settings
        .into_iter()
        .find(|settings| settings.r#type == SettingsKind::Script)
        .ok_or(Error::new(
            ErrorKind::EntityNotFound,
            anyhow::anyhow!("related script settings id not found"),
        ))?;
    let item = SettingsItem::fetch_by_id(item_id, &mut conn)
        .await?
        .ok_or(Error::new(
            ErrorKind::EntityNotFound,
            anyhow::anyhow!("settings item by {item_id} not found"),
        ))?;

    if related_settings.id != item.settings_id {
        return Err(Error::new(
            ErrorKind::InvalidSettingsRequest,
            anyhow::anyhow!("attempted to delete non-script settings item"),
        ));
    }

    SettingsDictItem::delete_by_item_id(item_id, &mut conn).await?;
    SettingsItem::delete_by_id(item_id, &mut conn).await?;

    Ok(AppResponse::new(StatusCode::OK, ()))
}
