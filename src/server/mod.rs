use std::{fs, path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use log::info;
use tokio::net::TcpListener;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use crate::{
    cmd::{PasswordArgGroup, ServerConfig},
    devices::TapoDevice,
    server::actions::make_router,
};

use self::{
    sessions::refresh_session,
    state::StateData,
    schedules::{scheduler_loop},
    schedules_api::router as schedules_router,
};

mod actions;
mod auth;
mod errors;
mod sessions;
mod state;
mod schedules;
mod schedules_api;

pub use actions::TapoDeviceType;
pub use errors::{ApiError, ApiResult};

pub type SharedState = Arc<StateData>;

pub async fn serve(
    config: ServerConfig,
    devices: Vec<TapoDevice>,
    sessions_file: PathBuf,
    schedules_file: PathBuf,
    schedule_log: PathBuf,
) -> Result<()> {
    let ServerConfig { port, password } = config;

    let PasswordArgGroup {
        auth_password,
        password_from_file,
    } = password;

    let auth_password = match (auth_password, password_from_file) {
        (Some(auth_password), None) => auth_password,

        (None, Some(file)) => {
            if !file.is_file() {
                bail!(
                    "Provided file password path does not exist: {}",
                    file.display()
                );
            }

            fs::read_to_string(&file).with_context(|| {
                format!("Failed to read file password at path: {}", file.display())
            })?
        }

        (Some(_), Some(_)) | (None, None) => unreachable!(),
    };

    let cors = CorsLayer::new()
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .allow_origin(
            // TODO: make this configurable
            AllowOrigin::any(),
        );

    let state = Arc::new(
        StateData::init(
            auth_password,
            devices,
            sessions_file,
            schedules_file,
            schedule_log,
        )
        .await?,
    );

    tokio::spawn(scheduler_loop(state.clone()));

    let app = Router::new()
        .route("/login", post(auth::login))
        .route("/refresh-session", get(refresh_session))
        .nest("/actions", make_router())
        .merge(schedules_router())
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");

    info!("Launching server on {addr}...");

    let tcp_listener = TcpListener::bind(addr).await?;

    axum::serve(tcp_listener, app.into_make_service())
        .await
        .map_err(Into::into)
}
