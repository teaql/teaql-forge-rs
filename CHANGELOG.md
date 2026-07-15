# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Advanced Rust Axum workspace capabilities (Dockerfile, GitHub Actions CI).
- Support for Ledger tracking with internal entity getters.
- Migrated Rust code generation engine to support TeaQL 4.1.0 data service traits.

### Changed
- Replaced `Repository` suffix with `DataService` across the codebase.
- Replaced `Registry` and `Behavior` trait interfaces for better naming semantics.
