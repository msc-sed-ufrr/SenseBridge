# Arquitetura e Princípios

> **Atualizado em:** 23/09/2026 · **Base:** código em `sense-bridge/` e `firmware/esp32/`, [PRD](01-prd.md) e documentos de `docs/`.

Para contexto de produto, personas, regras de negócio e casos de uso, ver [docs/01-prd.md](01-prd.md). Este documento cobre o desenho do sistema: componentes, fluxo de dados, módulos, stack, observabilidade, princípios e decisões de arquitetura.

---

## 1. Componentes

O repositório separa **a biblioteca** (publicável, independente de hardware) do **caso de uso** (firmware de uma placa específica). O workspace da raiz contém apenas a biblioteca; `firmware/` fica em `exclude` para que `cargo test` na raiz rode inteiramente no computador de desenvolvimento.

| Unidade | Runtime / alvo | Responsabilidade |
| --- | --- | --- |
| `sense-bridge/` (crate `sense_bridge`) | Rust 2024, sem dependências, host e alvo embarcado | Núcleo: modelo de leituras e falhas, validação, ciclo de aquisição, estados de operação e registro integrado |
| `sense_bridge::sim` | idem | Fontes simuladas com resultado ajustável entre ciclos; permite reproduzir falha e recuperação sem hardware |
| `sense-bridge/tests/scenarios.rs` | host | Cenários C1–C5, limites de validação, transições e intervalo de consulta externa |
| `sense-bridge/examples/simulate.rs` | host | Percorre C1–C5 com fontes simuladas e imprime um registro por ciclo em CSV |
| `firmware/esp32/` (bin `sense_bridge_esp32`, `publish = false`) | Rust + toolchain do ESP32 (a definir) | Caso de uso: driver do DHT11, cliente Open-Meteo com tempo limite, relógio monotônico, cadência dos ciclos, instrumentação e saída CSV |
| DHT11 | periférico | Fonte local de temperatura e umidade |
| Open-Meteo | serviço HTTP externo | Fonte externa de contexto (nunca referência de verdade para o DHT11) |

Cada placa futura entra como projeto irmão em `firmware/`, com a sua própria toolchain, consumindo a mesma crate.

