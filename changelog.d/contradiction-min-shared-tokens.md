### Added
- `[contradiction] min_shared_tokens = N` (`analysant_logical::Config::min_shared_tokens`): a pair needs at least N shared words that are not generic (see `generic_df_percent`). Off by default, so the no-config baseline and the Gate A oracle are unchanged. Gate B round 2 option for pairs held by a single shared word (round 1: 4 of 7 contradiction false positives).
