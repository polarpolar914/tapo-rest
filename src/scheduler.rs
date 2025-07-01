use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc, Weekday, Datelike};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::RwLock, time::sleep};

use crate::server::StateData;
use crate::devices::TapoDeviceInner;

#[derive(Serialize, Deserialize, Clone)]
pub enum Action {
    On,
    Off,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ScheduleKind {
    Once {
        datetime: DateTime<Utc>,
    },
    Recurring {
        weekdays: Vec<Weekday>,
        time: NaiveTime,
        end_date: Option<NaiveDate>,
        exclude_dates: Vec<NaiveDate>,
    },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Schedule {
    pub id: String,
    pub owner: String,
    pub device: String,
    pub action: Action,
    pub kind: ScheduleKind,
}

pub struct Scheduler {
    path: PathBuf,
    log_path: PathBuf,
    map: RwLock<HashMap<String, Schedule>>, // id -> schedule
}

impl Scheduler {
    pub async fn load(path: PathBuf, log_path: PathBuf) -> Result<Arc<Self>> {
        let map = if path.exists() {
            let data = fs::read_to_string(&path)
                .await
                .context("Failed to read schedules file")?;
            serde_json::from_str(&data).context("Failed to parse schedules file")?
        } else {
            HashMap::new()
        };

        Ok(Arc::new(Self { path, log_path, map: RwLock::new(map) }))
    }

    async fn save(&self, map: &HashMap<String, Schedule>) -> Result<()> {
        let data = serde_json::to_string(map).unwrap();
        fs::write(&self.path, data)
            .await
            .context("Failed to write schedules file")?;
        Ok(())
    }

    pub async fn insert(&self, mut schedule: Schedule) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        schedule.id = id.clone();
        let mut lock = self.map.write().await;
        lock.insert(id.clone(), schedule);
        self.save(&lock).await?;
        Ok(id)
    }

    pub async fn update(&self, id: &str, schedule: Schedule) -> Result<()> {
        let mut lock = self.map.write().await;
        if lock.contains_key(id) {
            lock.insert(id.to_string(), schedule);
            self.save(&lock).await?;
        }
        Ok(())
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        let mut lock = self.map.write().await;
        lock.remove(id);
        self.save(&lock).await?;
        Ok(())
    }

    pub async fn list_owner(&self, owner: &str) -> Vec<Schedule> {
        let lock = self.map.read().await;
        lock.values()
            .filter(|s| s.owner == owner)
            .cloned()
            .collect()
    }

    pub async fn list_all(&self) -> Vec<Schedule> {
        self.map.read().await.values().cloned().collect()
    }

    pub async fn run(self: Arc<Self>, state: Arc<StateData>) {
        loop {
            self.check(&state).await;
            sleep(Duration::from_secs(30)).await;
        }
    }

    async fn check(&self, state: &StateData) {
        let now = Utc::now();
        let mut to_remove = vec![];
        let map_snapshot = self.map.read().await.clone();
        for (id, sched) in map_snapshot.iter() {
            match &sched.kind {
                ScheduleKind::Once { datetime } => {
                    if now >= *datetime {
                        if let Err(e) = execute_action(&sched.action, &sched.device, state).await {
                            eprintln!("Failed to run schedule {id}: {e}");
                        } else {
                            self.log(format!("{} executed {}\n", now, id)).await;
                        }
                        to_remove.push(id.clone());
                    }
                }
                ScheduleKind::Recurring { weekdays, time, end_date, exclude_dates } => {
                    let today = now.date_naive();
                    if let Some(end) = end_date {
                        if today > *end { to_remove.push(id.clone()); continue; }
                    }
                    if exclude_dates.contains(&today) { continue; }
                    if !weekdays.contains(&now.weekday()) { continue; }
                    let now_time = now.time();
                    if now_time.hour() == time.hour() && now_time.minute() == time.minute() && now_time.second() < 30 {
                        if let Err(e) = execute_action(&sched.action, &sched.device, state).await {
                            eprintln!("Failed to run schedule {id}: {e}");
                        } else {
                            self.log(format!("{} executed {}\n", now, id)).await;
                        }
                    }
                }
            }
        }
        if !to_remove.is_empty() {
            let mut lock = self.map.write().await;
            for id in to_remove { lock.remove(&id); }
            let _ = self.save(&lock).await;
        }
    }

    async fn log(&self, text: String) {
        if let Ok(mut file) = fs::OpenOptions::new().append(true).create(true).open(&self.log_path).await {
            use tokio::io::AsyncWriteExt;
            let _ = file.write_all(text.as_bytes()).await;
        }
    }
}

use chrono::Timelike;

async fn execute_action(action: &Action, device: &str, state: &StateData) -> Result<()> {
    let device = state.devices.get(device).context("Unknown device")?;
    device
        .with_client(async move |client| -> Result<()> {
            match action {
                Action::On => match client {
                    TapoDeviceInner::L510(c) => c.on().await?,
                    TapoDeviceInner::L520(c) => c.on().await?,
                    TapoDeviceInner::L610(c) => c.on().await?,
                    TapoDeviceInner::L530(c) => c.on().await?,
                    TapoDeviceInner::L535(c) => c.on().await?,
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
                Action::Off => match client {
                    TapoDeviceInner::L510(c) => c.off().await?,
                    TapoDeviceInner::L520(c) => c.off().await?,
                    TapoDeviceInner::L610(c) => c.off().await?,
                    TapoDeviceInner::L530(c) => c.off().await?,
                    TapoDeviceInner::L535(c) => c.off().await?,
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
            }
            Ok(())
        })
        .await??;
    Ok(())
}

