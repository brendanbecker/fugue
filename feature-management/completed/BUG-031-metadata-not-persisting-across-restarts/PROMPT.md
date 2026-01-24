# BUG-031: Session Metadata Not Persisting Across Daemon Restarts

## Summary
Session metadata set via `fugue_set_metadata` is lost when the daemon restarts. This violates FEAT-050's acceptance criteria which states "Metadata persists across server restarts".

## Steps to Reproduce

1. Create a session: `fugue_create_session` with name "test-session"
2. Set metadata: `fugue_set_metadata(session="test-session", key="qa.tester", value="claude")`
3. Verify metadata: `fugue_get_metadata(session="test-session")` returns `{"qa.tester": "claude"}`
4. Restart the daemon (kill fugue-server or full fugue restart)
5. Check metadata: `fugue_get_metadata(session="test-session")` returns `{}`

## Expected Behavior
- Metadata should be saved to checkpoint files
- On daemon restart, metadata should be restored from checkpoints
- `fugue_get_metadata` should return the same values after restart

## Actual Behavior
- Metadata is stored in memory only
- Daemon restart clears all metadata
- Sessions survive but their metadata is empty

## Evidence from QA Demo

Before restart:
```json
{
  "name": "dev-qa",
  "metadata": {
    "qa.purpose": "full-feature-demo",
    "qa.tester": "claude"
  }
}
```

After restart:
```json
{
  "name": "dev-qa",
  "metadata": {}
}
```

## FEAT-050 Requirements (Not Met)

From FEAT-050 Section 3 - Persistence:
- [ ] Update checkpoint format to include metadata
- [ ] Update checkpoint restore to load metadata
- [ ] Add migration for existing checkpoints (empty metadata)

From FEAT-050 Acceptance Criteria:
- [ ] **Metadata persists across server restarts** ← FAILING

## Files to Investigate

| File | Investigation |
|------|---------------|
| `fugue-persistence/src/checkpoint.rs` | Check if metadata is included in checkpoint format |
| `fugue-session/src/session.rs` | Verify metadata field exists and is serializable |
| `fugue-server/src/persistence.rs` | Check checkpoint save/restore logic |

## Impact
- **Severity**: P2 Medium - Feature works but lacks durability
- **Use Case Impact**: Agents cannot rely on metadata for identity across restarts
- **Workaround**: Re-set metadata after each restart, or use environment variables instead
