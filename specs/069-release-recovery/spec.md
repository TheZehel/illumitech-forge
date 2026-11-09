# F069 — Recover partial release publication

Tracks #11. The 1.7.1 publisher creates a release and uploads every asset in one call.
A timeout/5xx can leave a remote draft, incomplete upload or completed publication without
a local completion receipt. A second explicit invocation must reconcile before writing.

Acceptance:
- Preserve current clean-source, evaluated revision, human gate, remote, visibility and
  tag checks. Dry-run remains local/read-only. Publication is never triggered by execution.
  Source identity comes from the resolved remote tag, peeling annotated tags. An existing
  release's `target_commitish` may be symbolic (for example `main`) and is not source evidence.
- Advertise additive `contracts.release_publication=1`. Recovery guidance requires this
  capability; older supported CLIs retain other routes but need read-only provider inspection
  and manual operator recovery instead of repeating the nonrecoverable publisher.
- Journal the publication contract (manifest/source fingerprint, repo/tag, notes and all
  asset hashes) outside source, before remote mutation. Changed inputs block recovery.
  Use a separate metadata namespace from draft-PR publication; keep state schema additive.
- Create a draft without assets, upload only missing expected assets, verify downloads,
  then publish. Uncertain mutation outcomes stop with retained intent; an explicit retry
  reads provider state before another write and never reruns implementation tasks.
- Reuse matching uploaded assets. Remove only an observed empty starter asset belonging
  to the expected candidate and a prior upload intent; never clobber valid/conflicting or
  unexpected remote assets. Unknown/mismatching tag, notes, visibility, inventory or bytes
  blocks recovery without overwriting remote changes.
- An already published release matching every approved input is a verified read-only
  reconciliation. Final local completion follows provider publication and download checks.
- All native subprocesses retain bounded output, timeout and cancellation; GitHub hosts
  remain pinned even with GH_HOST set. No account/model changes, credentials or API fallback.

Prove failures after create, upload and publish; starter recovery; repeated completion;
remote tampering and stale approval. Use deterministic disposable transports, not production.
