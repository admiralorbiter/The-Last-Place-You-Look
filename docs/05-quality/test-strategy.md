# Test Strategy

## Testing goal
This product must feel trustworthy before it feels magical.

Because it influences cleanup and organization decisions, testing must emphasize correctness, reversibility, and clarity of explanation.

## Test layers

### 1. Unit tests
Cover:
- duplicate grouping logic
- recommendation ranking logic
- protection rule evaluation
- path/state normalization
- relationship edge rules

### 2. Integration tests
Cover:
- scan pipeline against fixture directories
- SQLite persistence and migrations
- source mount/unmount behavior
- quarantine operations
- move/merge orchestration

### 3. Fixture-based archive tests
Use synthetic file trees to model:
- exact duplicates across sources
- intentional backup copies
- same name different content
- same content different name
- offline/remounted sources
- mixed media/doc archives
- quarantined files

### 4. UI and end-to-end tests
Cover:
- first scan flow
- duplicate review flow
- rule editing flow
- move/merge confirmation flow
- quarantine/restore flow
- map readability at common scopes

### 5. Performance tests
Measure:
- time to first visible inventory
- duplicate computation throughput
- search latency
- preview generation latency
- memory use during scan and hash workloads

## Critical safety tests
These must be high priority:
- no hard delete paths in MVP
- quarantine stays on expected drive
- protection state never overclaims backup certainty
- unplugging a source does not erase cataloged knowledge prematurely
- duplicate recommendation explanation matches underlying ranking state

## Release gates
Before any MVP release:
- migration tests pass
- scan/rescan tests pass
- duplicate fixtures produce expected groups
- protection rule evaluation matches expected outputs
- quarantine restore is verified
- manual regression pass covers organization actions

## Manual exploratory priorities
- weird path names
- large media files
- interrupted scans
- source reattachment with changed drive letter
- user confusion around duplicate vs intentional backup
