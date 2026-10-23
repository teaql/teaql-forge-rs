# TeaQL Forge Rust agent instructions

## Space-first build artifact lifecycle

- Prefer disk space over avoiding recompilation. Apply this policy during build,
  generation and test work; read-only or documentation-only tasks do not authorize
  unrelated cleanup sweeps.
- Before substantial builds, check free space and the effective output/cache paths
  (including environment variables and script overrides). Reuse stable directories
  for compatible project/toolchain/profile/baseline combinations; do not accumulate
  timestamped or per-attempt copies of the same build or test executable.
- Give each one-off probe an explicitly owned temporary directory and register
  cleanup at creation with shell traps or context managers/try-finally. Preserve
  the command's exit status. Wait for, or stop and reap, only this task's children
  before deleting their outputs; never stop other sessions or their services.
- After consuming a successful one-off run, remove its disposable build outputs,
  copied binaries, dependency checkouts and virtual environments. Keep compact
  reports, commands, source revisions, hashes and test outcomes. On failure, retain
  the smallest useful diagnostic/reproducer, not the whole build tree by default.
  Keep full disposable artifacts only when explicitly requested for investigation
  or delivery, and report their path, size and reason for retention.
- Cleanup is limited to exact, verified paths owned by this task, known to be
  reproducible and no longer in use. Never infer safety from a directory name or
  from being ignored/untracked. Preserve source, uncommitted changes, generated
  source fixtures, expected results, retained evidence, databases and deliverables.
- Other Codex sessions may share build/download caches. Do not run broad cleanup,
  global `cargo clean`, or delete a shared target/cache while another task may use
  it. Shared cache eviction needs explicit coordination/locking; skip it if that
  cannot be established. Never run Docker prune as routine task cleanup.
- Remove temporary Git worktrees/clones only after checking tracked and untracked
  changes, nested repositories and that their commits are retained elsewhere; do
  not force removal or discard uncommitted work to save space.
- Share dependency downloads where test isolation permits. For throwaway Rust
  probes, prefer disabling incremental compilation and reducing debug information
  when compatible with the gate. Never weaken required tests, debug assertions,
  sanitizers, reference/candidate isolation or clean-room checks to save space.
- At milestones and task completion, inspect growth, remove eligible task-owned
  leftovers and summarize what was removed or retained (sizes when measured).
  Periodic cleanup is only a fallback for interrupted tasks, not a substitute for
  exit cleanup or a cache capacity limit. Report unsafe-to-clean growth rather than
  deleting another task's files.

## Forge-specific boundaries

- Check the effective Cargo target directory before building; shell/script
  overrides can bypass a shared configuration. Reuse compatible build caches,
  while keeping concurrent or incompatible configurations safely isolated. Do not
  allocate a fresh persistent target directory for every verification attempt.
- During iteration, target the affected workspace package/test when appropriate;
  still run every required acceptance gate before claiming completion. Space
  savings must not change generator semantics, test expectations or parity claims.
- Reuse a tested server/generator executable where isolation permits instead of
  copying a large executable into every run directory. Wait for the build to
  succeed before using it, and never replace/delete a binary still needed by a
  running task. Remove task-owned disposable binary copies when consumers exit.
- Generated model/source fixtures, templates, `tests/expected`, vendored source
  changes and uncommitted parity work are development inputs, not disposable
  build output. Keep compact oracle/comparison diagnostics and pinned source
  identities; clean only the reproducible scratch/build portion of a probe.
