# F071: Bundled SQLite maintenance

The candidate dependency audit found RUSTSEC-2026-0459 in libsqlite3-sys 0.38.2,
reachable through rusqlite's bundled feature. A local crafted database/WAL pair can
expose stale process memory. Update to a corrected bundled library before delivery.

Acceptance: locked rusqlite 0.41.0 and libsqlite3-sys 0.39.1; unchanged manifest/state
schemas and frontend dependency graphs; complete notices; current audit without findings;
workspace tests and the independent publication/install gates pass. Existing state remains
in place. No production migration, history rewrite or publication is part of this change.

Primary references: https://api.osv.dev/v1/vulns/RUSTSEC-2026-0459 and
https://github.com/rusqlite/rusqlite/releases/tag/v0.41.0 .
