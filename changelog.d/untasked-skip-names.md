### Added
- `[untasked] skip_names = true` in `.analysant/project.toml`: the untasked check sets aside Name-kind claims (bare identifiers, labels, crate names), which state no work. Off by default, so the no-config baseline and the Gate A oracle (runs v4, v5) are unchanged. Gate B round 1 tuning option (3 of 7 untasked false positives on the baseline were names).
