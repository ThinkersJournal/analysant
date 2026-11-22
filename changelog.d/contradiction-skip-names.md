### Added
- `[contradiction] skip_names = true` in `.analysant/project.toml` (`analysant_logical::Config::skip_name_claims`): Name-kind claims (a bare identifier, clause id or label) are not paired, because they assert nothing a denial could contradict. Off by default, so the no-config baseline and the Gate A oracle are unchanged. Gate B round 1 tuning option.
