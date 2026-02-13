# Design — 18-mcp-writeback

Approach:
- Extend `NereidMcp` to optionally carry a `store::SessionFolder` for persistence.
- In “persistent” mode, mutating tools apply changes to a cloned `Session`, attempt `SessionFolder::save_session(&Session)`, then commit the cloned session into server state on success.
- Keep the existing non-persistent constructor for tests/demos that don’t want filesystem writes.

Notes:
- Store writes are per-file atomic; failures can still leave partial disk updates. Tool calls should still fail, and the in-memory session should remain unchanged.

