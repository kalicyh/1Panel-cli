---
name: 1panel-rust-cli
description: Use when an AI agent should operate the Rust 1panel-cli for static-site, image, and compose deployment workflows with persistent config and safe non-interactive commands.
---

# 1Panel Rust CLI Skill

Use this skill when running `rust/1panel-cli` commands in CI or agent workflows.

## Preferred workflow

1. Set defaults once.
2. Verify config.
3. Discover targets (websites/compose).
4. Execute deploy/update command.

## Commands

Set defaults:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  set --base-url https://your.panel.domain --api-key <API_KEY> --insecure true
```

View config:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- config --json
```

Unset one key:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- config --unset base-url
```

List websites:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- list-websites --json
```

List compose files:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- list-composes --json
```

Update compose image and apply:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-compose-update \
  --compose-name wiki \
  --compose-path /opt/1panel/docker/compose/wiki/docker-compose.yml \
  --from-image gitea.nz.com/tigger/wiki:v1.0.1 \
  --to-image gitea.nz.com/tigger/wiki:v1.0.2 \
  --apply --json
```

Full image pipeline + compose deploy:

```bash
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-all-compose \
  --image-tag gitea.nz.com/tigger/wiki:v1.0.2 \
  --compose-name wiki \
  --compose-path /opt/1panel/docker/compose/wiki/docker-compose.yml \
  --from-image gitea.nz.com/tigger/wiki:v1.0.1 \
  --service wiki \
  --apply --json
```

## Rules

- Prefer `--json` for automation.
- Prefer `--base-url` over `--host` for HTTPS domains.
- Use `--insecure` only for self-signed or untrusted certificates.
- For compose updates, always provide `--service` or `--from-image`.
- Run `--dry-run` first on production targets.

## Config precedence

1. CLI args
2. env vars
3. `~/.1panel-cli/config.json`
