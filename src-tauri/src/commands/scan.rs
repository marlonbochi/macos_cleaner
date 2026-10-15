//! Scan commands (US1): start, cancel and observe a scan.

use std::path::{Path, PathBuf};

use cleaner_core::cancel::CancellationToken;
use cleaner_core::domain::platform::Platform;
use cleaner_core::domain::scan::ScanId;
use cleaner_core::domain::size::ByteSize;
use cleaner_core::services::scan_service::{ScanObserver, ScanProgress};
use tauri::State;
use tauri::ipc::Channel;

use crate::dto::{ScanDefaultsDto, ScanEvent, ScanSummaryDto};
use crate::error::{AppError, CommandResult};
use crate::platform;
use crate::state::AppState;

/// Default threshold for listing individual files (spec assumption: 100 MB).
pub const DEFAULT_MIN_FILE_SIZE: ByteSize = ByteSize::from_mb(100);

/// Individual "inaccessible" events are capped; the total is still reported in progress.
const MAX_INACCESSIBLE_EVENTS: usize = 200;

/// Forwards core scan notifications to the frontend channel (Adapter + Observer).
struct ChannelObserver {
    channel: Channel<ScanEvent>,
    inaccessible_sent: usize,
}

impl ScanObserver for ChannelObserver {
    fn on_progress(&mut self, progress: &ScanProgress) {
        send(&self.channel, ScanEvent::from(progress));
    }

    fn on_inaccessible(&mut self, path: &Path) {
        if self.inaccessible_sent < MAX_INACCESSIBLE_EVENTS {
            self.inaccessible_sent += 1;
            send(
                &self.channel,
                ScanEvent::Inaccessible {
                    path: path.to_string_lossy().into_owned(),
                },
            );
        }
    }
}

fn send(channel: &Channel<ScanEvent>, event: ScanEvent) {
    // The window may have been closed; there is nothing useful to do with the error.
    if let Err(e) = channel.send(event) {
        tracing::debug!(error = %e, "scan event dropped");
    }
}

#[tauri::command]
pub fn get_scan_defaults(state: State<'_, AppState>) -> ScanDefaultsDto {
    let presets = platform::scan_presets(&state.home);
    let text = |paths: Vec<PathBuf>| -> Vec<String> {
        paths
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    };
    ScanDefaultsDto {
        roots: text(presets.default_roots),
        home: state.home.to_string_lossy().into_owned(),
        min_file_size: DEFAULT_MIN_FILE_SIZE.bytes(),
        platform: Platform::current().into(),
        applications: text(presets.applications),
        disks: text(presets.disks),
    }
}

/// Starts a scan in the background and returns immediately with the `Running` summary.
/// Progress and the final summary arrive through `on_progress`.
#[tauri::command]
pub fn start_scan(
    state: State<'_, AppState>,
    roots: Vec<String>,
    min_file_size: Option<u64>,
    on_progress: Channel<ScanEvent>,
) -> CommandResult<ScanSummaryDto> {
    let roots: Vec<PathBuf> = roots.into_iter().map(PathBuf::from).collect();
    if roots.iter().any(|r| !r.is_dir()) {
        return Err(AppError::invalid_input(
            "every scan root must be an existing folder",
        ));
    }
    let min = min_file_size.map_or(DEFAULT_MIN_FILE_SIZE, ByteSize::new);
    let scan = state.scan.begin(roots, min)?;
    let summary = ScanSummaryDto::from(&scan);
    send(
        &on_progress,
        ScanEvent::Started {
            summary: summary.clone(),
        },
    );

    let cancel = CancellationToken::new();
    state.running.set(scan.id, cancel.clone());
    let service = state.scan.clone();
    let running = state.running.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let id = scan.id;
        let mut observer = ChannelObserver {
            channel: on_progress.clone(),
            inaccessible_sent: 0,
        };
        match service.execute(scan, &cancel, &mut observer) {
            Ok(done) => {
                tracing::info!(scan = id.0, status = ?done.status, "scan finished");
                send(
                    &on_progress,
                    ScanEvent::Finished {
                        summary: ScanSummaryDto::from(&done),
                    },
                );
            }
            Err(e) => {
                tracing::error!(scan = id.0, error = %e, "scan could not be persisted");
                send(
                    &on_progress,
                    ScanEvent::Error {
                        message: e.to_string(),
                    },
                );
            }
        }
        running.clear(id);
    });
    Ok(summary)
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>, scan_id: i64) -> CommandResult<()> {
    if state.running.cancel(ScanId(scan_id)) {
        Ok(())
    } else {
        // Idempotent from the user's point of view: the scan already ended.
        tracing::debug!(scan_id, "cancel requested for a scan that is not running");
        Ok(())
    }
}

#[tauri::command]
pub fn get_latest_scan(state: State<'_, AppState>) -> CommandResult<Option<ScanSummaryDto>> {
    Ok(state
        .catalog
        .latest_scan()?
        .as_ref()
        .map(ScanSummaryDto::from))
}
