# Publicação de uma versão

Como publicar uma versão nova da crate `sense_bridge` no crates.io. Os registros das publicações estão em `docs/memory/`: a [0.1.0](../memory/2026-10-28-publicacao-sense-bridge-0.1.0.md), em 28/10/2026, e a [0.1.1](../memory/2026-11-22-publicacao-sense-bridge-0.1.1.md), em 22/11/2026.

A crate é publicada a partir de `main`, por um branch `release/*`, como manda o GitFlow do [AGENTS.md](../../AGENTS.md). Os dois autores são donos da crate no crates.io (`marcelobns` e `ezequiasferreira`); outro autor entra com `cargo owner --add <usuário-do-github>`. Uma versão publicada não pode ser apagada nem substituída: `cargo yank` só impede que projetos novos passem a depender dela. Antes da 0.1.0, a titularidade e a licença foram confirmadas com o orientador e o núcleo de inovação da UFRR (BR-36); se isso vale para as próximas versões está em aberto em [product.md](product.md).

1. Criar o branch da versão a partir de `develop`: `git checkout -b release/sense_bridge-X.Y.Z develop`.
2. Em `sense-bridge/CHANGELOG.md`, trocar `[Não lançado]` por `[X.Y.Z] - AAAA-MM-DD` e conferir `version` em `sense-bridge/Cargo.toml`.
3. Conferir o pacote: `cargo package -p sense_bridge --list` e `cargo publish -p sense_bridge --dry-run`.
4. Fazer o merge do branch em `main` e em `develop` e criar a tag `sense_bridge-vX.Y.Z` em `main`.
5. Com um token de <https://crates.io/settings/tokens> e `cargo login`, publicar a partir de `main`: `cargo publish -p sense_bridge`.
6. Conferir <https://crates.io/crates/sense_bridge> e a documentação em <https://docs.rs/sense_bridge>.
