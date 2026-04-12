# Duplicate and Relationship Engine

## Purpose
This subsystem turns raw files into understandable relationships.

## MVP scope
MVP focuses on:
- exact duplicate detection
- likely best copy recommendation
- intentional backup distinction
- focused relationship maps

## Exact duplicate definition
Two file instances are exact duplicates when the system has sufficient evidence that their content is identical. Final exact grouping should rely on content hashing rather than only filename or size.

## Not exact duplicates
These should not be treated as exact duplicates:
- transcodes
- exports
- resized images
- same name different contents
- same project but different versions

## Recommendation model
The app should recommend a likely best copy, but never auto-resolve without user confirmation.

The recommendation UI must show:
- likely best copy
- why it was chosen
- what differs among members
- what action is available
- what the system is not sure about

## Initial ranking signals
Candidate signals for likely best copy:
- not quarantined
- lives in a preferred or protected source
- richer metadata completeness
- newer meaningful timestamp when relevant
- more user markings / collection membership
- better location semantics (for example not in temp/export/trash-like folders)

## Intentional backup handling
A user should be able to mark copies or folders as intentional mirrors/backups.
That state must affect:
- recommendation language
- cleanup suggestions
- protection evaluation

## Relationship maps in MVP
### Duplicate group map
Shows:
- selected group
- its file instances
- source locations
- preferred copy if any
- intentional backup status

### Item relationship map
Shows:
- asset/file instance
- duplicates
- derivative edges where known
- storage location links

### Project cluster view
Optional in MVP if schedule allows. Should remain focused, not global.

## Acceptance criteria
- exact duplicates can be grouped across sources
- recommendation is visible and explainable
- intentional backups can be distinguished from accidental duplicates
- group review supports user confirmation before action
- relationship map is readable and scoped

## Deferred work
- near-duplicate images
- semantic similarity
- automated project reconstruction
- transcoding lineage inference beyond basic rules
