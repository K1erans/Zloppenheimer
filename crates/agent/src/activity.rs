use crate::ThreadsDatabase;
use acp_thread::{AcpThread, AcpThreadEvent, ThreadStatus};
use anyhow::{Result, anyhow};
use chrono::{Days, Local, NaiveDate};
use futures::{FutureExt, future::Shared};
use gpui::{App, AppContext, Context, Entity, Global, Task};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use util::ResultExt;

#[derive(Clone, Debug)]
pub(crate) struct ActivityRecord {
    pub day: NaiveDate,
    pub session: String,
    pub tokens: u64,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ActivitySnapshot {
    pub daily_tokens: BTreeMap<NaiveDate, u64>,
    pub lifetime_tokens: u64,
    pub peak_tokens: u64,
    pub longest_chat_seconds: u64,
    pub current_streak: u64,
    pub longest_streak: u64,
    pub first_recorded_day: Option<NaiveDate>,
    pub loading: bool,
    pub error: Option<String>,
}

struct GlobalActivityStore(Entity<ActivityStore>);
impl Global for GlobalActivityStore {}

pub struct ActivityStore {
    records: BTreeMap<(NaiveDate, String), ActivityRecord>,
    database: Option<Arc<ThreadsDatabase>>,
    pending: Vec<ActivityRecord>,
    saves: Vec<Shared<Task<()>>>,
    loading: bool,
    error: Option<String>,
}

impl ActivityStore {
    pub fn init_global(cx: &mut App) {
        if Self::try_global(cx).is_some() {
            return;
        }
        let store = cx.new(|cx: &mut Context<Self>| {
            cx.on_app_quit(|this, _cx| {
                let saves = std::mem::take(&mut this.saves);
                async move {
                    futures::future::join_all(saves).await;
                }
            })
            .detach();
            let database = ThreadsDatabase::connect(cx);
            cx.spawn(async move |this, cx| {
                let result: Result<_> = async {
                    let database = database.await.map_err(|error| anyhow!(error))?;
                    let records = database.load_activity().await?;
                    Ok((database, records))
                }
                .await;
                this.update(cx, |this, cx| {
                    this.loading = false;
                    match result {
                        Ok((database, records)) => {
                            for record in records {
                                this.merge(record);
                            }
                            this.database = Some(database);
                            for record in std::mem::take(&mut this.pending) {
                                this.persist(record, cx);
                            }
                        }
                        Err(error) => {
                            log::error!("Could not load agent activity: {error:#}");
                            this.error = Some(format!("Could not load activity: {error:#}"));
                        }
                    }
                    cx.notify();
                })
                .log_err();
            })
            .detach();
            Self {
                records: BTreeMap::new(),
                database: None,
                pending: Vec::new(),
                saves: Vec::new(),
                loading: true,
                error: None,
            }
        });
        cx.set_global(GlobalActivityStore(store));
        cx.observe_new::<AcpThread>(|thread, _, cx| {
            if thread.parent_session_id().is_some() {
                return;
            }
            let mut started_at = None;
            cx.subscribe_self(move |thread, event: &AcpThreadEvent, cx| {
                if !matches!(event, AcpThreadEvent::StatusChanged) {
                    return;
                }
                let session = format!(
                    "{}:{}",
                    thread.connection().telemetry_id(),
                    thread.session_id()
                );
                match thread.status() {
                    ThreadStatus::Generating if started_at.is_none() => {
                        started_at = Some(Instant::now());
                        Self::record_tokens(session, 0, cx);
                    }
                    ThreadStatus::Idle => {
                        if let Some(started_at) = started_at.take()
                            && let Some(store) = Self::try_global(cx)
                        {
                            let duration_ms =
                                u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
                            store.update(cx, |store, cx| {
                                store.record(
                                    ActivityRecord {
                                        day: Local::now().date_naive(),
                                        session,
                                        tokens: 0,
                                        duration_ms,
                                    },
                                    cx,
                                )
                            });
                        }
                    }
                    _ => {}
                }
            })
            .detach();
        })
        .detach();
    }

    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalActivityStore>()
            .map(|store| store.0.clone())
    }

    pub(crate) fn record_tokens(session: String, tokens: u64, cx: &mut App) {
        if let Some(store) = Self::try_global(cx) {
            store.update(cx, |store, cx| {
                store.record(
                    ActivityRecord {
                        day: Local::now().date_naive(),
                        session,
                        tokens,
                        duration_ms: 0,
                    },
                    cx,
                )
            });
        }
    }

    fn merge(&mut self, record: ActivityRecord) {
        self.records
            .entry((record.day, record.session.clone()))
            .and_modify(|previous| {
                previous.tokens = previous.tokens.saturating_add(record.tokens);
                previous.duration_ms = previous.duration_ms.saturating_add(record.duration_ms);
            })
            .or_insert(record);
    }

    fn record(&mut self, record: ActivityRecord, cx: &mut Context<Self>) {
        self.merge(record.clone());
        if self.database.is_some() {
            self.persist(record, cx);
        } else {
            self.pending.push(record);
        }
        cx.notify();
    }

    fn persist(&mut self, record: ActivityRecord, cx: &mut Context<Self>) {
        let Some(database) = &self.database else {
            return;
        };
        let save = database.record_activity(record);
        let task = cx
            .spawn(async move |this, cx| {
                if let Err(error) = save.await {
                    log::error!("Could not save agent activity: {error:#}");
                    this.update(cx, |this, cx| {
                        this.error = Some(format!("Could not save activity: {error:#}"));
                        cx.notify();
                    })
                    .log_err();
                }
            })
            .shared();
        self.saves.retain(|task| task.peek().is_none());
        self.saves.push(task);
    }

    pub fn snapshot(&self) -> ActivitySnapshot {
        let mut snapshot = ActivitySnapshot {
            loading: self.loading,
            error: self.error.clone(),
            ..Default::default()
        };
        let mut sessions = BTreeMap::<&str, u64>::new();
        for record in self.records.values() {
            let tokens = snapshot.daily_tokens.entry(record.day).or_default();
            *tokens = tokens.saturating_add(record.tokens);
            let duration = sessions.entry(&record.session).or_default();
            *duration = duration.saturating_add(record.duration_ms);
        }
        snapshot.lifetime_tokens = snapshot
            .daily_tokens
            .values()
            .fold(0u64, |total, tokens| total.saturating_add(*tokens));
        snapshot.peak_tokens = snapshot
            .daily_tokens
            .values()
            .copied()
            .max()
            .unwrap_or_default();
        snapshot.longest_chat_seconds = sessions.values().copied().max().unwrap_or_default() / 1000;
        snapshot.first_recorded_day = snapshot.daily_tokens.keys().next().copied();
        let mut previous_day: Option<NaiveDate> = None;
        let mut streak = 0;
        for day in snapshot.daily_tokens.keys().copied() {
            streak = if previous_day.and_then(|day| day.checked_add_days(Days::new(1))) == Some(day)
            {
                streak + 1
            } else {
                1
            };
            snapshot.longest_streak = snapshot.longest_streak.max(streak);
            previous_day = Some(day);
        }
        let today = Local::now().date_naive();
        if previous_day == Some(today) || previous_day == today.checked_sub_days(Days::new(1)) {
            snapshot.current_streak = streak;
        }
        snapshot
    }
}
