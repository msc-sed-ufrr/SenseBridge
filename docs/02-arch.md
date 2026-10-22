# Arquitetura e Princípios

> **Atualizado em:** 21/10/2026 · **Base:** código em `sense-bridge/`, `firmware/esp32/`, `tools/console/`, [PRD](01-prd.md) e documentos de `docs/`.

Para contexto de produto, personas, regras de negócio e casos de uso, ver [docs/01-prd.md](01-prd.md). Este documento cobre o desenho do sistema: componentes, fluxo de dados, módulos, stack, observabilidade, princípios e decisões de arquitetura.

---

## 1. Componentes

O repositório separa **a biblioteca** (independente de hardware), as **ferramentas de host** e o **firmware da placa**. O workspace da raiz contém a crate `sense_bridge` e ferramentas de desenvolvimento como `sense_bridge_console`; `firmware/esp32/` permanece separado da execução normal dos testes de host por exigir toolchain e alvo específicos do ESP32.

| Unidade | Runtime / alvo | Responsabilidade |
| --- | --- | --- |
| `sense-bridge/` (crate `sense_bridge`) | Rust 2024, sem dependências, host e alvo embarcado | Núcleo: modelo de leituras e falhas, validação, ciclo de aquisição, estados de operação e registro integrado |
| `sense_bridge::sim` | idem | Fontes simuladas com resultado ajustável entre ciclos; permite reproduzir falha e recuperação sem hardware |
| `sense-bridge/tests/scenarios.rs` | host | Cenários C1–C5, limites de validação, transições e intervalo de consulta externa |
| `sense-bridge/tests/open_meteo.rs` | host | Interpretação de respostas reais da Open-Meteo, gravadas em `tests/fixtures/open_meteo/` |
| `sense-bridge/examples/simulate.rs` | host | Percorre C1–C5 com fontes simuladas e imprime um registro por ciclo em CSV |
| `tools/console/` (crate `sense_bridge_console`) | host | Captura a saída serial do ESP32, acompanha os ciclos, grava evidências em TXT/CSV e confere as capturas com o estado esperado em cada cenário |
| `firmware/esp32/` (bin `sense_bridge_esp32`, `publish = false`) | Rust + Xtensa + ESP-IDF/`esp-idf-svc` | Caso de uso: driver do DHT11, Wi-Fi, cliente Open-Meteo com tempo limite, relógio monotônico, cadência dos ciclos, instrumentação e saída CSV |
| DHT11 | periférico | Fonte local de temperatura e umidade |
| Open-Meteo | serviço HTTP externo | Fonte externa de contexto (nunca referência de verdade para o DHT11) |

Cada placa futura entra como projeto irmão em `firmware/`, com a sua própria toolchain, consumindo a mesma crate.

> **Estado da implementação em 22/10/2026.** A crate declara `#![no_std]`, o README participa dos doctests e o núcleo pode ser verificado no host sem hardware. A suíte completa possui **62 testes aprovados**: 20 unitários, 7 da integração Open-Meteo, 12 de cenários, 21 do console e 2 doctests. O firmware ESP32 está funcional, conecta-se ao Wi-Fi, consulta a Open-Meteo, executa o `SenseBridge`, mede latência e memória e produz registros experimentais. A integração real do DHT11 está concluída e validada. Um shield ESP-01S DHT11 v1.0 usado posteriormente em outra montagem não respondeu à leitura, sendo tratado apenas como ocorrência específica de hardware e não como pendência arquitetural.

---

## 2. Fluxo de dados

### Topologia

```mermaid
flowchart TD
    subgraph board["ESP32 — firmware/esp32"]
        App["Aplicação: loop de ciclos<br/>relógio monotônico"]
        Dht["Timed&lt;Dht11&gt;<br/>impl LocalSource"]
        Meteo["Timed&lt;OpenMeteo&gt;<br/>impl ExternalSource"]
        Bridge["sense_bridge::SenseBridge"]
        Parse["sense_bridge::open_meteo<br/>caminho e interpretação"]
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
        CSV["Evidências experimentais<br/>capturas/*.csv e *.txt"]
    end

    App -->|"cycle(now_ms)"| Bridge
    Bridge -->|"Record"| App
    Bridge -->|"read()"| Dht
    Bridge -->|"fetch()"| Meteo
    Dht -->|"1-wire"| DHT11
    Meteo --> WIFI
    Meteo -->|"write_path() / interpret()"| Parse
    WIFI -->|"HTTP + tempo limite"| OM
    App -->|"serial"| CSV
    Sim --> Tests
    Tests -->|"cycle(now_ms)"| Bridge
```

