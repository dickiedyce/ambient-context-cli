use crate::{capture, config, daemon, reader, writer};
use chrono::Local;
use std::path::PathBuf;

pub fn start(folder_override: Option<String>) -> Result<(), String> {
    // Check if already running
    if daemon::is_running() {
        let pid = daemon::read_pid().unwrap_or(0);
        return Err(format!("already running (PID {pid})"));
    }

    // Load config
    let mut cfg = config::load();

    // Apply folder override
    if let Some(folder) = folder_override {
        cfg.folder = PathBuf::from(folder);
    }

    // Pre-flight checks
    let helper_path = cfg.ax_helper_path.clone();

    // Check accessibility permission
    let permission = reader::macos::permission_status(helper_path.as_deref());
    if permission == reader::Permission::NotGranted {
        return Err(
            "accessibility permission not granted. Grant permission in System Settings > Privacy & Security > Accessibility.".to_string()
        );
    }

    // Check helper binary
    if reader::macos::snapshot(helper_path.as_deref()).is_none() {
        // Try to find the helper to give a better error
        let helper_found = std::process::Command::new("which")
            .arg("ambient-context-ax")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !helper_found && helper_path.is_none() {
            return Err(
                "ambient-context-ax helper not found. Install it or set ax_helper_path in ~/.config/ambient-context/config.toml".to_string()
            );
        }
    }

    // Check/create folder
    if !cfg.folder.exists() {
        std::fs::create_dir_all(&cfg.folder)
            .map_err(|e| format!("cannot create folder: {e}"))?;
    }

    // Warn about iCloud
    if config::is_icloud_path(&cfg.folder) {
        eprintln!("warning: folder is inside iCloud Drive. Files will be uploaded to Apple's servers.");
    }

    // Initialize daemon
    daemon::write_pid().map_err(|e| format!("cannot write PID file: {e}"))?;
    let _log_file = daemon::init_log().map_err(|e| format!("cannot initialize log: {e}"))?;

    // Print confirmation
    let pid = std::process::id();
    let log_path = config::log_path();
    println!("Started (PID {pid}). Logs: {}", log_path.display());

    // Start capture
    let folder = cfg.folder.clone();
    let state = capture::CaptureState::new();
    capture::start(&state, cfg);

    // Write AGENTS.md
    let _ = writer::ensure_agents_file(&folder);

    // Set up signal handler
    let running = state.running_flag();
    ctrlc::set_handler(move || {
        running.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .map_err(|e| format!("cannot set signal handler: {e}"))?;

    // Wait for capture to finish
    while state.is_running() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Cleanup
    daemon::remove_pid();
    Ok(())
}

pub fn stop() -> Result<(), String> {
    if !daemon::is_running() {
        println!("Not running.");
        return Ok(());
    }

    match daemon::stop_daemon() {
        Ok(true) => {
            println!("Stopped.");
            Ok(())
        }
        Ok(false) => {
            println!("Not running (stale PID file cleaned up).");
            Ok(())
        }
        Err(e) => Err(format!("failed to stop: {e}")),
    }
}

pub fn status(json: bool) -> Result<(), String> {
    let status = daemon::read_status().unwrap_or(daemon::Status {
        running: daemon::is_running(),
        pid: daemon::read_pid(),
        blocks_today: 0,
        started_at: None,
        last_write: None,
        folder: config::load().folder.to_string_lossy().to_string(),
        last_error: None,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&status).unwrap());
    } else {
        let running = if status.running { "running" } else { "stopped" };
        println!("Status:    {running}");
        if let Some(pid) = status.pid {
            println!("PID:       {pid}");
        }
        println!("Blocks:    {} today", status.blocks_today);
        println!("Folder:    {}", status.folder);
        if let Some(started) = &status.started_at {
            println!("Started:   {started}");
        }
        if let Some(last) = &status.last_write {
            println!("Last write: {last}");
        }
        if let Some(err) = &status.last_error {
            println!("Last error: {err}");
        }
    }

    Ok(())
}

pub fn today() -> Result<(), String> {
    let cfg = config::load();
    let today = Local::now().date_naive();
    let path = writer::file_path(&cfg.folder, today);

    // Create folder if needed
    if !cfg.folder.exists() {
        std::fs::create_dir_all(&cfg.folder)
            .map_err(|e| format!("cannot create folder: {e}"))?;
    }

    // Create file with frontmatter if it doesn't exist
    if !path.exists() {
        let mut dedup = writer::DayDedup::new();
        let block = crate::segment::Block {
            app: String::new(),
            title: None,
            document: None,
            url: None,
            start: Local::now(),
            end: Local::now(),
            lines: vec![],
        };
        writer::append_block(&cfg.folder, &block, &mut dedup)
            .map_err(|e| format!("cannot create file: {e}"))?;
    }

    println!("{}", path.display());
    Ok(())
}

pub fn snapshot(unredacted: bool) -> Result<(), String> {
    let cfg = config::load();
    let helper_path = cfg.ax_helper_path.clone();

    let permission = reader::macos::permission_status(helper_path.as_deref());
    if permission == reader::Permission::NotGranted {
        return Err(
            "accessibility permission not granted. Grant permission in System Settings > Privacy & Security > Accessibility.".to_string()
        );
    }

    let snapshot = reader::macos::snapshot(helper_path.as_deref())
        .ok_or("failed to take snapshot. Is the helper binary installed?")?;

    if unredacted {
        println!("App:       {}", snapshot.app);
        if let Some(title) = &snapshot.window_title {
            println!("Title:     {title}");
        }
        if let Some(doc) = &snapshot.document {
            println!("Document:  {doc}");
        }
        if let Some(url) = &snapshot.url {
            println!("URL:       {url}");
        }
        println!("Elements:  {}", snapshot.text.len());
        println!();
        for line in &snapshot.text {
            println!("{line}");
        }
    } else {
        let redacted = crate::redact::redact_snapshot(snapshot)
            .ok_or("snapshot dropped (excluded app or private window)")?;

        println!("App:       {}", redacted.app);
        if let Some(title) = &redacted.window_title {
            println!("Title:     {title}");
        }
        if let Some(doc) = &redacted.document {
            println!("Document:  {doc}");
        }
        if let Some(url) = &redacted.url {
            println!("URL:       {url}");
        }
        println!("Elements:  {}", redacted.text.len());
        println!();
        for line in &redacted.text {
            println!("{line}");
        }
    }

    Ok(())
}

pub fn logs(follow: bool) -> Result<(), String> {
    if follow {
        daemon::follow_logs().map_err(|e| format!("cannot follow logs: {e}"))
    } else {
        let lines = daemon::read_logs(50).map_err(|e| format!("cannot read logs: {e}"))?;
        for line in lines {
            println!("{line}");
        }
        Ok(())
    }
}
