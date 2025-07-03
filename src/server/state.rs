use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

use crate::devices::TapoDevice;

use super::{sessions::Sessions, schedules::ScheduleStore};

pub struct StateData {
    pub auth_password: String,
    pub devices: HashMap<String, TapoDevice>,
    pub sessions: Sessions,
    pub schedules: tokio::sync::Mutex<ScheduleStore>,
    pub schedule_log: PathBuf,
}

impl StateData {
    pub async fn init(
        auth_password: String,
        devices: Vec<TapoDevice>,
        sessions_file: PathBuf,
        schedules_file: PathBuf,
        schedule_log: PathBuf,
    ) -> Result<Self> {
        Ok(Self {
            auth_password,
            devices: devices
                .into_iter()
                .map(|device| (device.name().to_owned(), device))
                .collect(),
            sessions: Sessions::create(sessions_file).await?,
            schedules: tokio::sync::Mutex::new(ScheduleStore::load(schedules_file).await?),
            schedule_log,
        })
    }

    pub async fn schedules_mut(&self) -> tokio::sync::MutexGuard<'_, ScheduleStore> {
        self.schedules.lock().await
    }
}
