# ADR 0006: Zustand for frontend state management

## Status
Accepted

## Decision
Use **Zustand** for global React state management.

## Rationale
The UI has a meaningful amount of global shared state: scan progress, source list, library items, duplicate groups, protection states, and background job status. This state flows from Tauri backend events into the frontend and needs to be accessible across multiple components without prop drilling.

Zustand fits this use case well:
- Minimal boilerplate — stores are plain functions with `set`
- Works naturally alongside Tauri event listeners (subscribing in a store action is idiomatic)
- Good DevTools integration for debugging asynchronous scan state flows
- Small bundle size with no provider wrapping required

Redux Toolkit was considered but is better suited for larger teams and more complex async data flows than this product requires. React Context alone would be insufficient once multiple subsystems (sources, library, jobs, duplicates) share state.

## Consequences
- Local component state (UI toggles, form inputs, modal open/close) remains in React's own `useState` — Zustand is for cross-component shared state only
- Zustand stores should be organized by domain slice: `useSourceStore`, `useLibraryStore`, `useScanStore`, etc.
- Tauri event subscriptions should be set up in store actions (e.g., `initSourceStore` subscribes to `sources://status_updated`)