> **Lacunas entre documentação e código (23/09/2026).** O README da crate e o PRD descrevem `sense_bridge` como `no_std`; `lib.rs` ainda **não** declara `#![no_std]`. O código não usa `std` hoje, mas nada impede que passe a usar — a garantia não está sendo verificada pelo compilador. Pelo mesmo motivo o README da crate não roda como doctest (falta `#![doc = include_str!("../README.md")]`): `cargo test` executa **22 testes** (10 unitários + 12 de cenários) e nenhum doc-test. `firmware/esp32/src/main.rs` é um `fn main() {}` vazio: nada da coluna "firmware" desta arquitetura está exercitado. O estado por capacidade está na [tabela de escopo do PRD](01-prd.md#5-escopo).

---

## 2. Fluxo de dados

### Topologia

```mermaid
flowchart TD
    subgraph board["ESP32 — firmware/esp32 (a implementar)"]
        App["Aplicação: loop de ciclos<br/>relógio monotônico"]
        Dht["Dht11Source<br/>impl LocalSource"]
        Meteo["OpenMeteoSource<br/>impl ExternalSource"]
        Bridge["sense_bridge::SenseBridge"]
    end

    subgraph periph["Periféricos e rede"]
        DHT11["DHT11"]
        WIFI["Wi-Fi"]
    end

    subgraph ext["Serviço externo"]
        OM["API Open-Meteo"]
    end

    subgraph host["Computador de desenvolvimento"]
        Tests["cargo test<br/>cenários C1–C5"]
        Sim["sense_bridge::sim<br/>fontes simuladas"]
        CSV["CSV dos experimentos<br/>data/ (a criar)"]
    end

    App -->|"cycle(now_ms)"| Bridge
    Bridge -->|"Record"| App
    Bridge -->|"read()"| Dht
    Bridge -->|"fetch()"| Meteo
    Dht -->|"1-wire"| DHT11
    Meteo --> WIFI
    WIFI -->|"HTTP + tempo limite"| OM
    App -->|"serial"| CSV
    Sim --> Tests
    Tests -->|"cycle(now_ms)"| Bridge
```

A biblioteca é o único ponto que combina as duas fontes, e conhece cada uma apenas pela sua trait. Driver, pilha de rede, relógio e serialização ficam do lado do firmware.

### Ciclo de aquisição

```mermaid
flowchart TD
    Start(["cycle(now_ms)"]) --> LRead["LocalSource::read()"]
    LRead --> LVal{"validate_local<br/>faixa e finitude"}
    LVal -->|"aprovada"| LOK["local = Ok(LocalReading)"]
    LVal -->|"reprovada ou falha do sensor"| LERR["local = Err(LocalFailure)"]

    Start --> Age{"última leitura externa<br/>com idade &lt; external_refresh_ms?"}
    Age -->|"sim"| Reuse["external = Ok(ExternalSample { age_ms })<br/>a fonte não é consultada"]
    Age -->|"não"| Fetch["ExternalSource::fetch()"]
    Fetch --> EVal{"validate_external"}
    EVal -->|"aprovada"| ECache["guarda como última leitura<br/>external = Ok(age_ms: 0)"]
    EVal -->|"reprovada ou falha da consulta"| EErr["descarta a última leitura<br/>external = Err(ExternalFailure)"]

    LOK --> State
    LERR --> State
    Reuse --> State
    ECache --> State
    EErr --> State

    State["determine_state(local.is_ok(), external.is_ok())"] --> Diff{"estado difere do<br/>ciclo anterior?"}
    Diff -->|"sim"| Tr["transition = Some(Transition { from, to })"]
    Diff -->|"não"| NoTr["transition = None"]
    Tr --> Rec
    NoTr --> Rec
    Rec["Record { cycle, timestamp_ms, local, external, transition }"]
```

### Caminho de um ciclo

1. A aplicação chama `cycle(now_ms)` com o instante de um relógio monotônico. A cadência e o intervalo mínimo de amostragem do DHT11 são responsabilidade dela.
2. `LocalSource::read()` devolve a leitura ou uma `LocalFailure` de tipo identificado (`NoResponse`, `Checksum`).
3. `validate_local` aplica `Limits` — faixa com limites inclusivos e rejeição de NaN/infinito — e converte a reprovação em `LocalFailure::Invalid(InvalidReason)`.
4. Se existe leitura externa guardada com idade menor que `external_refresh_ms`, ela é reutilizada e o registro informa `age_ms`; a fonte externa não é consultada nesse ciclo.
5. Caso contrário, `ExternalSource::fetch()` é chamado. O tempo limite é aplicado pela implementação, que devolve `ExternalFailure::Timeout` ao excedê-lo; a resposta passa por `validate_external`.
6. Qualquer falha na consulta ou na validação externa **descarta** a leitura guardada, de modo que um valor obtido antes da falha nunca volta a aparecer como atual.
7. `determine_state` deriva o estado da disponibilidade das duas fontes naquele ciclo (`Normal`, `DegradedLocal`, `DegradedExternal`, `Error`).
8. Se o estado difere do ciclo anterior, o registro carrega um `Transition`; `is_recovery()` identifica o retorno a `Normal`. No primeiro ciclo não há transição, porque nenhum estado é presumido antes dele.
9. O `Record` é devolvido com identificador sequencial de ciclo, instante, resultado de cada fonte e a transição. O firmware o serializa em CSV para os experimentos.

---

## 3. Módulos do domínio

| Módulo | Tipos principais | Papel |
| --- | --- | --- |
| `bridge` | `SenseBridge<L, E>`, `Config` | Orquestra o ciclo: aquisição, validação, reuso da leitura externa, estado, transição e montagem do registro. Única parte com memória entre ciclos |
| `source` | `LocalSource`, `ExternalSource` | Fronteira com hardware e rede. É por onde — e só por onde — a plataforma entra na biblioteca |
| `reading` | `LocalReading`, `ExternalReading`, `LocalFailure`, `ExternalFailure`, `InvalidReason` | Modelo de dados e taxonomia de falhas, separada por fonte: o tipo de falha local não é expressável na externa e vice-versa |
| `validation` | `Range`, `Limits`, `validate_local`, `validate_external` | Faixas com limites inclusivos e rejeição de valores não numéricos. `Limits::DHT11` e `Limits::PLAUSIBLE` são padrões provisórios (PRD, D3–D4) |
| `state` | `OperationState`, `Transition`, `determine_state` | Estado derivado da disponibilidade e detecção de transição e recuperação. Função pura, sem memória |
| `record` | `Record`, `ExternalSample` | Saída de um ciclo: origem, validade, motivo da falha, idade da leitura externa e transição |
| `sim` | `Simulated<T, F>`, `SimulatedLocal`, `SimulatedExternal` | Fontes controláveis entre ciclos, com contagem de consultas, para testes e exemplos no host |

O estado retido em `SenseBridge` é deliberadamente mínimo: `next_cycle` (identificador sequencial), `state` (estado do ciclo anterior, para a transição) e `last_external` (última leitura externa válida e o instante em que foi obtida, para o reuso dentro do intervalo).

---

## 4. Observabilidade

A biblioteca não registra log, não aloca e não imprime: **o próprio `Record` é a telemetria**. Cada ciclo produz exatamente um registro, inclusive no estado `Error`, com identificador de ciclo, instante, resultado por fonte, idade da leitura externa e a transição — o suficiente para reconstruir o comportamento sem inspecionar o código.

Como `timestamp_ms` é fornecido pelo chamador e a biblioteca não lê relógio, toda medição de tempo acontece fora dela, sem contaminar o trecho medido.

| Sinal | Onde é coletado | Situação |
| --- | --- | --- |
| Registros e transições | `Record` devolvido por `cycle` | Implementado |
| Latência por operação (leitura local, obtenção externa, validação/integração, ciclo completo) | Firmware, marcando instantes em volta de `read`, `fetch` e `cycle` | A implementar |
| RAM em execução | Firmware (marca d'água de pilha e uso de heap, se houver) | A implementar |
| Flash | Tamanho do firmware final, comparado a um firmware base sem a biblioteca (PRD, D8) | A implementar |
| Exportação | CSV por serial, uma linha por ciclo | Parcial: `examples/simulate.rs` imprime `scenario,cycle,timestamp_ms,state,local,external,transition`; faltam as colunas do protocolo (repetição, estado esperado, latências) |

Para não distorcer o que se quer medir, as amostras de latência são acumuladas em memória e exportadas fora da janela medida, e o custo da própria instrumentação é quantificado à parte (risco previsto no PRD).

---

## 5. Stack

### Linguagem e ferramentas

| Camada | Linguagem | Versão |
| --- | --- | --- |
| Biblioteca | Rust (edition 2024) | `rust-version = 1.85`; verificada em 1.97.1 |
| Firmware ESP32 | Rust | toolchain e alvo a definir |

### `sense-bridge/` (crate `sense_bridge`)

- **Sem dependências.** `[dependencies]` está vazio e assim deve permanecer: cada crate a mais é código que precisa compilar para o alvo embarcado e entra na conta de Flash do experimento.
- **Traits genéricas em vez de `dyn`.** `SenseBridge<L, E>` é monomorfizado, sem vtable nem ponteiro gordo no caminho do ciclo.
- **Tipos `Copy`, sem alocação.** `Record`, leituras e falhas são valores pequenos copiáveis; nada é movido para heap nem exige `alloc`.
- **`f32` para temperatura e umidade.** Precisão suficiente para as grandezas envolvidas e mais barata que `f64` no ESP32; a finitude é checada na validação.
- **`Result` por fonte.** A indisponibilidade de uma fonte é representada no tipo, não por sentinela ou flag paralela.

### `firmware/esp32/`

Apenas o esqueleto e a dependência de caminho para a crate. As escolhas pendentes com impacto arquitetural:

- pilha do ESP32 (PRD, D12) — `esp-hal` (`no_std`) ou `esp-idf-hal`/`esp-idf-svc` (com `std`), o que determina como entram Wi-Fi, TLS e HTTP;
- driver do DHT11 (biblioteca existente, conforme o escopo do projeto);
- cliente HTTP com tempo limite e interpretação da resposta da Open-Meteo, esta última planejada **dentro** da biblioteca para poder ser testada com respostas gravadas.

### Dados

Não há banco nem persistência no dispositivo. O estado vive em memória durante a execução (`Option<OperationState>` e `Option<(ExternalReading, u64)>`), e os dados brutos dos experimentos ficam versionados em CSV, em `data/` (a criar).

### Convenções

- A API pública é o que `lib.rs` reexporta; o resto é detalhe de implementação, mesmo em módulo público.
- Tempo em milissegundos (`u64`) de relógio monotônico, sempre fornecido pelo chamador; diferenças com `saturating_sub`.
- Faixas são intervalos fechados: o valor exatamente no limite é válido.
- Identificador de ciclo começa em 0 e só cresce.
- Documentação e comentários em português; nomes de código em inglês.
- Testes unitários junto do módulo (`#[cfg(test)]`); cenários do protocolo em `tests/scenarios.rs`, nomeados pelo comportamento verificado.
- Clippy sem avisos, com avisos tratados como erro.

---

## 6. Princípios de arquitetura

- **O hardware fica fora da biblioteca.** Toda E/S entra por `LocalSource` e `ExternalSource`. Nenhum módulo da crate pode passar a depender de HAL, pilha de rede, relógio, alocador ou `std`: o que a plataforma oferece entra por parâmetro ou por trait, ou não entra.
- **Falha é um valor, não um efeito colateral.** Cada fonte aparece no registro como `Result` com o tipo de falha identificado. A biblioteca não entra em pânico, não reinicia o dispositivo e não interrompe os ciclos seguintes — nem no estado `Error`.
- **Nada inválido atravessa a fronteira como válido.** A validação acontece antes de qualquer valor chegar ao `Record`; `Ok` existe apenas para leitura aprovada. Um caminho novo que monte um registro sem passar pela validação viola o critério de sucesso central do projeto.
- **As fontes não se misturam.** Não há média, substituição nem correção de uma fonte pela outra. A divergência entre DHT11 e Open-Meteo não altera a validade de nenhuma leitura nem o estado: a fonte externa é contexto, não referência de verdade.
- **O estado é derivado, não acumulado.** `determine_state` é função pura da disponibilidade no ciclo. A única memória entre ciclos é o estado anterior (para emitir a transição) e a última leitura externa válida (para o reuso) — qualquer outra memória deve ser justificada, porque cria caminhos que os testes de cenário não cobrem.
- **A falha invalida o que estava guardado.** Toda falha na fonte externa descarta a leitura em cache. Nenhum valor anterior à última falha identificada pode reaparecer como atual, mesmo que ainda esteja dentro do intervalo de consulta.
- **O chamador é dono do tempo.** A biblioteca não dorme, não agenda e não lê relógio. Cadência do ciclo, intervalo mínimo do sensor e tempo limite da consulta são do firmware, o que mantém a medição de latência fora do trecho medido.
- **O núcleo é verificável sem hardware.** Todo comportamento novo do núcleo precisa ser demonstrável no host com `sim` e respostas gravadas. O que só puder ser exercitado no ESP32 pertence ao firmware, não à crate.

---

## 7. Diretrizes de desenvolvimento

Convenções de Git e fluxo de contribuição estão em [`AGENTS.md`](../AGENTS.md) — GitFlow, commits focados e verificação da árvore de trabalho após cada operação. A estrutura do repositório e as instruções de execução estão no [README do projeto](../README.md); o uso da API, no [README da crate](../sense-bridge/README.md).

Na raiz, sem hardware:

```sh
cargo test                                     # testes unitários e cenários C1–C5
cargo clippy --all-targets -- -D warnings      # análise estática
cargo run -p sense_bridge --example simulate   # imprime os cenários em CSV
```

Os firmwares compilam com a toolchain da sua placa, dentro de `firmware/<placa>/`.

---

## 8. Decisões de arquitetura

### ADR-001 — Fontes como traits, núcleo sem hardware

**Status:** aceita  
**Contexto:** A biblioteca precisa rodar no ESP32 com DHT11 e Open-Meteo, mas o núcleo tem de ser verificável no computador de desenvolvimento, sem sensor e sem rede, e reutilizável em outras placas.  
**Decisão:** O núcleo define `LocalSource` e `ExternalSource` e não conhece nenhum periférico. Driver, pilha de rede e tempo limite ficam nas implementações, no firmware.  
**Justificativa:** Inverter a dependência move toda a parte não determinística para fora do código que se quer testar. Com genéricos em vez de `dyn`, o custo em tempo de execução é nulo.  
**Consequências:** A crate não compila sozinha nada que leia o mundo real — é preciso um firmware. Em troca, os cenários C1–C5 rodam no host com `sim`, e outra placa reaproveita o núcleo implementando duas traits.

### ADR-002 — Sem `std`, sem alocação e sem dependências

**Status:** aceita (implementação parcial)  
**Contexto:** O alvo tem RAM e Flash restritas, e o projeto mede o custo próprio da biblioteca. Cada dependência transitiva entra nessa conta e precisa compilar para o alvo.  
**Decisão:** A crate não usa `std`, não aloca e não declara dependências. Tudo que ela produz cabe em tipos `Copy` de tamanho conhecido.  
**Justificativa:** Sem alocador, não há falha de alocação nem fragmentação a tratar em tempo de execução, e a medição de Flash atribuída à biblioteca fica interpretável.  
**Consequências:** Nada de `String`, `Vec` ou formatação dentro da crate — a serialização em CSV é do firmware e do exemplo. **Pendência:** `lib.rs` ainda não declara `#![no_std]`, então a restrição hoje é convenção, não garantia do compilador; declará-lo (e incluir o README como doctest) fecha a lacuna.

### ADR-003 — O instante do ciclo é injetado pelo chamador

**Status:** aceita  
**Contexto:** A biblioteca precisa datar registros, medir a idade da leitura externa e decidir sobre o intervalo de consulta, mas não deve depender de temporizador nem interferir na latência medida.  
**Decisão:** `cycle(now_ms)` recebe o instante de um relógio monotônico. A biblioteca não lê relógio, não dorme e não agenda.  
**Justificativa:** Tempo injetado torna o comportamento determinístico e testável (reuso da leitura externa e expiração são testados sem esperar). A instrumentação de latência fica no firmware, fora do trecho medido.  
**Consequências:** Respeitar o intervalo mínimo do DHT11 e a cadência do ciclo passa a ser obrigação do firmware. Diferenças usam `saturating_sub`, de modo que um relógio que retroceda não produz idade negativa — mas a biblioteca não detecta esse retrocesso.

### ADR-004 — Recuperação é evento de transição, e a reação é no próprio ciclo

**Status:** aceita (PRD, D1 e D2, fechadas)  
**Contexto:** Recuperação poderia ser um quinto estado da máquina, e falha/recuperação poderiam exigir N ciclos consecutivos (histerese) para serem declaradas.  
**Decisão:** Quatro estados apenas. A recuperação é o `Transition` cujo destino é `Normal` (`is_recovery()`), e a reação acontece no próprio ciclo, sem histerese.  
**Justificativa:** Manter o estado como função da disponibilidade do ciclo torna a tabela de estados verificável linha a linha e sustenta o critério "recuperação em 1 ciclo" de C5. Histerese acrescentaria memória e atrasaria a detecção que o experimento quer medir.  
**Consequências:** Falhas intermitentes do DHT11 produzem oscilação de estado entre ciclos; isso é observável no CSV e será relatado como taxa de falhas espontâneas. Introduzir histerese depois muda o resultado esperado de C2–C5.

### ADR-005 — Uma falha externa descarta a leitura reutilizada

**Status:** aceita  
**Contexto:** Entre duas consultas, a última leitura externa válida é reutilizada com a idade informada. Quando a consulta seguinte falha, seria possível continuar servindo esse valor até o fim do intervalo.  
**Decisão:** Toda falha de consulta ou de validação externa apaga a leitura guardada e a fonte fica indisponível até uma nova consulta bem-sucedida.  
**Justificativa:** Um valor anterior à falha apresentado como atual é exatamente o tipo de propagação silenciosa que o projeto existe para evitar.  
**Consequências:** C3 entra em degradado já no ciclo da falha. Em compensação, uma queda ocorrida durante o período de reuso só é percebida na consulta seguinte — com o intervalo de 15 minutos fechado em D5a, esse atraso é significativo, e como tratá-lo está em aberto (PRD, D10).

### ADR-006 — Um `Result` por fonte no registro, sem fusão

**Status:** aceita  
**Contexto:** O registro poderia carregar valores fundidos (média, substituição) ou campos opcionais acompanhados de flags de validade.  
**Decisão:** `Record` guarda `Result<LocalReading, LocalFailure>` e `Result<ExternalSample, ExternalFailure>`, com taxonomias de falha distintas por fonte e nenhuma combinação numérica.  
**Justificativa:** O tipo torna impossível apresentar um valor sem origem ou sem validade, e impede que um estado inconsistente (valor presente com flag de inválido) seja representável. Taxonomias separadas evitam falhas de rede em leitura local.  
**Consequências:** A aplicação sempre trata as duas fontes explicitamente — não existe "o valor" do ciclo. O estado é derivado desses dois `Result` e nunca precisa ser armazenado no registro.

### ADR-007 — Firmware fora do workspace, um projeto por placa

**Status:** aceita  
**Contexto:** Cada placa exige toolchain e alvo próprios. Se o firmware fizesse parte do workspace, `cargo test` na raiz tentaria compilá-lo para o host.  
**Decisão:** O workspace contém apenas `sense-bridge/`; `firmware/` está em `exclude`, com um projeto por placa, marcado `publish = false` e dependendo da crate por caminho.  
**Justificativa:** Mantém os comandos da raiz rodando sem hardware e o caso de uso fora do que vai para o crates.io.  
**Consequências:** Cada firmware tem seu próprio `Cargo.lock` e precisa ser compilado no seu diretório; a compilação para o alvo ESP32 não é exercitada pelos comandos da raiz e depende de verificação à parte.

### Decisões ainda em aberto

As pendências com impacto no comportamento estão em [Decisões de projeto (PRD, §12)](01-prd.md#12-decisões-de-projeto): faixas de validação (D3, D4), intervalo do ciclo local e tempo limite da consulta (D5b, D5c), coordenadas da consulta (D6), repetições por cenário (D7), tratamento do atraso de identificação (D10) e a escolha da pilha do ESP32 (D12), que condiciona Wi-Fi, TLS e cliente HTTP do firmware.

Já estão fechadas e sustentam os ADRs acima: recuperação como evento de transição (D1), ausência de histerese (D2), intervalo de 15 min entre consultas externas (D5a), referência dupla para RAM e Flash (D8), licença dupla (D9) e nova tentativa a cada ciclo (D11).
