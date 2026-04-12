# Glossary

## Asset
A logical thing the user cares about, independent of a single path. An asset may have one or more file instances and may participate in relationships such as duplicate, derivative, or project membership.

## File Instance
A physical file occurrence at a specific storage location and path. A copied file on another drive is a new file instance, even when it represents the same asset content.

## Relationship Edge
A typed connection between assets and/or file instances. Examples: exact duplicate, derivative, intentional backup, preferred copy, project membership.

## Storage Source
A mounted local volume or folder root registered for scanning. MVP focuses on mounted local storage sources, primarily removable external drives.

## Storage Location
A stable identity for where a file instance lives. On Windows this should be modeled independently from drive letters so removable media can be recognized across remounts.

## Duplicate Group
A set of file instances believed to represent the same underlying content, initially through exact-match methods.

## Intentional Backup
A copy the user wants to keep as part of a rule or archival strategy. It must be represented differently from an accidental duplicate.

## Canonical / Preferred Copy
The file instance the system recommends as the best working or reference copy based on visible criteria.

## Derivative
A related output created from another asset or file instance, such as an export or transcode. Derivatives are not the same thing as exact duplicates.

## Collection
A user-visible virtual grouping that does not require moving files on disk.

## Quarantine
A non-destructive holding area used instead of hard delete. In MVP, quarantine is per-drive.

## Protection Rule
A user-visible rule that determines whether an item is considered protected. Example: “protected if present on two distinct storage sources.”

## Protection State
The current status derived from visible rules, such as Only Copy, Multiple Copies, Protected by Rule, Not Protected by Rule, or Unknown.

## Staged Scan
A scan pipeline that inventories basic file information first and enriches content, hashes, previews, and relationships later.
