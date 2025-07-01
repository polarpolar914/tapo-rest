use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;

use crate::devices::TapoDevice;
use crate::scheduler::Scheduler;

use super::sessions::Sessions;

pub struct StateData {
    pub auth_password: String,
    pub devices: HashMap<String, TapoDevice>,
    pub sessions: Sessions,
    pub scheduler: std::sync::Arc<Scheduler>,
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
            scheduler: Scheduler::load(schedules_file, schedule_log).await?,
        })
    }
}
