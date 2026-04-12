# Domain Model

## Core model choice
Use **Asset + File Instance + Relationship Edge**.

This is the correct model for the product because file-only modeling breaks down once the app needs to reason about duplicates, intentional backups, derivatives, and storage maps.

## Entities

### Asset
Represents the logical item of interest.

Suggested responsibilities:
- user-facing identity in many UI contexts
- grouping of related file instances
- association with collections/tags
- attachment point for some relationships

### File Instance
Represents a file at a specific source/path.

Suggested fields:
- file_instance_id
- asset_id
- storage_source_id
- stable_location_identity
- current_path
- file_name
- extension
- size_bytes
- modified_at
- created_at if available
- content_hash status/value
- preview status
- quarantine status

### Relationship Edge
Represents typed connections.

Initial relationship types:
- exact_duplicate
- intentional_backup
- preferred_copy
- derivative_of
- same_group_candidate
- project_member_of

### Storage Source
Represents a registered mounted local source.

Suggested fields:
- storage_source_id
- display_name
- source_kind (internal, removable)
- stable_volume_identity
- current_mount_path
- currently_mounted
- quarantine_root

### Protection Rule
Represents a user-visible rule for protection evaluation.

### Collection
Represents virtual grouping.

### Scan Job
Represents staged pipeline work.

## Identity rules
- moving or renaming the same file without copying should preserve continuity where possible
- copying to another drive creates a new file instance
- exact duplicates may map to the same asset group
- derivatives such as transcodes/exports are related but not exact duplicates

## Recommended modeling note
Do not collapse intentional backups into generic duplicates. Preserve that distinction in the relationship layer and UI.

## Example reasoning
A photo exists on Drive A and Drive B with identical content.
- one asset
- two file instances
- one exact_duplicate relationship group
- possibly one intentional_backup edge if user marks it that way

A transcoded video export exists beside raw footage.
- likely separate asset or derivative sub-asset depending on implementation detail
- derivative_of edge from export to source
- not exact_duplicate

## Open modeling questions for later refinement
- whether preferred copy is stored as an edge or computed projection
- whether project cluster becomes first-class in MVP or just a grouping relation
- whether asset identity is created only after duplicate detection or earlier
