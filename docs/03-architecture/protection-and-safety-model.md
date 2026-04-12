# Protection and Safety Model

## Core rule
The app must not imply that something is “backed up” unless it satisfies a user-visible rule.

## MVP model
Use a **rule-based protection model with clear language**.

## Example protection states
- Only Copy
- Multiple Copies
- Protected by Rule
- Not Protected by Rule
- Unknown

## Example rules
- protected if item exists on at least 2 distinct storage sources
- protected if one copy exists in a user-designated backup source
- protected if copy exists in a marked archive mirror location

## Safety posture
MVP is intentionally non-destructive.

### Allowed
- move
- merge
- quarantine
- restore from quarantine
- user-confirmed organization changes

### Not allowed in MVP
- hard delete
- silent auto-cleanup
- hidden destructive automation

## Quarantine design
Use **per-drive quarantine**.

Why:
- preserves drive locality
- avoids costly cross-drive moves for large files
- matches external-drive-first workflows better

UI requirement:
- the UI should present quarantine as one coherent feature even though storage is per-drive under the hood

## Recommendation language
The app should speak carefully.

Preferred style:
- “Likely best copy”
- “Protected by your rule: Two distinct sources”
- “This appears to be your only known copy”
- “This file has additional copies, but none currently match a protection rule”

Avoid:
- “safe” when the system only has partial evidence
- “backed up” unless rule criteria explicitly justify it

## Acceptance criteria
- user can define/edit protection rules
- protection states update when rules or source availability changes
- only-copy warnings are prominent
- quarantine is reversible
- all destructive-adjacent actions require explicit confirmation
