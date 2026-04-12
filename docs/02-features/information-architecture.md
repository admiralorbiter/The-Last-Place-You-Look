# Information Architecture

## Top-level surfaces

### 1. Home
Purpose: orient the user and summarize archive state.

Contents:
- newly scanned sources
- duplicate summary
- protection summary
- risky items (only copy, unprotected)
- items needing review
- recent actions and quarantine activity

### 2. Library
Purpose: unified browse/search surface across all mounted local storage sources.

Capabilities:
- search
- filter by type, source, protection state, duplicate state, collection, date, size
- sort and grouping modes
- quick preview and selection

### 3. Item Detail
Purpose: explain one asset or one file instance in depth.

Contains:
- metadata
- preview
- relationships
- storage locations
- rule-derived protection state
- available actions

### 4. Duplicate Group Review
Purpose: safe comparison and decision-making.

Contains:
- grouped members
- likely best copy recommendation
- explanation of ranking
- per-member differences
- actions: keep as intentional backup, move, merge, quarantine

### 5. Maps
MVP includes focused maps, not an all-library graph.

Included:
- relationship map for an item or duplicate group
- storage map showing where copies live
- optional light project cluster view if ready

### 6. Collections
Purpose: virtual organization without immediate disk movement.

Includes:
- user collections
- tags
- saved groupings

### 7. Sources
Purpose: manage mounted storage sources.

Includes:
- source registration
- scan status
- mount status
- source-specific settings
- quarantine location visibility

### 8. Quarantine
Purpose: review non-destructively removed files.

Includes:
- per-drive quarantine inventory
- restore actions
- age and origin information

### 9. Settings
Includes:
- protection rules
- scan preferences
- UI density / disclosure preferences
- preview/cache settings

## Navigation philosophy
The app should feel simple at first glance and deeper as the user opens more context.

Guidelines:
- dashboard first, then drill into library
- overview first, zoom/filter, then details
- advanced actions appear where context is strongest
- maps are focused, not hairball visualizations

## View modes
Use one adaptive UI with progressive disclosure rather than separate simple and advanced apps.

Possible mechanisms:
- expandable side panels
- optional inspector pane
- advanced detail sections collapsed by default
- user-selectable density/layout preferences