A biblioteca é o único ponto que combina as duas fontes, e o ciclo conhece cada uma apenas pela sua trait. Driver, pilha de rede, relógio e serialização ficam do lado do firmware; `Timed` envolve cada fonte para medir a duração da consulta fora da biblioteca. Da Open-Meteo, a biblioteca oferece apenas a montagem da consulta e a interpretação da resposta, que a implementação da fonte externa usa (ADR-008).

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
| `validation` | `Range`, `Limits`, `validate_local`, `validate_external` | Faixas com limites inclusivos e rejeição de valores não numéricos. `Limits::DHT11` e `Limits::PLAUSIBLE` seguem as faixas fechadas em D3 e D4 (PRD) |
| `state` | `OperationState`, `Transition`, `determine_state` | Estado derivado da disponibilidade e detecção de transição e recuperação. Função pura, sem memória |
| `record` | `Record`, `ExternalSample` | Saída de um ciclo: origem, validade, motivo da falha, idade da leitura externa e transição |
| `sim` | `Simulated<T, F>`, `SimulatedLocal`, `SimulatedExternal` | Fontes controláveis entre ciclos, com contagem de consultas, para testes e exemplos no host |
| `open_meteo` | `HOST`, `write_path`, `interpret` | Caminho da consulta e interpretação da resposta da Open-Meteo, sem alocação nem E/S. Usado pela implementação de `ExternalSource`, não pelo ciclo (ADR-008) |

O estado retido em `SenseBridge` é deliberadamente mínimo: `next_cycle` (identificador sequencial), `state` (estado do ciclo anterior, para a transição) e `last_external` (última leitura externa válida e o instante em que foi obtida, para o reuso dentro do intervalo).

---

## 4. Observabilidade

A biblioteca não registra log, não aloca e não imprime: **o próprio `Record` é a telemetria**. Cada ciclo produz exatamente um registro, inclusive no estado `Error`, com identificador de ciclo, instante, resultado por fonte, idade da leitura externa e a transição — o suficiente para reconstruir o comportamento sem inspecionar o código.

Como `timestamp_ms` é fornecido pelo chamador e a biblioteca não lê relógio, toda medição de tempo acontece fora dela, sem contaminar o trecho medido.

| Sinal | Onde é coletado | Situação |
| --- | --- | --- |
| Registros e transições | `Record` devolvido por `cycle` | Implementado |
| Latência por operação e ciclo | Firmware, com `read_us`, `fetch_us` e `cycle_us` | Implementado |
| RAM em execução | Firmware: heap livre, menor heap livre e margem de stack | Implementado e medido |
| Flash | Tamanho do firmware final via `xtensa-esp32-elf-size`, comparado ao firmware-base sem a biblioteca (D8) | Firmware completo e firmware-base medidos ([avaliação experimental](03-avaliacao-experimental.md)) |
| Exportação | Serial processada por `sense_bridge_console`, com gravação em TXT/CSV | Implementado; o cenário e a repetição vêm do nome da captura, e `--analyze` confere o estado esperado (UC-008) |

Para não distorcer o que se quer medir, as durações de cada ciclo ficam em memória e a linha CSV só é emitida depois do ciclo, fora da janela medida, e o custo da própria instrumentação é quantificado à parte (risco previsto no PRD).

---

## 5. Stack

### Linguagem e ferramentas

| Camada | Linguagem | Versão |
| --- | --- | --- |
| Biblioteca | Rust (edition 2024) | `rust-version = 1.85`, verificada na CI; desenvolvida em 1.97.1 |
| Firmware ESP32 | Rust + ESP-IDF | Toolchain `esp` do espup, sem versão fixa (1.97.0.0 em 21/10/2026); ESP-IDF v5.5.5; alvo Xtensa ESP32 |

### `sense-bridge/` (crate `sense_bridge`)

- **Sem dependências.** `[dependencies]` está vazio e assim deve permanecer: cada crate a mais é código que precisa compilar para o alvo embarcado e entra na conta de Flash do experimento.
- **Traits genéricas em vez de `dyn`.** `SenseBridge<L, E>` é monomorfizado, sem vtable nem ponteiro gordo no caminho do ciclo.
- **Tipos `Copy`, sem alocação.** `Record`, leituras e falhas são valores pequenos copiáveis; nada é movido para heap nem exige `alloc`.
- **`f32` para temperatura e umidade.** Precisão suficiente para as grandezas envolvidas e mais barata que `f64` no ESP32; a finitude é checada na validação.
- **`Result` por fonte.** A indisponibilidade de uma fonte é representada no tipo, não por sentinela ou flag paralela.

