# F071 plan

Raise the workspace rusqlite requirement to 0.41 and update only its locked package and
libsqlite3-sys. Regenerate the Rust dependency inventory from Cargo metadata, copy actual
downloaded license texts and retain existing notices. Frontend lock dependency entries
remain byte-equivalent; refresh only their own version/lock digests.

Re-run the OSV registry-package audit, required workspace checks and final release-binary
black-box gates. Preserve all earlier evidence as historical; source fingerprints and
artifact hashes bind final acceptance separately. Do not open or modify operator databases.

The updated progress-handler API requires a mutable connection binding in the read-only
dashboard. Preserve the SQLite read-only flags and all existing query/time bounds.
