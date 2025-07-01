use axum::{extract::{Path, State}, routing::{get, post, patch}, Json, Router};
use axum_extra::TypedHeader;
use axum_extra::headers::{authorization::Bearer, Authorization};
use serde::{Deserialize, Serialize};
use crate::{scheduler::{Schedule, ScheduleKind, Action}, server::{auth::auth, ApiError, ApiResult, SharedState}};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc, Weekday};

pub fn make_router() -> Router<SharedState> {
    Router::new()
        .route("/", post(add_schedule).get(list_schedules))
        .route("/:id", patch(update_schedule).delete(delete_schedule))
        .route("/all", get(list_all))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")] 
struct OnceData { datetime: DateTime<Utc> }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecurringData {
    weekdays: Vec<Weekday>,
    time: NaiveTime,
    end_date: Option<NaiveDate>,
    #[serde(default)]
    exclude_dates: Vec<NaiveDate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleInput {
    device: String,
    action: String,
    #[serde(flatten)]
    kind: KindInput,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum KindInput {
    Once(OnceData),
    Recurring(RecurringData),
}

#[derive(Serialize)]
struct IdResp { status: &'static str, schedule_id: String }

#[derive(Serialize)]
struct StatusResp { status: &'static str }

async fn add_schedule(
    auth_header: TypedHeader<Authorization<Bearer>>, 
    State(state): State<SharedState>,
    Json(data): Json<ScheduleInput>,
) -> ApiResult<Json<IdResp>> {
    let session = auth(auth_header, &state.sessions).await?;
    let action = match data.action.as_str() { "on" => Action::On, "off" => Action::Off, _ => return Err(ApiError::new(axum::http::StatusCode::BAD_REQUEST, "invalid action")) };
    let kind = match data.kind {
        KindInput::Once(o) => ScheduleKind::Once { datetime: o.datetime },
        KindInput::Recurring(r) => ScheduleKind::Recurring { weekdays: r.weekdays, time: r.time, end_date: r.end_date, exclude_dates: r.exclude_dates },
    };
    let schedule = Schedule { id: String::new(), owner: session.email.clone(), device: data.device, action, kind };
    let id = state.scheduler.insert(schedule).await?;
    Ok(Json(IdResp{status: "ok", schedule_id: id}))
}

async fn list_schedules(auth_header: TypedHeader<Authorization<Bearer>>, State(state): State<SharedState>) -> ApiResult<Json<Vec<Schedule>>> {
    let session = auth(auth_header, &state.sessions).await?;
    let list = state.scheduler.list_owner(&session.email).await;
    Ok(Json(list))
}

async fn list_all(auth_header: TypedHeader<Authorization<Bearer>>, State(state): State<SharedState>) -> ApiResult<Json<Vec<Schedule>>> {
    let session = auth(auth_header, &state.sessions).await?;
    if session.email != "admin@yourdomain.com" { return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "권한 없음")); }
    Ok(Json(state.scheduler.list_all().await))
}


async fn update_schedule(
    auth_header: TypedHeader<Authorization<Bearer>>, 
    Path(id): Path<String>,
    State(state): State<SharedState>,
    Json(update): Json<ScheduleInput>,
) -> ApiResult<Json<StatusResp>> {
    let session = auth(auth_header, &state.sessions).await?;
    let mut schedule = state.scheduler.list_all().await.into_iter().find(|s| s.id==id).ok_or(ApiError::new(axum::http::StatusCode::NOT_FOUND, "not found"))?;
    if schedule.owner != session.email { return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "권한 없음")); }
    let action = match update.action.as_str() { "on" => Action::On, "off" => Action::Off, _ => return Err(ApiError::new(axum::http::StatusCode::BAD_REQUEST, "invalid action")) };
    let kind = match update.kind {
        KindInput::Once(o) => ScheduleKind::Once { datetime: o.datetime },
        KindInput::Recurring(r) => ScheduleKind::Recurring { weekdays: r.weekdays, time: r.time, end_date: r.end_date, exclude_dates: r.exclude_dates },
    };
    schedule.device = update.device;
    schedule.action = action;
    schedule.kind = kind;
    state.scheduler.update(&id, schedule).await?;
    Ok(Json(StatusResp{status: "updated"}))
}

async fn delete_schedule(auth_header: TypedHeader<Authorization<Bearer>>, Path(id): Path<String>, State(state): State<SharedState>) -> ApiResult<Json<StatusResp>> {
    let session = auth(auth_header, &state.sessions).await?;
    let schedule = state.scheduler.list_all().await.into_iter().find(|s| s.id==id).ok_or(ApiError::new(axum::http::StatusCode::NOT_FOUND, "not found"))?;
    if schedule.owner != session.email { return Err(ApiError::new(axum::http::StatusCode::FORBIDDEN, "권한 없음")); }
    state.scheduler.remove(&id).await?;
    Ok(Json(StatusResp{status:"deleted"}))
}
