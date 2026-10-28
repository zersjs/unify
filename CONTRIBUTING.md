# Contributing to Unify

Thank you for considering a contribution. Unify is early-stage software that handles keyboard input and clipboard contents, so changes need to be reviewed carefully.

## Before you start

- For bugs and small fixes, open a pull request directly.
- For new features, protocol changes, or changes to pairing, TLS, or input handling, open an issue first and describe the approach.
- Security issues must not be reported in public issues. See [SECURITY.md](SECURITY.md).

## Development setup

```bash
cargo test -p unify-proto -p unify-core -p unify-security -p unify-net -p unify-platform -p unify-runtime -p unify-cli
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

For the UI in `app/`:

```bash
npm ci
npm run typecheck
npm test
```

## Pull request rules

- Keep each pull request focused on one change.
- Add or update tests for behavior you change.
- Do not log clipboard contents, key codes, or secrets.
- Describe platform-specific changes and state which OS you tested on. Changes that were only compiled, not run on hardware, must say so.
- CI must pass before review.

Pull requests are reviewed by the code owners listed in [CODEOWNERS](.github/CODEOWNERS).

## License

By contributing, you agree that your contribution is licensed under the MIT License.
