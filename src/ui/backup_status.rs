use chrono::prelude::*;

use crate::borg;
use crate::borg::log_json;
use crate::borg::Run;
use crate::config::history;
use crate::config::*;
use crate::ui;
use crate::ui::prelude::*;
use crate::ui::utils;

#[derive(Debug)]
pub struct Display {
    pub title: String,
    pub subtitle: Option<String>,
    pub graphic: Graphic,
    pub progress: Option<f64>,
    pub stats: Option<Stats>,
}

#[derive(Debug)]
pub enum Stats {
    Progress(log_json::ProgressArchive),
    Final(history::RunInfo),
}

#[derive(Debug)]
pub enum Graphic {
    OkIcon(String),
    WarningIcon(String),
    ErrorIcon(String),
    Spinner,
}

impl Display {
    pub fn new_from_id(config_id: &ConfigId) -> Self {
        BORG_OPERATION.with(|operations| {
            if let Some(op) = operations.load().get(config_id) {
                Self::from(op.as_ref())
            } else if let Some(last_run) = BACKUP_HISTORY
                .load()
                .get_result(config_id)
                .ok()
                .and_then(|x| x.run.get(0))
            {
                Self::from(last_run)
            } else {
                Self::default()
            }
        })
    }
}

impl From<&history::RunInfo> for Display {
    fn from(run_info: &history::RunInfo) -> Self {
        match run_info.outcome {
            borg::Outcome::Completed { .. }
                if run_info.messages.clone().filter_handled().max_log_level()
                    > Some(log_json::LogLevel::Info) =>
            {
                Self {
                    title: gettext("Last backup completed with warnings"),
                    subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                    graphic: Graphic::WarningIcon("dialog-warning-symbolic".to_string()),
                    progress: None,
                    stats: Some(Stats::Final(run_info.clone())),
                }
            }
            borg::Outcome::Completed { .. } => Self {
                title: gettext("Last backup successful"),
                subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                graphic: Graphic::OkIcon("emblem-default-symbolic".to_string()),
                progress: None,
                stats: Some(Stats::Final(run_info.clone())),
            },
            borg::Outcome::Aborted(borg::error::Abort::User) => Self {
                title: gettext("Last backup aborted"),
                subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                graphic: Graphic::WarningIcon("dialog-warning-symbolic".to_string()),
                progress: None,
                stats: Some(Stats::Final(run_info.clone())),
            },
            _ => Self {
                title: gettext("Last backup failed"),
                subtitle: Some(utils::duration::ago(&(Local::now() - run_info.end))),
                graphic: Graphic::ErrorIcon("dialog-error-symbolic".to_string()),
                progress: None,
                stats: Some(Stats::Final(run_info.clone())),
            },
        }
    }
}

impl From<&dyn ui::operation::OperationExt> for Display {
    fn from(op: &dyn ui::operation::OperationExt) -> Self {
        if let Some(op_create) = op.try_as_create() {
            Self::from(op_create)
        } else {
            Default::default()
        }
    }
}

impl From<&ui::operation::Operation<borg::task::Create>> for Display {
    fn from(op: &ui::operation::Operation<borg::task::Create>) -> Self {
        let status = op.communication().specific_info.get();

        let mut progress = None;
        let mut stats = None;
        let mut subtitle = None;

        if let Some(ref last_message) = op.last_log() {
            match last_message.as_ref() {
                log_json::Output::Progress(log_json::Progress::Archive(
                    ref progress_archive_ref,
                )) => {
                    stats = Some(Stats::Progress(progress_archive_ref.clone()));
                    if let Some(size) = &status.estimated_size {
                        let fraction =
                            progress_archive_ref.original_size as f64 / size.total as f64;
                        progress = Some(fraction);

                        let mut sub = gettextf(
                            // xgettext:no-c-format
                            "{} % finished",
                            &[&format!("{:.1}", fraction * 100.0)],
                        );

                        // Do not show estimate when stalled for example
                        if matches!(op.communication().status(), borg::status::Run::Running) {
                            if let Some(remaining) = status.time_remaining() {
                                sub.push_str(&format!(" – {}", utils::duration::left(&remaining)));
                            }
                        }

                        subtitle = Some(sub);
                    }
                }
                ref other_message => {
                    subtitle = Some(other_message.to_string());
                }
            }
        }

        let title = match op.communication().status() {
            Run::Init => gettext("Preparing backup"),
            Run::Running => gettext("Backup running"),
            Run::Stalled => gettext("Backup destination unresponsive"),
            Run::Reconnecting => {
                subtitle = Some(gettextf(
                    "Connection lost, reconnecting in {}",
                    &[&utils::duration::plain(&utils::duration::from_std(
                        borg::DELAY_RECONNECT,
                    ))],
                ));
                gettext("Reconnecting")
            }
            Run::Stopping => gettext("Stopping backup"),
        };

        if subtitle.is_none() {
            if let Some(log) = op.last_log() {
                subtitle = Some(log.to_string());
            }
        }

        Self {
            title,
            subtitle,
            graphic: Graphic::Spinner,
            progress,
            stats,
        }
    }
}

impl Default for Display {
    fn default() -> Self {
        Self {
            title: gettext("Backup never ran"),
            subtitle: Some(gettext("Start by creating your first backup")),
            graphic: Graphic::WarningIcon("dialog-information-symbolic".to_string()),
            progress: None,
            stats: None,
        }
    }
}
