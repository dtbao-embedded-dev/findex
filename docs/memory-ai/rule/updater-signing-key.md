---
title: Updater Signing Key
category: rule
purpose: Where the updater signing key lives and what must never happen to it.
status: active
updated: 2026-10-07
source: src-tauri/tauri.conf.json, src-tauri/tauri.release.conf.json, .github/workflows/release.yml
confidence: confirmed
verified_at: 2026-10-07
keywords: pubkey, TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PASSWORD, createUpdaterArtifacts, tauri signer
---

# Updater Signing Key

> Release builds are signed with one key pair generated on 2026-10-07; every installed
> copy trusts only its public key, so the private key must never be lost, rotated
> casually or committed.

## Where it lives

| Piece | Location |
|-------|----------|
| Public key | `plugins.updater.pubkey` in `tauri.conf.json` (public by design) |
| Private key | `~/.tauri/findex.key` on the maintainer's machine (outside the Drive-synced E: drive) |
| Password | `~/.tauri/findex.key.password`, randomly generated |
| CI copies | Repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |

## Rule

- Only the release workflow signs: it passes `tauri.release.conf.json`, which turns on
  `createUpdaterArtifacts`. A local `pnpm tauri build` needs no key.
- Never commit the private key or password, never copy them into this memory bank,
  and keep an offline backup (password manager). Losing either means installed copies
  can never auto-update again; leaking it means anyone can sign a fake update.
- Changing `pubkey` strands every installed copy on its current version; do it only
  after a leak, and tell users to reinstall manually.

## See also

- [App shell](../architecture/app-shell.md)
