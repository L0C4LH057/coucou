# Architecture & Technical Research: Bringing Coucou to Ubuntu Linux (24.04 LTS to 26.04 LTS)

## 1. Executive Summary

**Coucou** is currently native to macOS (Swift/AppKit/SwiftUI) and Windows (Tauri 2 / Rust / TS). 

To achieve seamless, elegant, and native integration on **Ubuntu Linux (24.04 LTS Noble Numbat to 26.04 LTS Resilient Ringtail)** using **GNOME 46 through 50+**, we leverage **Tauri 2 (Rust + TypeScript frontend)**.

This document provides the exhaustive architectural blueprint and implementation plan for Linux compatibility.

---

## 2. Linux Desktop Environment & Compositor Strategy

### 2.1 Display Server Architectures (Wayland vs. X11)
Ubuntu 24.04 LTS defaults to **GNOME on Wayland** for Intel/AMD/NVIDIA GPUs, with X11 as a fallback option. 

| Feature | X11 | Wayland (Mutter / GNOME) | Coucou Linux Strategy |
|---|---|---|---|
| **Window Positioning** | `XMoveWindow` / Arbitrary absolute placement | Restricted by compositor | Position relative to primary monitor bounds via Tauri window geometry APIs |
| **Always On Top** | `_NET_WM_STATE_ABOVE` | `xdg-toplevel` / Mutter stack order | Set `alwaysOnTop: true` & `skipTaskbar: true` |
| **Frameless / Transparent** | Composite extensions | Native ARGB buffer support | `decorations: false`, `transparent: true` in Tauri window config |
| **Mouse Click-Through** | `XShapeCombineMask` | GTK Input Regions (`gdk_window_input_shape_combine_region`) | Conditional `set_ignore_cursor_events(!accept)` |
| **Cursor Position Query** | `XQueryPointer` | Security-restricted | Polled relative to window origin + GTK device pointer querying |

### 2.2 GNOME Shell Integration (GNOME 46 to 50)
On GNOME Desktop, the top bar occupies the top $24\text{px} - 32\text{px}$ of the screen.

```
       +-------------------------------------------------------------+
       |   Top Bar (GNOME Shell) - Status Indicators / System Tray   |
       +-------------------------------------------------------------+
       |                      [ Mochi Island ]                       |
       |                   (Floats at Top-Center)                    |
       |                                                             |
       |                      Active Workspace                       |
       +-------------------------------------------------------------+
```

1. **Top-Edge Docking**: 
   - Floating centered directly below or overlapping the top panel edge.
   - Expandable panel down to $320\text{px}$ height.
2. **AppIndicator / StatusNotifierItem Tray**:
   - Uses Tauri's `tray-icon` feature.
   - Compatible with Ubuntu's pre-installed `gnome-shell-extension-appindicator` (default on Ubuntu 24.04+).

---

## 3. Subsystem Architectural Mapping

| Subsystem | macOS Implementation | Windows Implementation | **Linux Implementation Strategy** |
|---|---|---|---|
| **Window & Panel** | Native `NSPanel` | Tauri 2 transparent window + Win32 `WS_EX_NOACTIVATE` | Tauri 2 transparent window + GTK3/4 window hints |
| **IPC Relay Socket** | Unix Domain Socket (`~/.Library/Application Support/NotchBuddy/nb.sock`) | Windows Named Pipe (`\\.\pipe\coucou-<sid>`) | **Unix Domain Socket** (`$XDG_RUNTIME_DIR/coucou.sock` or `~/.config/coucou/coucou.sock`) |
| **Relay Binary** | Swift script / binary `nb-hook` | Rust binary `coucou-hook.exe` | **Rust binary `coucou-hook`** (POSIX executable) |
| **Secrets Storage** | macOS Keychain | Windows Credential Manager | **Linux Secret Service API** via `keyring` crate (GNOME Keyring / KWallet / KeePassXC) |
| **Terminal Integration** | AppleScript / VS Code | VS Code / Explorer | `code` (VS Code), `gnome-terminal`, `ptyxis` (GNOME 47+ terminal), `xterm` |
| **Autostart** | LaunchAgent | Windows Registry (`Run`) | XDG Autostart Specification (`~/.config/autostart/fr.louisraille.coucou.desktop`) |
| **Packaging & Dist** | `.app` / `.dmg` | NSIS `.exe` installer | **AppImage**, **Debian (`.deb`)**, and **Flatpak** |

---

## 4. Implementation Blueprint

### 4.1 Refactoring Project Directory Structure
To support cross-platform builds elegantly, we structure the Tauri backend to conditionally compile OS-specific implementations:

```
windows/             --> renamed or structured as `desktop/` or multi-target Tauri workspace
  src-tauri/
    src/
      lib.rs          --> OS-agnostic command dispatcher & setup
      secrets.rs      --> Cross-platform keyring abstraction (`keyring` crate)
      pipe/
        mod.rs        --> Async IPC trait / module
        windows.rs    --> Windows Named Pipes
        unix.rs       --> Linux Unix Domain Sockets
      island/
        mod.rs        --> Geometry & click-through dispatcher
        windows.rs    --> Win32 API cursor poll & EX_STYLE
        linux.rs      --> GTK / X11 / Wayland geometry & event handling
      hooks.rs        --> Cross-platform hook manager (~/.claude/settings.json)
```

### 4.2 Linux IPC Relay (`coucou-hook`)
The Unix domain socket server in Rust listens on `$XDG_RUNTIME_DIR/coucou.sock` (falling back to `~/.config/coucou/coucou.sock` with `0700` permissions):

```rust
#[cfg(target_os = "linux")]
pub fn socket_path() -> std::path::PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("coucou.sock")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        std::path::PathBuf::from(home).join(".config").join("coucou").join("coucou.sock")
    }
}
```

Protocol exchange matches the Windows implementation:
- Non-blocking events: JSON over Unix domain socket.
- `PermissionRequest`: Socket connection held open until island emits `approval_ack` and `approval_decision` (`allow` / `deny`).

### 4.3 Key Storage via Secret Service API
In `Cargo.toml`:
```toml
[dependencies]
keyring = { version = "3", default-features = false, features = ["sync-secret-service", "windows-native", "apple-native"] }
```
This automatically targets **GNOME Keyring** and **KWallet** on Linux distributions through D-Bus SecretService protocol with zero OS-specific glue code required.

---

## 5. Roadmap & Verification Plan

1. **Step 1: Refactor IPC & Geometry Layer**: Implement conditional compilation (`#[cfg(target_os = "linux")]`) in Rust core.
2. **Step 2: Build `coucou-hook` Linux Binary**: Compile lightweight POSIX CLI tool for Claude Code event hooks.
3. **Step 3: Support XDG & Desktop Specifications**: Generate `.desktop` launcher files and mime handling.
4. **Step 4: Package for Ubuntu 24.04 - 26.04**: Provide AppImage and `.deb` package setup in Tauri configuration.

---
*Created for Coucou Project — Linux Port Blueprint.*
