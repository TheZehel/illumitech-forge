# Observable economy

Forge measures journaled calls, not the entire account. Cached input is included in input. Missing counters remain unknown. Tokens are not money or subscription balance.

## Compare deliveries

```sh
forge eval compare baseline.json candidate.json
```

Use complete schema-version-1 reports. The reader preserves effective outcome, coverage, attempts, duration, failures, cancellations and stale evidence. Tokens per accepted delivery are available only with current passing evidence and complete counters.

Attach evaluation_context with case, model, client_version, source_commit, verification_sha256 and configuration_sha256 to both reports, or wrap a report under the report key. These are operator declarations of observed experiment conditions; equal strings do not authenticate a model or account. Missing or incompatible conditions prevent a reduction figure.

## Bounded context

New workflows default to a 16,384-token heuristic budget with retrievable sources. Existing manifests preserve full context by default. Mandatory instructions never disappear to fit a budget. Native tools, background activity and account-wide consumption may be outside the measurement.

Six runs provide exploratory observations, not generalized savings or established quality equivalence.

Workflows link successful native planning receipts into their token journal. Failed/unlinked planning, hooks and other sessions remain outside that total, explicitly. Planning duration may remain unknown.
