use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveTime, Datelike, Local};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::devices::TapoDeviceInner;

use super::{ApiError, ApiResult, SharedState};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    Single,
    Recurring,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Schedule {
    pub id: String,
    pub device_id: String,
    pub action: String,
    pub schedule_type: ScheduleType,
    pub time: String,
    #[serde(default)]
    pub days_of_week: Vec<String>,
    #[serde(default)]
    pub exclude_dates: Vec<String>,
    pub end_date: Option<String>,
    pub owner: String,
}

impl Schedule {
    pub fn new_single(device_id: String, action: String, time: NaiveTime) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            device_id,
            action,
            schedule_type: ScheduleType::Single,
            time: time.format("%H:%M").to_string(),
            days_of_week: vec![],
            exclude_dates: vec![],
            end_date: None,
            owner: "admin@yourdomain.com".into(),
        }
    }
}

#[derive(Default)]
pub struct ScheduleStore {
    path: PathBuf,
    pub schedules: HashMap<String, Schedule>,
}

impl ScheduleStore {
    pub async fn load(path: PathBuf) -> Result<Self> {
        let schedules = if path.exists() {
            let data = fs::read_to_string(&path)
                .await
                .context("Failed to read schedules file")?;
            serde_json::from_str(&data).context("Failed to parse schedules file")?
        } else {
            HashMap::new()
        };

        Ok(Self { path, schedules })
    }

    pub async fn save(&self) -> Result<()> {
        let data = serde_json::to_string(&self.schedules).unwrap();
        fs::write(&self.path, &data)
            .await
            .context("Failed to save schedules file")?
            ;
        Ok(())
    }

    pub async fn insert(&mut self, schedule: Schedule) -> Result<String> {
        let id = schedule.id.clone();
        self.schedules.insert(id.clone(), schedule);
        self.save().await?;
        Ok(id)
    }

    pub async fn update(&mut self, id: &str, partial: ScheduleUpdate) -> Result<()> {
        let Some(schedule) = self.schedules.get_mut(id) else { return Err(anyhow::anyhow!("schedule not found")); };
        if let Some(time) = partial.time {
            schedule.time = time;
        }
        if let Some(days) = partial.days_of_week {
            schedule.days_of_week = days;
        }
        if let Some(exclude) = partial.exclude_dates {
            schedule.exclude_dates = exclude;
        }
        if let Some(end_date) = partial.end_date {
            schedule.end_date = Some(end_date);
        }
        if let Some(action) = partial.action {
            schedule.action = action;
        }
        self.save().await
    }

    pub async fn delete(&mut self, id: &str) -> Result<()> {
        self.schedules.remove(id);
        self.save().await
    }
}

#[derive(Deserialize)]
pub struct ScheduleUpdate {
    pub time: Option<String>,
    pub days_of_week: Option<Vec<String>>,
    pub exclude_dates: Option<Vec<String>>,
    pub end_date: Option<String>,
    pub action: Option<String>,
}

pub async fn scheduler_loop(state: SharedState) {
    loop {
        check_schedules(&state).await;
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

async fn check_schedules(state: &SharedState) {
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let date_str = now.format("%Y-%m-%d").to_string();
    let weekday = now.format("%a").to_string();
    let mut to_run = vec![];

    {
        let store = state.schedules.lock().await;
        for sched in store.schedules.values() {
            if sched.time != time_str { continue; }
            if sched.exclude_dates.iter().any(|d| d == &date_str) { continue; }
            match sched.schedule_type {
                ScheduleType::Single => {
                    if date_str == sched.end_date.clone().unwrap_or_default() {
                        to_run.push(sched.clone());
                    }
                }
                ScheduleType::Recurring => {
                    if sched.days_of_week.iter().any(|d| d == &weekday) {
                        if let Some(end) = &sched.end_date {
                            if date_str > *end { continue; }
                        }
                        to_run.push(sched.clone());
                    }
                }
            }
        }
    }

    for sched in to_run {
        if let Some(device) = state.devices.get(&sched.device_id) {
            let action = sched.action.clone();
            device.with_client(async move |client| {
                match action.as_str() {
                    "on" => match client {
                        TapoDeviceInner::L510(c) => c.on().await?,
                        TapoDeviceInner::L520(c) => c.on().await?,
                        TapoDeviceInner::L530(c) => c.on().await?,
                        TapoDeviceInner::L535(c) => c.on().await?,
                        TapoDeviceInner::L610(c) => c.on().await?,
                        TapoDeviceInner::L630(c) => c.on().await?,
                        TapoDeviceInner::L900(c) => c.on().await?,
                        TapoDeviceInner::L920(c) => c.on().await?,
                        TapoDeviceInner::L930(c) => c.on().await?,
                        TapoDeviceInner::P100(c) => c.on().await?,
                        TapoDeviceInner::P105(c) => c.on().await?,
                        TapoDeviceInner::P110(c) => c.on().await?,
                        TapoDeviceInner::P110M(c) => c.on().await?,
                        TapoDeviceInner::P115(c) => c.on().await?,
                    },
                    "off" => match client {
                        TapoDeviceInner::L510(c) => c.off().await?,
                        TapoDeviceInner::L520(c) => c.off().await?,
                        TapoDeviceInner::L530(c) => c.off().await?,
                        TapoDeviceInner::L535(c) => c.off().await?,
                        TapoDeviceInner::L610(c) => c.off().await?,
                        TapoDeviceInner::L630(c) => c.off().await?,
                        TapoDeviceInner::L900(c) => c.off().await?,
                        TapoDeviceInner::L920(c) => c.off().await?,
                        TapoDeviceInner::L930(c) => c.off().await?,
                        TapoDeviceInner::P100(c) => c.off().await?,
                        TapoDeviceInner::P105(c) => c.off().await?,
                        TapoDeviceInner::P110(c) => c.off().await?,
                        TapoDeviceInner::P110M(c) => c.off().await?,
                        TapoDeviceInner::P115(c) => c.off().await?,
                    },
                    _ => {}
                }
                Ok::<(), anyhow::Error>(())
            }).await.ok();
        }
        let log_line = format!(
            "[{}] 예약 실행: ID={} 디바이스={} 동작={}\n",
            now.to_rfc3339(),
            sched.id,
            sched.device_id,
            sched.action.to_uppercase()
        );
        if let Ok(mut f) = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&state.schedule_log)
            .await
        {
            use tokio::io::AsyncWriteExt;
            let _ = f.write_all(log_line.as_bytes()).await;
        }
    }
}

