# AGENTS

## Project

- **What:** `sense_bridge`, a `no_std` Rust library that integrates a local sensor (DHT11) and an external weather source (Open-Meteo) with explicit validity, failure type and operating state on every cycle, plus its ESP32 reference firmware and a host console for the experiments.
- **Stack:** Rust 2024 (`rust-version = 1.85`) for the library and the console; Rust on ESP-IDF v5.5.5 with `esp-idf-svc` for the firmware (Xtensa ESP32, `esp` toolchain from espup).

| Action | Command |
| --- | --- |
| Setup | Install Rust with rustup; for the firmware, once per machine: `cargo install espup espflash ldproxy --locked` and `espup install --targets esp32` |
| Test | `cargo test` |
| Lint | `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` |
| Build | `cargo build`; firmware: `. ~/export-esp.sh`, then `cargo build --release` in `firmware/esp32/` |
| Evidence | `scripts/evidencias.sh` writes `docs/evidencias/verificacao.md` |

The firmware is outside the root workspace and outside CI: build it with the ESP toolchain inside `firmware/esp32/`, which needs a `cfg.toml` copied from `cfg.toml.example`.

## What to read

1. This file, always.
2. `docs/knowledge/product.md`, `docs/knowledge/architecture.md`, and the code the task touches, with its tests.
3. Before changing a rule: its reason and the ADRs that cite it (`grep -rl BR-xx docs/memory`).

Do not read all of `docs/` without a reason. `docs/memory/` is history: read a log only when the task is about how something got where it is. `docs/evidencias/` holds the official captures and verification records; `docs/00-Projeto de Pesquisa - PTT4.pdf` is the approved research project the scope comes from.

## Where to write

The reason lives next to where it will be read. What the code and the tests already say does not become a document.

`docs/knowledge/` is read by people and by agents, and is written in Portuguese, like the rest of the project's documentation and comments. Write for someone who joins the project today: full sentences, a domain term explained the first time it appears, a Mermaid diagram when a flow does not fit in a sentence, a link instead of a repetition.

| What | Where | Rule |
| --- | --- | --- |
| A rule or a command every session needs | This file | Rules and commands only. Its history goes to `docs/memory/`, its explanation next to the code it governs, and the steps of a check stay in the hook or script that runs them: name the command, do not copy the steps |
| Product rule, with its reason | `docs/knowledge/product.md` | Overwrite: the current state is what counts |
| Components, flow, stack and invariants, with the reason | `docs/knowledge/architecture.md` | Update in the same commit that changes the structure |
| The reason, next to the code or the configuration | A one-line comment where the rule is implemented (`BR-03: <reason>`, or the decision ID, as in `(BR-10, D3)`) | Changes in the same commit as what it explains |
| Acceptance criterion | A test that cites the rule or the scenario in its name or description | Written with the code, never after |
| Invariant a machine can check | A test or a CI step that fails, cited in `architecture.md` › Invariantes | Prefer it to prose: prose only when nothing can check it |
| What a module does in detail | The module's own documentation (`//!`, the crate README, the firmware README) | `architecture.md` keeps the map |
| Work left half done | An `#[ignore]` test saying what is missing, and a commit saying where it stopped | Goes away when the test passes |
| Unanswered question, out-of-scope request | `product.md` › Perguntas em aberto | Once answered, it becomes a rule and leaves that section |
| Decision with a discarded alternative, a reversal, or an author outside the code | `docs/memory/decisions/ADR-0NN.md` | Append only; in the old one, only the Status changes |
| Pitfall that fits in neither a test nor a comment | `docs/memory/lessons.md` | Append only |
| A measurement, an audit, a finished plan | `docs/memory/<YYYY-MM-DD>-<subject>.md`, dated | Append only; the current state goes to `docs/knowledge/` |
| Official experiment captures | `docs/evidencias/experimentos/`, named `c<scenario>-r<repetition>` | Working captures stay in `capturas/`, ignored by git |
| Change a user of the crate will notice | `sense-bridge/CHANGELOG.md` | Under `[Não lançado]` until the release |

ADR template: `docs/memory/decisions/_template.md`. ADR IDs are sequential (`ADR-001` to `ADR-010` exist; the next is `ADR-011`), and the code cites them by ID. The decisions D1–D14 and the use cases UC-001–UC-009 keep their IDs too; each is defined once, in `product.md`, `architecture.md` or an ADR. When the folder passes ~50 ADRs, split it into one folder per theme (`decisions/<theme>/`).

## Rules

1. **A recorded decision changes only with confirmation.** A request that undoes something recorded with a reason (a rule, an architecture choice, an accepted ADR): show the recorded reason and confirm with whoever decided before changing it. The rules are decided by the two authors together; a change to the scope of the research project also goes through the advisor (`product.md` › Propósito).
2. **Nothing pending stays only in the conversation.** An unanswered question or an out-of-scope request goes to `product.md` › Perguntas em aberto, not into the code. Work left half done becomes an `#[ignore]` test. Do this before you finish.
3. **No source, no claim.** A fact about the project cites a file or the code (link to the file, never to a `#heading-anchor`). If you did not find it, say you do not know.
4. **Report what you ran.** Say what you verified and what was left out, including that the firmware is not built by CI.
5. **A divergence stops the work.** When the request, the code and the docs disagree: point it out and ask.
6. **The tree is shared.** Do not use `stash`, `reset` or `checkout` on changes that are not yours.
7. **A leftover placeholder is unfinished work.** Fill it in or delete it.

## Verification

| When | Command |
| --- | --- |
| Iterating | `cargo test -p sense_bridge` or `cargo test -p sense_bridge_console`, for what you touched |
| Firmware change | `cargo build --release` in `firmware/esp32/`, with the ESP toolchain; CI does not build it |
| Before finishing | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `git grep -nE '\{\{[[:alnum:]]' -- ':!docs/memory/decisions/_template.md'` |

## Critical conventions

- Documentation and comments in Portuguese; code names in English.
- State and failure names from `as_str()` are a contract: the CSV, the console and the versioned captures depend on them. The CSV header lives in `firmware/esp32/src/csv_header.in`, shared by the firmware and the console.
- The crate keeps `[dependencies]` empty and does not use `std` or `alloc`.

## Git

- Prefer GitFlow as the branching model: `main` holds released code, `develop` is the integration branch, and all work happens on short-lived branches created from `develop` (`feature/*`, `release/*`, `hotfix/*` — the latter branching from `main`).
- Never commit directly to `main`; merge into it only through `release/*` or `hotfix/*`, and merge hotfixes back into `develop`.
- Keep the working tree clean and maintain a clear commit history.
- Commit changes after completing a coherent development cycle or logical unit of work.
- Keep commits focused: avoid mixing refactoring, formatting, dependency changes, and feature work when they are unrelated.
- After committing, verify that the working tree is in the expected state.
- Never blindly overwrite local changes.
- After synchronization, verify the resulting history and working tree.
