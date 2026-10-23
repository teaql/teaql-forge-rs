# Upstream and source revision

This Rust-only repository is exported from the `public/` tree of the private
`teaql-forge-commence` repository. Shared code is changed upstream first. Each
public sync is a new commit based on the current public `origin/main`; it does
not merge the private repository's Git history. The sync commit message records
the full commercial commit ID, the public base commit ID and the SHA-256 of the
exported file tree.

Generated text is compared against the original `teaql-code-gen` remote
`origin/main` at the fixed commit
`6f2d0a33699277b943148db948bdec2d5725df37`. Moving that baseline requires
a separate incremental comparison and an update to this document.

The commercial checkout uses `scripts/export-open-source.py` to prepare a
public candidate branch. The command does not push or merge the candidate.
