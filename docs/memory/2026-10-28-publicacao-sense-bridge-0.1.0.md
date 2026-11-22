# Publicação da `sense_bridge` 0.1.0 no crates.io

> **Situação em 03/11/2026:** a crate `sense_bridge` foi **publicada no crates.io na versão 0.1.0, em 28/10/2026**, após a confirmação da titularidade e da licença com o orientador e o núcleo de inovação da UFRR. Página: <https://crates.io/crates/sense_bridge>; documentação: <https://docs.rs/sense_bridge>. Donos da crate: `marcelobns` e `ezequiasferreira`. O passo a passo abaixo segue valendo para as próximas versões.

## Estado da preparação da 0.1.0

| Item | Situação |
| --- | --- |
| Nome | `sense_bridge`, registrado no crates.io na publicação da 0.1.0 (`sense-bridge` é o mesmo nome para o crates.io) |
| Metadados | `Cargo.toml` com descrição, licença `MIT OR Apache-2.0`, repositório, README, palavras-chave, categorias e `rust-version = "1.85"` |
| Documentação da API | Todos os itens públicos documentados; `#![warn(missing_docs)]` mantém a cobertura |
| README da crate | Também é a documentação da raiz no docs.rs; os links de licença e do CHANGELOG são absolutos para funcionar no crates.io e no docs.rs |
| Pacote | `cargo package -p sense_bridge` gera 23 arquivos (cerca de 19 KiB comprimidos) e compila a crate a partir deles |
| CI | Formatação, clippy, testes, documentação, pacote, Rust 1.85 e compilação para um alvo sem `std` (`.github/workflows/ci.yml`) |
| Histórico de versões | `sense-bridge/CHANGELOG.md` |

O passo a passo para as próximas versões está em [docs/knowledge/release.md](../knowledge/release.md).
