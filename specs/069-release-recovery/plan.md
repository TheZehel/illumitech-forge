# F069 plan

Keep `release publish` and its options. Extract a small release-publication implementation
from publication.rs if that improves clarity. Reuse Store metadata/journal and process
supervision. Read typed GitHub release/asset state through the authenticated gh CLI, validate
all expected input identities and persist intent before create/upload/delete/publish.

Suggested transport: gh API reads for releases/assets; gh release create --draft,
release upload, release download into a fresh external temporary directory, and release
edit --draft=false. Treat only an observed typed HTTP 404 as absence, or use a complete
paginated collection; other errors are unknown. Do not infer absence from stderr text.
Download binary bytes through files, not the UTF-8 log string. Reject unexpected downloads.

Retain existing generic release tests by upgrading their fake provider to the new protocol;
do not weaken permissions or assertions. Add behavioral regressions around interruptions
and preservation. Run the independent operator fixture plus existing publication tests,
workspace gates and docs. Actual remote publication requires a later exact-candidate review.
