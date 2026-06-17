# BAM network history

`history.jsonl` is an **append-only, timestamped record of the Jito BAM network**,
captured automatically every 6 hours by
[`.github/workflows/snapshot.yml`](../.github/workflows/snapshot.yml) and
committed straight back to this repository.

Each line is one capture produced by `bam-net snapshot`, read from the public
BAM explorer API (`https://explorer.bam.dev/api/v1`):

```text
{"ts":"2026-06-17T12:00:00Z","stake":{…},"nodes":[…],"validators":[…]}
```

Because the file is **append-only and lives in git history**, the dataset is
independently verifiable: anyone can replay the commit log to see how BAM
stake share, node/validator counts, concentration, and validator↔node churn
moved over time — without trusting any single party to host it.

## Working with it

```bash
# Adoption time series from the committed log
bam-net history --cache data/history.jsonl

# Validator↔node churn between the two most recent captures
bam-net churn --cache data/history.jsonl

# Or process it with any JSONL tooling
jq -c '{ts, pct: .stake.bam_stake_percentage}' data/history.jsonl
```

The log only grows — captures are never edited or deleted, so the series stays
faithful to what the API reported at each point in time.