### `firmware/esp32/`

O firmware de referência está implementado sobre ESP-IDF/`esp-idf-svc`, com suporte a `std`, Wi-Fi, TLS/HTTP e integração com a biblioteca por dependência de caminho.

Responsabilidades do firmware:

- configurar e executar o ESP32;
- ler o DHT11 com driver próprio, que identifica a etapa do protocolo em que a leitura falhou e, com `dht11_power_gpio`, reinicia o sensor travado;
- estabelecer a conexão Wi-Fi;
- consultar a Open-Meteo com tempo limite configurável;
- fornecer o relógio monotônico e a cadência dos ciclos;
- executar `SenseBridge::cycle`;
- coletar latência, heap e stack;
- emitir os registros pela serial para captura pelo `sense_bridge_console`.

A interpretação da resposta da Open-Meteo permanece testável no host por meio de respostas gravadas.

### Dados

Não há banco nem persistência de longo prazo no dispositivo. O estado vive em memória durante a execução (`Option<OperationState>` e `Option<(ExternalReading, u64)>`). As evidências experimentais são capturadas no computador de apoio pelo console, em formatos CSV e TXT; as capturas de trabalho ficam em `capturas/`, fora do git, e as oficiais em `docs/evidencias/`.

### Convenções

- A API pública é o que `lib.rs` reexporta; o resto é detalhe de implementação, mesmo em módulo público.
- Tempo em milissegundos (`u64`) de relógio monotônico, sempre fornecido pelo chamador; diferenças com `saturating_sub`.
- Faixas são intervalos fechados: o valor exatamente no limite é válido.
- Identificador de ciclo começa em 0 e só cresce.
- Estados e falhas são serializados pelos nomes de `as_str()`, que fazem parte da API; a saída de `Debug` não é contrato. O cabeçalho do CSV fica em `firmware/esp32/src/csv_header.in`, incluído pelo firmware e pelo console.
- Documentação e comentários em português; nomes de código em inglês.
- Testes unitários junto do módulo (`#[cfg(test)]`); cenários do protocolo em `tests/scenarios.rs`, nomeados pelo comportamento verificado.
- Clippy sem avisos, com avisos tratados como erro.
- Todo item público da crate é documentado (`#![warn(missing_docs)]`).

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
scripts/evidencias.sh                          # registra as verificações em docs/evidencias/
```

A CI (`.github/workflows/ci.yml`) roda formatação, clippy, testes, documentação e o pacote da crate a cada push e pull request, além de compilar a crate com Rust 1.85 e para um alvo sem `std`.

Os firmwares compilam com a toolchain da sua placa, dentro de `firmware/<placa>/`, e ficam fora da CI.

---

## 8. Decisões de arquitetura

### ADR-001 — Fontes como traits, núcleo sem hardware

**Status:** aceita  
**Contexto:** A biblioteca precisa rodar no ESP32 com DHT11 e Open-Meteo, mas o núcleo tem de ser verificável no computador de desenvolvimento, sem sensor e sem rede, e reutilizável em outras placas.  
**Decisão:** O núcleo define `LocalSource` e `ExternalSource` e não conhece nenhum periférico. Driver, pilha de rede e tempo limite ficam nas implementações, no firmware.  
**Justificativa:** Inverter a dependência move toda a parte não determinística para fora do código que se quer testar. Com genéricos em vez de `dyn`, o custo em tempo de execução é nulo.  
**Consequências:** A crate não compila sozinha nada que leia o mundo real — é preciso um firmware. Em troca, os cenários C1–C5 rodam no host com `sim`, e outra placa reaproveita o núcleo implementando duas traits.

### ADR-002 — Sem `std`, sem alocação e sem dependências

**Status:** aceita e implementada  
**Contexto:** O alvo tem RAM e Flash restritas, e o projeto mede o custo próprio da biblioteca. Cada dependência transitiva entra nessa conta e precisa compilar para o alvo.  
**Decisão:** A crate não usa `std`, não aloca e não declara dependências. Tudo que ela produz cabe em tipos `Copy` de tamanho conhecido.  
**Justificativa:** Sem alocador, não há falha de alocação nem fragmentação a tratar em tempo de execução, e a medição de Flash atribuída à biblioteca fica interpretável.  
**Consequências:** Nada de `String` ou `Vec` dentro da crate, e a serialização dos registros permanece fora do núcleo. A única formatação é a do caminho da consulta à Open-Meteo, escrito em um `fmt::Write` fornecido pelo chamador (ADR-008). `lib.rs` declara `#![no_std]` e o README é verificado por doctests, de modo que essa restrição deixou de ser apenas uma convenção documental.

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
**Consequências:** C3 entra em degradado já no ciclo da falha. Em compensação, uma queda ocorrida durante o período de reuso só é percebida na consulta seguinte — com o intervalo de 15 minutos fechado em D5a, esse atraso é significativo. A D10 manteve a arquitetura: nos experimentos C3–C5, `external_refresh_ms = 5000` faz a fonte externa ser consultada aproximadamente a cada ciclo.

