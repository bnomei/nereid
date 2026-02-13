# Design — 21-mcp-persist-session-active

Implement persistence by following the existing “retry-safe” pattern used by other persistent tools:
- clone `Session`
- apply the mutation to the clone
- call `SessionFolder::save_session(&Session)` (fail with `INTERNAL_ERROR` on write error)
- commit the cloned session into server state on success

