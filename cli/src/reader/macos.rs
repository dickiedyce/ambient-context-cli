#![cfg(target_os = "macos")]

use super::{parse_snapshot_json, permission_from_code, Permission, Snapshot};
use std::path::Path;
use std::process::Command;

fn find_helper(explicit_path: Option<&Path>) -> Option<std::path::PathBuf> {
    // 1. Explicit config path
    if let Some(path) = explicit_path {
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }

    // 2. Same directory as current binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("ambient-context-ax");
            if sibling.exists() {
                return Some(sibling);
            }
        }
    }

    // 3. On PATH
    if let Ok(output) = Command::new("which").arg("ambient-context-ax").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }

    None
}

fn run_helper(command: &str, helper_path: Option<&Path>) -> Result<String, String> {
    let path = find_helper(helper_path)
        .ok_or("ambient-context-ax helper not found. Install it or set ax_helper_path in config.")?;

    let output = Command::new(&path)
        .arg(command)
        .output()
        .map_err(|e| format!("failed to run helper: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("helper failed: {stderr}"));
    }

    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("helper output not valid UTF-8: {e}"))
}

pub fn permission_status(helper_path: Option<&Path>) -> Permission {
    match run_helper("permission", helper_path) {
        Ok(code) => {
            let code: i32 = code.parse().unwrap_or(0);
            permission_from_code(code)
        }
        Err(_) => Permission::NotGranted,
    }
}

#[allow(dead_code)]
pub fn request_permission(helper_path: Option<&Path>) -> Permission {
    match run_helper("request-permission", helper_path) {
        Ok(code) => {
            let code: i32 = code.parse().unwrap_or(0);
            permission_from_code(code)
        }
        Err(_) => Permission::NotGranted,
    }
}

pub fn snapshot(helper_path: Option<&Path>) -> Option<Snapshot> {
    let raw = run_helper("snapshot", helper_path).ok()?;
    match parse_snapshot_json(&raw) {
        Ok(snapshot) => Some(snapshot),
        Err(message) => {
            eprintln!("[ax] snapshot failed: {message}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_helper_returns_none_when_not_found() {
        // In test environment, helper likely doesn't exist
        let result = find_helper(Some(Path::new("/nonexistent/path")));
        assert!(result.is_none());
    }
}
