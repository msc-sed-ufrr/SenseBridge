# SenseBridge

**SenseBridge** é uma biblioteca de software embarcado desenvolvida em **Rust** para integração resiliente de dados ambientais em sistemas baseados em **ESP32**.

O projeto é desenvolvido no contexto da disciplina de **Arquitetura e Organização de Computadores**, do Mestrado Profissional em Computação Aplicada da Universidade Federal de Roraima — PPCA/UFRR.

## Objetivo

Desenvolver e validar uma biblioteca embarcada capaz de integrar dados ambientais provenientes de uma fonte local e de uma fonte externa, mantendo comportamento definido diante de falhas de leitura ou indisponibilidade das fontes.

A implementação de referência utiliza:

- ESP32;
- sensor DHT11 para temperatura e umidade;
- API Open-Meteo como fonte externa de dados meteorológicos;
- linguagem Rust.

## Visão geral

A biblioteca será responsável por:

- obter dados ambientais locais;
- obter dados meteorológicos externos;
- validar os dados recebidos;
- identificar a origem dos dados;
- detectar falhas das fontes;
- impedir a propagação de dados inválidos;
- operar de forma degradada quando apenas uma fonte estiver disponível;
- identificar explicitamente o estado de erro quando ambas as fontes estiverem indisponíveis;
- recuperar a operação normal após o restabelecimento das fontes.

## Cenários experimentais

| Cenário | DHT11 | Open-Meteo | Comportamento esperado |
|---|---|---|---|
| C1 — Operação normal | Disponível | Disponível | Integração das fontes e geração de registro válido |
| C2 — Falha local | Indisponível | Disponível | Detecção da falha e operação degradada |
| C3 — Falha externa | Disponível | Indisponível | Detecção da falha e operação degradada |
| C4 — Falha simultânea | Indisponível | Indisponível | Estado de erro explicitamente identificado |
| C5 — Recuperação | Recuperado | Recuperado | Retorno à operação normal sem inconsistências |

## Métricas

Durante os experimentos serão avaliados:

- corretude dos registros;
- detecção de falhas;
- consistência dos dados;
- transições entre estados;
- recuperação da operação;
- latência;
- utilização de memória RAM;
- utilização de memória Flash.

O projeto não inclui medição de consumo energético.

## Arquitetura inicial

```text

                    INTERNET
                       │
                       ▼
              ┌─────────────────┐
              │   Open-Meteo    │
              │ Dados externos  │
              └────────┬────────┘
                       │ Wi-Fi
                       ▼
┌───────────┐      ┌──────────────────┐
│   DHT11   │─────▶│      ESP32       │
│Temp./Umid.│GPIO4 │                  │
└───────────┘      │ • coleta local   │
                   │ • consulta web   │
                   │ • trata falhas   │
                   │ • agenda ciclos  │
                   │ • mede recursos  │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │ SenseBridge      │
                   │ lógica/estado    │
                   │                  │
                   │ Normal           │
                   │ DegradedLocal    │
                   │ DegradedExternal │
                   │ Error            │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │ Console / Logs   │
                   │ ciclos           │
                   │ tempo            │
                   │ heap / pilha     │
                   │ medições         │
                   └──────────────────┘
             
```

## Tecnologias

- Rust
- ESP32
- DHT11
- Open-Meteo
- VS Code
- Git
- GitHub

## Estrutura

```text
SenseBridge/
├── Cargo.toml          # workspace: biblioteca e ferramentas de host
├── sense-bridge/       # biblioteca publicável no crates.io (no_std, independente de hardware)
│   ├── src/
│   ├── tests/          # cenários C1–C5 com fontes simuladas e respostas gravadas da Open-Meteo
│   ├── examples/
│   └── README.md       # documentação da crate
├── tools/
│   └── console/        # console de host: acompanha os ciclos pela serial e grava as capturas
├── firmware/           # casos de uso, um projeto por placa (não publicados)
│   └── esp32/          # DHT11 + Open-Meteo
├── capturas/           # capturas TXT/CSV gravadas pelo console (ignoradas pelo git)
├── docs/               # PRD, arquitetura e avaliação experimental
├── README.md           # este arquivo: o projeto de pesquisa
├── LICENSE-APACHE
└── LICENSE-MIT
```

A documentação de uso da biblioteca está em [`sense-bridge/README.md`](sense-bridge/README.md).

Os firmwares ficam fora do workspace porque cada um usa a toolchain da sua placa. Na raiz, os comandos abaixo rodam a biblioteca e o console no computador de desenvolvimento, sem hardware:

```sh
cargo test                                     # testes da biblioteca e do console, com os cenários C1–C5
cargo clippy --all-targets -- -D warnings      # análise estática
cargo run -p sense_bridge --example simulate   # imprime os cenários em CSV
```

Para compilar e gravar o firmware e acompanhar os ciclos com o console, ver [`firmware/esp32/README.md`](firmware/esp32/README.md).

## Autores

- Ezequias Carlos Ferreira da Silva
- Marcelo Gomes Barbosa

## Orientador

Prof. Herbert Oliveira Rocha

## Instituição

Universidade Federal de Roraima — UFRR  
Programa de Pós-Graduação em Computação Aplicada — PPCA  
Linha de Sistemas Embarcados e Distribuídos — SED

## Status

Projeto em desenvolvimento.

**Entrega prevista:** 26 de novembro de 2026.

## Licença

Este projeto é licenciado sob a [Licença Apache 2.0](LICENSE-APACHE) ou a [Licença MIT](LICENSE-MIT), à escolha de quem o utiliza.
