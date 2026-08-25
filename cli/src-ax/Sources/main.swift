import Foundation
import AppKit
import ApplicationServices

// Status codes crossing the C boundary: 0 = not granted, 1 = granted.

/// Reads the current Accessibility trust state. Does not prompt.
func permissionStatus() -> Int32 {
    return AXIsProcessTrusted() ? 1 : 0
}

/// Raises the system prompt if the app is not yet trusted.
func requestPermission() -> Int32 {
    let key = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String
    let options = [key: true] as CFDictionary
    return AXIsProcessTrustedWithOptions(options) ? 1 : 0
}

// Content in browsers and Electron apps nests 15 to 30 levels deep
private let maxDepth = 40
private let maxElements = 2000
private let maxVisited = 20000

private func copyAttribute(_ element: AXUIElement, _ attribute: String) -> CFTypeRef? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, attribute as CFString, &value) == .success else {
        return nil
    }
    return value
}

private func isSecure(_ element: AXUIElement) -> Bool {
    if let role = copyAttribute(element, kAXRoleAttribute as String) as? String,
       role == "AXSecureTextField" {
        return true
    }
    if let subrole = copyAttribute(element, kAXSubroleAttribute as String) as? String,
       subrole == (kAXSecureTextFieldSubrole as String) {
        return true
    }
    return false
}

private func collectText(
    from element: AXUIElement,
    into texts: inout [String],
    depth: Int,
    visited: inout Int,
    webUrl: inout String?
) {
    visited += 1
    if depth > maxDepth || texts.count >= maxElements || visited > maxVisited { return }

    if isSecure(element) { return }

    if webUrl == nil,
       let role = copyAttribute(element, kAXRoleAttribute as String) as? String,
       role == "AXWebArea",
       let url = copyAttribute(element, "AXURL") {
        webUrl = (url as? NSURL)?.absoluteString
    }

    for attribute in [kAXValueAttribute as String, kAXTitleAttribute as String] {
        if let value = copyAttribute(element, attribute) as? String {
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty && trimmed.count < 8000 {
                texts.append(trimmed)
            }
        }
    }

    if let children = copyAttribute(element, kAXChildrenAttribute as String) as? [AXUIElement] {
        for child in children {
            collectText(from: child, into: &texts, depth: depth + 1, visited: &visited, webUrl: &webUrl)
        }
    }
}

// Chromium accessibility enabling
private let enabledPidsLock = NSLock()
private var enabledPids = Set<pid_t>()

private func enableAccessibilityOnce(_ pid: pid_t, _ appElement: AXUIElement, _ appName: String) {
    enabledPidsLock.lock()
    let alreadyTried = enabledPids.contains(pid)
    if !alreadyTried { enabledPids.insert(pid) }
    enabledPidsLock.unlock()
    if alreadyTried { return }

    if AXUIElementSetAttributeValue(
        appElement, "AXManualAccessibility" as CFString, kCFBooleanTrue) == .success {
        FileHandle.standardError.write(Data("[ax] enabled AXManualAccessibility for \(appName)\n".utf8))
        return
    }
    if AXUIElementSetAttributeValue(
        appElement, "AXEnhancedUserInterface" as CFString, kCFBooleanTrue) == .success {
        FileHandle.standardError.write(Data("[ax] enabled AXEnhancedUserInterface for \(appName)\n".utf8))
    }
}

/// Takes a snapshot of the focused window and prints JSON to stdout.
func snapshot() {
    guard AXIsProcessTrusted() else {
        print("ERROR: permission not granted")
        return
    }

    if let session = CGSessionCopyCurrentDictionary() as? [String: Any],
       session["CGSSessionScreenIsLocked"] as? Bool == true {
        print("ERROR: screen locked")
        return
    }

    guard let frontmost = NSWorkspace.shared.frontmostApplication else {
        print("ERROR: no frontmost application")
        return
    }

    let appName = frontmost.localizedName ?? "Unknown"
    let appElement = AXUIElementCreateApplication(frontmost.processIdentifier)

    AXUIElementSetMessagingTimeout(appElement, 0.5)

    enableAccessibilityOnce(frontmost.processIdentifier, appElement, appName)

    guard let focused = copyAttribute(appElement, kAXFocusedWindowAttribute as String) else {
        print("ERROR: no focused window")
        return
    }
    let window = focused as! AXUIElement

    let title = copyAttribute(window, kAXTitleAttribute as String) as? String

    var document = copyAttribute(window, "AXDocument") as? String
    if document == nil, let docUrl = copyAttribute(window, "AXDocument") {
        document = (docUrl as? NSURL)?.absoluteString
    }

    var texts: [String] = []
    var visited = 0
    var webUrl: String? = nil
    collectText(from: window, into: &texts, depth: 0, visited: &visited, webUrl: &webUrl)

    let payload: [String: Any] = [
        "app": appName,
        "window_title": title as Any,
        "document": document as Any,
        "url": webUrl as Any,
        "text": texts
    ]

    guard let data = try? JSONSerialization.data(withJSONObject: payload),
          let json = String(data: data, encoding: .utf8) else {
        print("ERROR: could not serialise snapshot")
        return
    }
    print(json)
}

// Main command handling
let args = CommandLine.arguments
guard args.count > 1 else {
    FileHandle.standardError.write(Data("Usage: ambient-context-ax <command>\nCommands: permission, request-permission, snapshot\n".utf8))
    exit(1)
}

let command = args[1]
switch command {
case "permission":
    print(permissionStatus())
case "request-permission":
    print(requestPermission())
case "snapshot":
    snapshot()
default:
    FileHandle.standardError.write(Data("Unknown command: \(command)\n".utf8))
    exit(1)
}
