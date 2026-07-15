# Contributing to teaql-forge-rs

First off, thank you for considering contributing to `teaql-forge-rs`. 

## Where do I go from here?

If you've noticed a bug or have a feature request, make sure to check our [Issues](https://github.com/teaql/teaql-forge-rs/issues) to see if someone else has already created one. If not, go ahead and create one!

## Fork & create a pull request

If you want to contribute code:

1. Fork the project.
2. Create your feature branch (`git checkout -b feature/my-new-feature`).
3. Commit your changes (`git commit -am 'Add some feature'`).
4. Push to the branch (`git push origin feature/my-new-feature`).
5. Create a new Pull Request.

## Requirements

Before submitting a Pull Request, please ensure you have run the following commands and that they succeed:
- `cargo fmt -- --check`
- `cargo clippy -- -D warnings`
- `cargo test`

We use Rust's standard tooling for formatting and linting.
