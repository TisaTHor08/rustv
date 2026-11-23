# Screen and sound (audio output, resolution, refresh rate, HDR)

The "Screen and sound" settings page lets you, with arrows and Enter only (phone remote):

- pick the Windows default audio output and change the volume of an output;
- change the resolution and refresh rate of the TV monitor, with a 15 second "keep or revert" safety;
- turn HDR ("advanced color") on or off for the TV monitor.

## Using it

Open Settings, then "Screen and sound".

- Audio output: left/right chooses an output (the current default is marked with `*`), Enter makes it the default for the console, multimedia and communications roles.
- Output volume: left/right changes it by 5 %.
- Resolution / Refresh rate: left/right cycles through the filtered list of modes; Enter (or the "Apply ..." row) applies. A first row "Keep this setting? (reverts in N s)" appears: Enter keeps the mode (it is then saved), left reverts immediately. If nothing is pressed for 15 seconds the previous mode is restored automatically by the Rust side (not by the page), so it also works if the page is closed or the screen is black.
- HDR: Enter toggles. Shown as "not supported" when Windows reports no advanced color support.

## How it works (crates/rustv-shell/src/display.rs)

- Monitor index: monitors are listed with `EnumDisplayDevices` / `EnumDisplaySettingsEx` and sorted by position (x then y), the same order as `tv::monitors`. The page uses `tvMonitor` (the TV monitor chosen in TV mode, or the last choice, or 0).
- Modes: interlaced modes, sizes below 800 x 600, rates below 24 Hz and depths below 24 bits (when a deeper one exists) are hidden; duplicates removed; sorted by size then refresh, largest first. Only modes in that list can be applied.
- Applying a mode: `ChangeDisplaySettingsEx` first with `CDS_TEST`, then as a temporary change (`CDS_FULLSCREEN`, nothing written to the registry). `display_confirm` re-applies it with `CDS_UPDATEREGISTRY` to make it permanent. `display_revert` or the 15 s timer restores the mode that was active before the first unconfirmed change (changes chained before confirming still return to the original confirmed mode).
- Audio: Core Audio (`IMMDeviceEnumerator`, `IAudioEndpointVolume`, friendly name from `PKEY_Device_FriendlyName`). Changing the default output uses the undocumented `IPolicyConfig` COM interface (CLSID `870af99c-171d-4f9e-af0d-e63df40c2bc9`, IID `f8679f50-850a-41cf-9c72-430f290290c8`), with its vtable declared by hand; `SetDefaultEndpoint` is slot 13. The older `PolicyConfigVista` interface (different IID) has no `ResetDeviceFormat` and `SetDefaultEndpoint` is slot 12; do not mix them up.
- HDR: `QueryDisplayConfig` finds the target whose source GDI name matches the monitor; `DisplayConfigGetDeviceInfo` (`GET_ADVANCED_COLOR_INFO`) reads supported/enabled and `DisplayConfigSetDeviceInfo` (`SET_ADVANCED_COLOR_STATE`) toggles.

## Limits

- "HDR supported" means Windows reports advanced color support; it is also true for some wide color gamut displays that are not real HDR displays.
- "Use HDR" can also depend on the Windows HDR setting (Settings, System, Display, HDR) and on the display being connected at a bandwidth that allows it. The brightness of SDR content while HDR is on is a separate Windows slider that this page does not control.
- The default output change relies on an undocumented interface that Microsoft may change; failure is reported as "Action impossible" and nothing else is touched.
- Display scaling (DPI percentage) is not shown.
- A temporary mode that was never confirmed survives an application crash until the next Windows sign-out or reboot (or until another mode change).
- Only the monitors that are active on the desktop are listed.

## Not verified

The COM and Win32 calls (audio enumeration, default device change, volume, mode change, revert timer thread, HDR read and toggle) compile but were never run, because they change the real hardware state. Only the pure logic (mode filtering, monitor choice, revert state machine, labels, helpers) has unit tests. Integration testing on a real TV is still to do, in particular `SetDefaultEndpoint` and the bit layout of the advanced color structures.

## Commands

`audio_outputs`, `audio_set_default`, `audio_set_output_volume`, `audio_cycle_output`, `display_info`, `display_set_mode`, `display_confirm`, `display_revert`, `display_set_hdr`.
