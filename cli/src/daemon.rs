use crate::config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub running: bool,
    pub pid: Option<u32>,
    pub blocks_today: usize,
    pub started_at: Option<String>,
    pub last_write: Option<String>,
    pub folder: String,
    pub last_error: Option<String>,
}

pub fn write_pid() -> std::io::Result<()> {
    let path = config::pid_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, std::process::id().to_string())
}

pub fn read_pid() -> Option<u32> {
    let path = config::pid_path();
    let contents = fs::read_to_string(path).ok()?;
    contents.trim().parse().ok()
}

pub fn remove_pid() {
    let _ = fs::remove_file(config::pid_path());
}

pub fn is_running() -> bool {
    let Some(pid) = read_pid() else {
        return false;
    };
    // Check if process exists by sending signal 0
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

pub fn stop_daemon() -> std::io::Result<bool> {
    let Some(pid) = read_pid() else {
        return Ok(false);
    };

    // Send SIGTERM
    let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if result == 0 {
        // Wait briefly for process to exit
        std::thread::sleep(std::time::Duration::from_millis(200));
        remove_pid();
        Ok(true)
    } else {
        // Process doesn't exist, clean up stale PID file
        remove_pid();
        Ok(false)
    }
}

pub fn write_status(status: &Status) -> std::io::Result<()> {
    let path = config::status_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(status)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)
}

pub fn read_status() -> Option<Status> {
    let path = config::status_path();
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn init_log() -> std::io::Result<fs::File> {
    let path = config::log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Rotate existing log
    let old_path = path.with_extension("log.1");
    if path.exists() {
        let _ = fs::rename(&path, &old_path);
    }

    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
}

pub fn read_logs(lines: usize) -> std::io::Result<Vec<String>> {
    let path = config::log_path();
    if !path.exists() {
        return Ok(vec!["No log file found.".to_string()]);
    }

    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let start = all_lines.len().saturating_sub(lines);
    Ok(all_lines[start..].to_vec())
}

pub fn follow_logs() -> std::io::Result<()> {
    let path = config::log_path();
    if !path.exists() {
        println!("No log file found.");
        return Ok(());
    }

    let mut file = fs::File::open(&path)?;
    let mut reader = BufReader::new(&file);

    // Print existing content
    let mut line = String::new();
    while reader.read_line(&mut line)? > 0 {
        print!("{line}");
        line.clear();
    }

    // Follow new content
    let mut last_len = file.metadata()?.len();
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if file was rotated
        if !path.exists() {
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let current_len = metadata.len();

        if current_len < last_len {
            // File was truncated/rotated, reopen
            file = fs::File::open(&path)?;
            reader = BufReader::new(&file);
        }

        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            print!("{line}");
            line.clear();
        }

        last_len = current_len;
    }
}

pub fn log(message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = writeln!(
        std::io::stderr(),
        "[{timestamp}] {message}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serializes_to_json() {
        let status = Status {
            running: true,
            pid: Some(12345),
            blocks_today: 42,
            started_at: Some("2024-01-01 10:00:00".to_string()),
            last_write: Some("2024-01-01 10:30:00".to_string()),
            folder: "/tmp/test".to_string(),
            last_error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("42"));
        assert!(json.contains("12345"));
    }
}
