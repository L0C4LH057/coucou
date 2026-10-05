# Implementation Plan - GNOME Shell Top-Bar Anchored Sticky Notch & Global Hover

## Context & Problem
Based on the user's screenshot (`Screenshot From 2026-10-05 16-55-01.png`), the Coucou notch window is currently floating in the middle of the screen as a standard window instead of being docked at the top center of the GNOME top bar. Furthermore:
1. It is not staying pinned across all workspace desktops (`gw.stick()` needs to be reinforced on GTK map & configure events).
2. It does not collapse to the 6px top wake strip anchored at `y = 0` (or `y = top_bar_height`) that expands on hover across all Linux workspaces.

## Root Cause Analysis
1. **Compositor Anchoring**: Under GNOME (Mutter), standard GTK `set_position()` is ignored by Mutter for Wayland windows unless:
   - Window hint is `Dock` (`gw.set_type_hint(gtk::gdk::WindowTypeHint::Dock)`).
   - Window geometry is explicitly calculated relative to the primary monitor top edge ($y = 0$).
   - The window is connected to GTK's `map-event` and `configure-event` signals to re-apply `gw.stick()`, `set_keep_above(true)`, `set_skip_taskbar_hint(true)`, and `set_skip_pager_hint(true)` every time Mutter re-maps the window.
2. **Click-Through & Input Region for Hover Expansion**:
   - On Linux, `CURSOR_POLL` is `false`. Hover expansion depends on GTK Cairo shape combine region (`input_shape_combine_region`).
   - When collapsed, the input region must be a top-center 240px x 6px (or 16px) strip at $y=0$ anchored to the GNOME top panel.
   - When the user hovers over this strip, GTK mouse enter events / webview mouseover events trigger `set_collapsed(false)` to smoothly expand Mochi's notch down into the panel view.

## Execution Plan

### Step 1: Reinforce GTK Window Docking & Multi-Workspace Stickiness
Modify [`windows/src-tauri/src/platform/linux.rs`](file:///home/localhost/projects/opensource/coucou/windows/src-tauri/src/platform/linux.rs):
- In `make_non_activating(win)`:
  - Connect a signal handler for `map-event` and `realize` to force:
    ```rust
    gw.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
    gw.set_keep_above(true);
    gw.stick();
    gw.set_skip_taskbar_hint(true);
    gw.set_skip_pager_hint(true);
    gw.set_accept_focus(false);
    ```
  - Connect GTK `enter-notify-event` / `motion-notify-event` on the GTK window widget so hovering over the top wake strip automatically notifies the app to expand the notch when collapsed.

### Step 2: Ensure Top-Bar Anchoring in `apply_geometry`
Modify [`windows/src-tauri/src/island.rs`](file:///home/localhost/projects/opensource/coucou/windows/src-tauri/src/island.rs):
- In `apply_geometry(app, pref, collapsed)`:
  - Calculate `x = mp.x + (ms.width as i32 - pw as i32) / 2`.
  - Fix `y = mp.y` (flush against top screen edge $y = 0$).
  - For Linux GTK window, call `win.set_position(PhysicalPosition::new(x, y))` and re-assert `win.set_always_on_top(true)`.

### Step 3: Verify Input Region Shapes & Hover State Management
In `refresh_click_through(app, gate)` in [`windows/src-tauri/src/island.rs`](file:///home/localhost/projects/opensource/coucou/windows/src-tauri/src/island.rs):
- Verify that when `collapsed = true`, the input shape combine region creates a hit-box anchored at top-center ($x = \text{center} - 120$, $y = 0$, $w = 240$, $h = 16$).
- When the mouse enters this hit-box on GNOME, front-end mouse enter events seamlessly expand the notch down into the full panel.

### Step 4: Verification & Release Build
- Commit changes and push to GitHub.
- Track GitHub Actions build run to completion.
- Provide the user with the new `.deb` release package.

## Verification Checklist
- [ ] Notch stays at top center ($y = 0$) flush under the GNOME top bar.
- [ ] Notch is visible across ALL workspaces (stickiness active).
- [ ] Hovering over the top wake strip expands the notch into full view.
- [ ] Hovering away/leaving collapses the notch back into the top wake strip.
