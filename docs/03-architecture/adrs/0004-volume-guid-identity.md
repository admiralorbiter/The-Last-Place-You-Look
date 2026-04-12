# ADR 0004: Windows Volume GUID as stable storage source identity

## Status
Accepted

## Decision
Use the **Windows Volume GUID** (`\\?\Volume{uuid}\`) as the `stable_volume_identity` for every registered storage source. Drive letters are explicitly not used as identity keys.

## Rationale
Drive letters on Windows are not stable identifiers:

- Windows reassigns letters when a drive is plugged into a different port or after a reboot
- A drive that was `E:\` can become `F:\` with no user action
- Using the drive letter as identity would cause the app to lose track of the source entirely, destroying the catalog association

The Volume GUID survives:
- Drive letter reassignment
- Plugging into a different USB port
- System reboots
- Drive label renames

The GUID is not human-readable (`\\?\Volume{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}\`), so a separate user-provided `display_name` is stored for all UI contexts. The GUID is an internal identity key only.

## Implementation
Use `GetVolumeNameForVolumeMountPoint` (Win32 API, `Win32_Storage_FileSystem` feature in the `windows` crate) to resolve the GUID from any path on that volume.

To match a stored GUID to a currently-mounted drive at startup, enumerate all mounted volumes using `FindFirstVolume` / `FindNextVolume` and compare GUIDs.

## Consequences
- MVP is Windows-only, so no cross-platform volume identity abstraction is needed yet
- Future NAS or network source support will need a different identity strategy — this is deferred
- If a drive is reformatted, it gets a new Volume GUID and the app will treat it as an unrecognized source
