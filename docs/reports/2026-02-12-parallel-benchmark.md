# Parallel Benchmark (Parse Mode)

Date: 2026-02-12
Host: C:\Users\Cade\Projects\agentbrowser
Input: crates\core\tests\corpus\snapshots\wikipedia-rust.html
Command: benchmark\bench_parallel.ps1 -Mode parse -Iterations 3

Summary
- Single instance averages ~560ms wall, ~41.67MB peak working set.
- 5 instances averaged ~718ms wall, ~157.91MB total peak, ~31.58MB per instance.
- 10 instances averaged ~1157ms wall, ~334.27MB total peak, ~33.43MB per instance.

Notes
- Parse mode isolates core parsing/selection work and avoids network variance.
- Per-instance peak memory stays roughly flat across 5–10 instances.
- Wall time rises sublinearly with instances in this run (likely CPU contention).
