### Fixed
- `analysant judge`: rows are keyed `<project>:<id>`. A finding id is a content id, so the same boilerplate claim in two projects shared an id and `judge report` refused the sample as listing it twice (found on Gate B run 1: 1587 ids repeat across KISS, PersistAnt and Humboldt). **Breaking for verdict files**: `--id` now takes `<project>:<id>`.
### Added
- `analysant judge report --stopped-agree-percent P --stopped-min-judged N`: the second-judge agreement rule for a kind that stopped before 10 items could be selected (the figure holds when at least P% of the judged agree and at least N were judged), so it is not voided by the 8-of-10 threshold.
