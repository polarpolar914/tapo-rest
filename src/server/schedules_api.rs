use axum::{extract::{Path, State}, routing::{get, post, patch, delete}, Json, Router};
use axum_extra::{TypedHeader, headers::{authorization::Bearer, Authorization}};
use serde::Deserialize;
use uuid::Uuid;

use super::{auth::auth, ApiResult, SharedState};
use crate::server::schedules::{Schedule, ScheduleUpdate, ScheduleType};

#[derive(Deserialize)]
pub struct ScheduleInput {
    pub device_id: String,
    pub action: String,
    pub schedule_type: ScheduleType,
    pub time: String,
    #[serde(default)]
    pub days_of_week: Vec<String>,
    #[serde(default)]
    pub exclude_dates: Vec<String>,
    pub end_date: Option<String>,
}

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/schedules", post(add_schedule).get(list_schedules))
        .route("/schedules/all", get(list_schedules))
        .route("/schedules/:id", patch(update_schedule).delete(delete_schedule))
}

async fn add_schedule(
    auth_header: TypedHeader<Authorization<Bearer>>,
    State(state): State<SharedState>,
    Json(input): Json<ScheduleInput>,
) -> ApiResult<Json<String>> {
    auth(auth_header, &state.sessions).await?;
    let mut store = state.schedules_mut().await;
    let schedule = Schedule {
        id: Uuid::new_v4().to_string(),
        device_id: input.device_id,
        action: input.action,
        schedule_type: input.schedule_type,
        time: input.time,
        days_of_week: input.days_of_week,
        exclude_dates: input.exclude_dates,
        end_date: input.end_date,
        owner: "admin@yourdomain.com".into(),
    };
    let id = store.insert(schedule).await?;
    Ok(Json(id))
}

async fn list_schedules(
    auth_header: TypedHeader<Authorization<Bearer>>,
    State(state): State<SharedState>,
) -> ApiResult<Json<Vec<Schedule>>> {
    auth(auth_header, &state.sessions).await?;
    let store = state.schedules.lock().await;
    let schedules = store.schedules.values().cloned().collect();
    Ok(Json(schedules))
}

async fn update_schedule(
    auth_header: TypedHeader<Authorization<Bearer>>,
    Path(id): Path<String>,
    State(state): State<SharedState>,
    Json(update): Json<ScheduleUpdate>,
) -> ApiResult<()> {
    auth(auth_header, &state.sessions).await?;
    let mut store = state.schedules_mut().await;
    store.update(&id, update).await?;
    Ok(())
}

async fn delete_schedule(
    auth_header: TypedHeader<Authorization<Bearer>>,
    Path(id): Path<String>,
    State(state): State<SharedState>,
) -> ApiResult<()> {
    auth(auth_header, &state.sessions).await?;
    let mut store = state.schedules_mut().await;
    store.delete(&id).await?;
    Ok(())
}

