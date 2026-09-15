# SenseBridge

**SenseBridge** é uma biblioteca de software embarcado desenvolvida em **Rust** para integração resiliente de dados ambientais em sistemas baseados em **ESP32**.

O projeto é desenvolvido no contexto da disciplina de **Arquitetura e Organização de Computadores**, do Mestrado Profissional em Computação Aplicada da Universidade Federal de Roraima — PPCA/UFRR.

## Objetivo

Desenvolver e validar uma biblioteca embarcada capaz de integrar dados ambientais provenientes de uma fonte local e de uma fonte externa, mantendo comportamento definido diante de falhas de leitura ou indisponibilidade das fontes.

A implementação de referência utiliza:

* ESP32;
* sensor DHT11 para temperatura e umidade;
* API Open-Meteo como fonte externa de dados meteorológicos;
* linguagem Rust.

## Visão geral

A biblioteca será responsável por:

* obter dados ambientais locais;
* obter dados meteorológicos externos;
* validar os dados recebidos;
* identificar a origem dos dados;
* detectar falhas das fontes;
* impedir a propagação de dados inválidos;
* operar de forma degradada quando apenas uma fonte estiver disponível;
* identificar explicitamente o estado de erro quando ambas as fontes estiverem indisponíveis;
* recuperar a operação normal após o restabelecimento das fontes.

## Cenários experimentais

A avaliação do SenseBridge considera inicialmente cinco cenários:

| Cenário               | DHT11        | Open-Meteo   | Comportamento esperado                             |
| --------------------- | ------------ | ------------ | -------------------------------------------------- |
| C1 — Operação normal  | Disponível   | Disponível   | Integração das fontes e geração de registro válido |
| C2 — Falha local      | Indisponível | Disponível   | Detecção da falha e operação degradada             |
| C3 — Falha externa    | Disponível   | Indisponível | Detecção da falha e operação degradada             |
| C4 — Falha simultânea | Indisponível | Indisponível | Estado de erro explicitamente identificado         |
| C5 — Recuperação      | Recuperado   | Recuperado   | Retorno à operação normal sem inconsistências      |

## Métricas

Durante os experimentos serão avaliados:

* corretude dos registros;
* detecção de falhas;
* consistência dos dados;
* transições entre estados;
* recuperação da operação;
* latência;
* utilização de memória RAM;
* utilização de memória Flash.

O projeto não inclui medição de consumo energético.

## Arquitetura inicial

```text
                +------------------+
                |    Open-Meteo    |
                +--------+---------+
                         |
                       Wi-Fi
                         |
+---------+        +-----v------+
|  DHT11  +------->|    ESP32   |
+---------+        | SenseBridge|
                   +-----+------+
                         |
                    aplicação
```

## Tecnologias

* Rust
* ESP32
* DHT11
* Open-Meteo
* VS Code
* Git
* GitHub

## Estrutura prevista

```text
SenseBridge/
├── src/
│   ├── lib.rs
│   ├── sensor/
│   ├── weather/
│   ├── validation/
│   └── state/
├── examples/
├── tests/
├── data/
├── docs/
├── README.md
├── Cargo.toml
└── LICENSE
```

A estrutura poderá ser ajustada durante o desenvolvimento conforme as necessidades da implementação.

## Autores

* Ezequias Carlos Ferreira da Silva
* Marcelo Gomes Barbosa

## Orientador

Prof. Herbert Oliveira Rocha

## Instituição

Universidade Federal de Roraima — UFRR
Programa de Pós-Graduação em Computação Aplicada — PPCA
Linha de Sistemas Embarcados e Distribuídos — SED

## Status

Projeto em desenvolvimento.

**Entrega prevista:** 26 de novembro de 2026.
