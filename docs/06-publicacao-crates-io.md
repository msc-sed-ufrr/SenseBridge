# Publicação da crate no crates.io

> **Situação em 21/10/2026:** preparação concluída; a crate `sense_bridge` **não foi publicada**.

## Antes de publicar

- **Titularidade e licença.** O PRD prevê confirmar a titularidade e a licença com o orientador e o núcleo de inovação da UFRR antes de abrir o código (§8, riscos) e deixa a publicação da crate para depois da disciplina (§11). A publicação depende dessa confirmação.
- **A publicação é irreversível.** Uma versão publicada não pode ser apagada nem substituída. `cargo yank` apenas impede que projetos novos passem a depender dela; o código continua disponível para download.
- **Donos da crate.** Quem publica primeiro vira dono da crate no crates.io. Os demais autores entram com `cargo owner --add <usuário-do-github>`.

## Estado da preparação

| Item | Situação |
| --- | --- |
| Nome | `sense_bridge` livre no crates.io em 21/10/2026 (`sense-bridge` é o mesmo nome para o crates.io) |
| Metadados | `Cargo.toml` com descrição, licença `MIT OR Apache-2.0`, repositório, README, palavras-chave, categorias e `rust-version = "1.85"` |
| Documentação da API | Todos os itens públicos documentados; `#![warn(missing_docs)]` mantém a cobertura |
| README da crate | Também é a documentação da raiz no docs.rs; os links de licença e do CHANGELOG são absolutos para funcionar no crates.io e no docs.rs |
| Pacote | `cargo package -p sense_bridge` gera 23 arquivos (cerca de 19 KiB comprimidos) e compila a crate a partir deles |
| CI | Formatação, clippy, testes, documentação, pacote, Rust 1.85 e compilação para um alvo sem `std` (`.github/workflows/ci.yml`) |
| Histórico de versões | `sense-bridge/CHANGELOG.md` |

## Passo a passo

Segue o GitFlow do [`AGENTS.md`](../AGENTS.md): a versão sai de um branch `release/*`.

1. Criar o branch da versão: `git checkout -b release/sense_bridge-0.1.0 develop`.
2. Em `sense-bridge/CHANGELOG.md`, trocar `[Não publicado]` por `[0.1.0] - AAAA-MM-DD`. Conferir `version` em `sense-bridge/Cargo.toml`.
3. Conferir o pacote: `cargo package -p sense_bridge --list` e `cargo publish -p sense_bridge --dry-run`.
4. Fazer o merge do branch em `main` e em `develop` e criar a tag `sense_bridge-v0.1.0` em `main`.
5. Criar um token em <https://crates.io/settings/tokens>, entrar com `cargo login` e publicar a partir de `main`: `cargo publish -p sense_bridge`.
6. Conferir a página em <https://crates.io/crates/sense_bridge> e a documentação gerada em <https://docs.rs/sense_bridge>.