### ADR-006 — Um `Result` por fonte no registro, sem fusão

**Status:** aceita  
**Contexto:** O registro poderia carregar valores fundidos (média, substituição) ou campos opcionais acompanhados de flags de validade.  
**Decisão:** `Record` guarda `Result<LocalReading, LocalFailure>` e `Result<ExternalSample, ExternalFailure>`, com taxonomias de falha distintas por fonte e nenhuma combinação numérica.  
**Justificativa:** O tipo torna impossível apresentar um valor sem origem ou sem validade, e impede que um estado inconsistente (valor presente com flag de inválido) seja representável. Taxonomias separadas evitam falhas de rede em leitura local.  
**Consequências:** A aplicação sempre trata as duas fontes explicitamente — não existe "o valor" do ciclo. O estado é derivado desses dois `Result` e nunca precisa ser armazenado no registro.

### ADR-007 — Firmware fora do workspace, um projeto por placa

**Status:** aceita  
**Contexto:** Cada placa exige toolchain e alvo próprios. Se o firmware fizesse parte do workspace, `cargo test` na raiz tentaria compilá-lo para o host.  
**Decisão:** O workspace de host contém a biblioteca e ferramentas auxiliares, como `tools/console/`. O firmware permanece isolado em `firmware/`, com um projeto por placa, `publish = false` e dependência da crate por caminho.  
**Justificativa:** Mantém os comandos da raiz rodando sem hardware e o caso de uso fora do que vai para o crates.io.  
**Consequências:** O núcleo e as ferramentas de host são testados sem depender do ESP32. O firmware possui configuração e build próprios e deve ser compilado e gravado explicitamente com a toolchain ESP.

### ADR-008 — Interpretação da Open-Meteo dentro da biblioteca

**Status:** aceita  
**Contexto:** A resposta da Open-Meteo é a entrada com mais casos de borda do sistema: status de erro, corpo truncado, JSON malformado, campos ausentes ou não numéricos. O firmware só roda no ESP32, e o princípio de que o núcleo é verificável sem hardware deveria valer também para a parte determinística da fonte externa.  
**Decisão:** O módulo `open_meteo` monta o caminho da consulta (`write_path`) e converte status e corpo em `ExternalReading` ou `ExternalFailure` (`interpret`), com um leitor JSON próprio, sem alocação e com profundidade de aninhamento limitada. Rede, HTTP, tempo limite e conectividade ficam na implementação de `ExternalSource`, no firmware. O ciclo não depende do módulo.  
**Justificativa:** A interpretação é determinística e pode ser testada no host com respostas reais gravadas em `tests/fixtures/open_meteo/`. Uma crate de JSON violaria a ADR-002.  
**Consequências:** A biblioteca passa a conhecer um provedor específico, como exceção à regra de conhecer as fontes apenas pelas traits. Outro provedor entraria como módulo irmão, sem alterar o ciclo. A validação de faixa continua no ciclo, como para qualquer fonte externa.

### Decisões de projeto do PRD

Todas as [decisões de projeto (PRD, §12)](01-prd.md#12-decisões-de-projeto) estão fechadas: recuperação como evento de transição (D1), ausência de histerese (D2), faixas do DHT11 (D3) e da fonte externa (D4), intervalo externo de 15 min (D5a), ciclo local de 5 s (D5b), timeout HTTP de 3 s (D5c), coordenadas configuráveis (D6), 5 repetições de 10 ciclos por cenário (D7), referência dupla para RAM e Flash (D8), licença dupla (D9), arquitetura mantida diante do atraso de identificação durante a reutilização (D10), nova tentativa a cada ciclo (D11) e uso de ESP-IDF/`esp-idf-svc` no ESP32 (D12).
