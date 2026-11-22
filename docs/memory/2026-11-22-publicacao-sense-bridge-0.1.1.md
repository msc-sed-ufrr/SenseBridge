# Publicação da `sense_bridge` 0.1.1 no crates.io — 22/11/2026

A crate `sense_bridge` foi publicada no crates.io na versão 0.1.1 em 22/11/2026, a partir de `main` na tag `sense_bridge-v0.1.1` (merge `ceddc98`, branch `release/sense_bridge-0.1.1`). Página: <https://crates.io/crates/sense_bridge>; documentação: <https://docs.rs/sense_bridge>. O passo a passo seguido está em [docs/knowledge/release.md](../knowledge/release.md).

## O que a versão traz

A 0.1.1 acrescenta o `FreezeDetector` e `FreezeDetector::reached_threshold` (BR-18, D14, [ADR-010](decisions/ADR-010.md)). A API é aditiva: o ciclo, o `Record`, o `Config` e a validação não mudam, e quem depende de `sense_bridge = "0.1"` recebe a versão sem quebra. As mudanças estão em `sense-bridge/CHANGELOG.md`.

O cenário C6, que exercita o detector no ESP32 por injeção de falha, está no firmware de referência e no console, que não são publicados. Na data da publicação, ele ainda não tinha sido executado em hardware, e o limiar sugerido de 360 ciclos não tinha sido validado.

## Verificações antes da publicação

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` | Sem avisos |
| `cargo test` | 83 testes aprovados |
| `cargo doc -p sense_bridge --no-deps` com `RUSTDOCFLAGS=-D warnings` | Sem avisos |
| `cargo build -p sense_bridge --target thumbv7em-none-eabihf` | Compila sem `std` |
| `cargo publish -p sense_bridge --dry-run` | Pacote de 25 arquivos montado e compilado |
| Rust 1.85 (`rust-version` da crate) | Não verificado localmente, por falta do toolchain; fica a cargo da CI (job `msrv`) |
| Firmware ESP32 | Não compilado; só o `Cargo.lock` mudou, com a nova versão da crate |

A confirmação da titularidade e da licença feita antes da 0.1.0 não foi repetida; se ela vale para as próximas versões segue em aberto (BR-36).
