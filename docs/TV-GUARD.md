# TV guard and boot to TV

The TV guard keeps the TV experience on top while TV mode is active, and "boot to TV" starts RusTV in TV mode when you sign in to Windows. Both are configured in Settings, page "TV guard", and stored in `%APPDATA%\RusTV\guard.json`.

None of this has been integration-tested on real hardware by the author of this document: the pure logic (overlap, allow lists, throttling, recovery, monitor selection) is unit-tested, the Win32 calls are thin wrappers that you should verify on your PC.

## Boot to TV

Setting "Start TV at Windows sign-in" has three values (same command as the "System" page, `set_autostart`):

| Value | Run entry (`HKCU\...\Run\RusTV`) | Behaviour |
| --- | --- | --- |
| No | removed | nothing starts |
| RusTV only | `"rustv.exe" --boot` | starts in the tray; resumes TV mode if TV mode was active when the PC was shut down, restarted or slept and the session was not ended from RusTV |
| TV mode | `"rustv.exe" --tv --boot` | always starts TV mode |

`--boot` means a quiet start: no pairing window, no first-run popup. It uses the screen remembered from the last TV session (name, resolution and position are stored in `guard.json`). If the TV is not awake yet, RusTV polls for it once per second for up to 60 seconds, then falls back to the saved screen index (or the last screen). `--screen N` overrides the remembered screen. Entries created by older versions (`--tv` without `--boot`) keep working as before.

If a screen is added, removed or changes resolution while in TV mode (TV waking up, resume from sleep), the TV windows are repositioned.

Keep awake (default on): while TV mode is active RusTV calls `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)` from a dedicated thread and releases it when TV mode ends or RusTV exits. Nothing network related (no Wake-on-LAN) is done.

## What the guard does

While TV mode is active and the guard is enabled (default), RusTV watches top-level windows with `SetWinEventHook` (foreground, show, location change, minimize end) plus a safety sweep every 500 ms.

- Only windows whose centre, or at least half of whose area, lies on the TV monitor are affected. Windows on your other monitors (your own tools, for instance) are never touched.
- An unauthorized window is minimized (default, `mode: "minimize"`) or moved to another monitor (`mode: "move"`, falls back to minimizing when there is no other monitor).
- When something steals the foreground, RusTV gives it back to the last allowed TV window, or the TV home. Every action is throttled: at most 4 actions per window per 5 s, then a pause of 3 s that doubles up to 60 s; foreground restoration is limited to 1 per 300 ms and 5 per 10 s, then a 5 s pause that doubles up to 60 s. The guard never fights a window in a tight loop.
- The window that has the foreground among the allowed TV windows is kept topmost; the previous one loses topmost.
- The taskbar of the TV monitor (`Shell_TrayWnd`, `Shell_SecondaryTrayWnd`) is hidden with `ShowWindow(SW_HIDE)` and restored on exit.
- Windows toast notifications are muted by writing `0` to `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings` value `NOC_GLOBAL_SETTING_TOASTS_ENABLED` (previous value saved, restored on exit). This is a per-user setting that Windows does not document as a stable API: some builds of Windows ignore it or only apply it after a delay, and some notifications (system, critical, other desktops) may still appear.

### Allowed windows

- RusTV and every process in its tree (mpv started by RusTV, the Brave/Vivaldi app window started by RusTV, WebView2).
- Programs you launched on purpose from a RusTV tile (native tiles, Steam, VLC...). After a launch, the guard adopts, for 60 s, a window whose process is newer than the launch or has the same executable name; for shortcuts, folders and UWP apps (the process is not known in advance) it adopts the first new window in the next 15 s. Adopted processes stay allowed until they exit. A window you bring forward with the remote's Alt+Tab is adopted too (4 s after the switcher was seen).
- The Windows task switcher (`MultitaskingViewFrame`, `XamlExplorerHostIslandWindow`, `ForegroundStaging`, `TaskSwitcherWnd`, `TaskSwitcherOverlayWnd`) and shell/system UI (taskbar, desktop `Progman`/`WorkerW`, menus, tooltips, IME and input panel classes, `Windows.UI.Core.CoreWindow`, TextInputHost, ShellExperienceHost...). These class names come from observation of Windows 10 and 11 and may differ on other builds: add yours with `allowClasses` (a trailing `*` is a prefix wildcard).
- Processes listed in `allowProcesses` (e.g. `"obs64.exe"`).

Command-line/profile-based detection of the browser is not used: the browser is recognized because it is a child of RusTV. If a browser hands off to an already running instance, its window is not a child and may be treated as an intruder.

### guard.json

```json
{
  "enabled": true, "mode": "minimize", "keepAwake": true, "hideTaskbar": true,
  "suppressToasts": true, "allowProcesses": [], "allowClasses": [], "bootToTv": "off",
  "recovery": { "taskbarHidden": false, "toasts": null, "session": { "tvActive": false } }
}
```

`bootToTv` is a read-only mirror of the Run entry. `recovery` holds the markers described below; do not edit it.

### Safety and recovery

Everything the guard changes is undone on TV mode exit, on app exit and on a Rust panic. Before hiding the taskbar or muting toasts, a marker is written to `guard.json`; if RusTV is killed or the PC loses power, the next start restores the taskbar (all taskbars are shown again) and the notification setting. The "panic" row in the settings page turns off the guard, taskbar hiding, toast muting and keep-awake and restores everything immediately. If you cannot reach the settings: end the `rustv.exe` process and start it again once, or delete `guard.json` after running `rustv.exe` once (Explorer restart also restores the taskbar).

## Limits (honest)

- The secure desktop (UAC prompts, Ctrl+Alt+Del, the lock screen, the sign-in screen) is a separate desktop that normal processes cannot see or change. The guard does not try to, and cannot prevent them from appearing.
- Windows Security / firewall consent dialogs, SmartScreen prompts and other elevated or protected windows may be above or unreachable by a normal process (UIPI): minimizing or moving them can fail silently.
- Exclusive fullscreen games and some DRM/overlay windows are not reliably handled; a game you launched from a tile is allowed.
- The Start menu, search and similar shell flyouts are intentionally left alone.
- A window with the "topmost" style from another program may still sit above RusTV briefly before the next event or sweep.
- This is a best-effort guard, not a security boundary. The user can always close or Alt+Tab.

## Recommended full kiosk for a dedicated PC (manual, not done by RusTV)

For an appliance PC, combine RusTV with Windows features. RusTV never applies these changes itself.

1. Create a dedicated standard Windows user (for example `tv`).
2. Auto sign-in for that user: `netplwiz` (untick "Users must enter a user name and password"), or Sysinternals Autologon (stores the password as an LSA secret instead of plain text in the registry). Autologon on a PC with sensitive data is a security risk; use a dedicated PC and account.
3. Start RusTV for that user with boot to TV set to "TV mode".
4. Optionally replace the shell so Explorer never appears:
   - Windows 11/10 Enterprise/Education: Shell Launcher v2 or Assigned Access (single app kiosk), configured by an administrator.
   - Documented Winlogon shell replacement for that user: `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Winlogon`, string value `Shell` = `"C:\path\to\rustv.exe" --tv --boot`. Without Explorer there is no taskbar, Start menu or Alt+Tab switcher, so test it before relying on it.
5. Disable sleep in Power options if you do not use RusTV's keep-awake.

To undo: sign in as another administrator, delete the `Shell` value (or Assigned Access / Shell Launcher configuration), run `netplwiz` and re-tick the password box (or run Autologon and click Disable), and remove the Run entry from the RusTV settings page. Safe mode and a second administrator account are your fallback if a kiosk shell does not start.
