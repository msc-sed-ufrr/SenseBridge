# Produto

O que o SenseBridge faz, para quem e as regras em vigor, cada uma com o seu motivo. Como ele é construído está em [architecture.md](architecture.md).

<!-- Sem tabela de status ("pronto", "80%", "não iniciado"): ela envelhece sem ninguém perceber. O que está construído está no código, nos testes e no CHANGELOG; o que falta está em "Perguntas em aberto". -->

## Propósito

Sistemas IoT embarcados que combinam um sensor local com uma fonte externa de dados precisam lidar com fontes pouco confiáveis: o sensor pode não responder ou devolver leituras inválidas, e a rede ou o serviço externo podem ficar indisponíveis. Sem comportamento definido, essas falhas espalham valores inválidos sem que ninguém perceba ou derrubam a aplicação inteira por causa de uma única fonte.

O **SenseBridge** é uma biblioteca de software embarcado em Rust (a crate `sense_bridge`) que integra temperatura e umidade de uma **fonte local**, o sensor DHT11, e de uma **fonte externa de contexto**, a API meteorológica Open-Meteo. A cada **ciclo de aquisição** ela valida as duas leituras e entrega à aplicação um registro com a origem e a validade de cada valor e com o **estado de operação** explícito. Ela continua funcionando de forma controlada quando uma fonte falha e volta sozinha ao normal quando as fontes se restabelecem. A biblioteca não depende de hardware; a implementação de referência roda em um ESP32.

O projeto nasceu na disciplina de Arquitetura e Organização de Computadores do PPCA/UFRR (semestre 2026.2), na trilha PTT4 — Biblioteca de software embarcado, a partir do [projeto de pesquisa](../memory/2026-09-17-projeto-de-pesquisa-ptt4.pdf) aprovado. A pergunta de pesquisa é: *como projetar e avaliar uma biblioteca embarcada em Rust para ESP32 que integre dados ambientais locais e externos, mantenha comportamento definido diante da indisponibilidade parcial das fontes e tenha custos de latência e memória mensuráveis?*

**Quem decide.** As regras e as decisões de projeto são decididas pelos dois autores em conjunto, Ezequias Carlos Ferreira da Silva e Marcelo Gomes Barbosa. Uma mudança no escopo do projeto de pesquisa passa também pelo orientador, Prof. Herbert Oliveira Rocha.

### Termos

- **Ciclo de aquisição:** execução completa que obtém o resultado das duas fontes, valida as leituras, determina o estado de operação e produz um registro integrado.
- **Fonte disponível:** fonte com leitura aprovada na validação no ciclo corrente, obtida no próprio ciclo ou, no caso da fonte externa, reutilizada dentro do intervalo de consulta.
- **Registro integrado** (`Record`): o resultado de um ciclo entregue à aplicação.
- **Estado de operação:** `Normal`, `DegradedLocal` (degradado por falha local), `DegradedExternal` (degradado por falha externa) ou `Error`.
- **Transição:** mudança do estado de operação entre dois ciclos consecutivos.
- **Recuperação:** transição de um estado degradado ou de erro para `Normal`.
- **Cenários C1–C6:** condições de disponibilidade das fontes usadas nos testes e nos experimentos: C1 operação normal, C2 falha local, C3 falha externa, C4 falha simultânea, C5 recuperação e C6 leitura local congelada.
- **Leitura congelada:** um DHT11 travado que repete um valor plausível. Ela passa pela validação de faixa.

## Para quem

| Quem | Precisa | Usa por meio de |
| --- | --- | --- |
| Desenvolvedor de firmware (usuário da biblioteca) | Obter, a cada ciclo, dados ambientais com origem, validade e estado explícitos, sem tratar à mão as falhas de cada fonte | API pública da crate, documentação no docs.rs, exemplos |
| Pesquisadores (os dois autores) | Executar os cenários de forma controlada e coletar corretude, latência, RAM e Flash | Firmware de referência, saída CSV, console de captura, suíte de testes |
| Avaliador (orientador e docente da disciplina) | Verificar se o comportamento observado corresponde ao especificado e reproduzir os resultados | Repositório, README de reprodução, dados brutos, relatório técnico, vídeo de demonstração |
| Comunidade embedded Rust | Reutilizar a biblioteca em outros projetos embarcados | Crate `sense_bridge` no crates.io e repositório público |

## Sucesso

Metas do projeto de pesquisa (questões Q1–Q3):

- **Q1.** Todo valor entregue à aplicação passou pela validação e tem origem identificada: nenhum dado inválido aparece como válido.
- **Q2.** O comportamento é definido e observável quando uma fonte, ou as duas, estão indisponíveis: operação degradada, erro explícito e recuperação automática.
- **Q3.** Há evidência experimental reproduzível, obtida no ESP32, sobre corretude dos registros, detecção e recuperação de falhas, latência e uso de RAM e Flash.

Critérios observáveis:

| Critério | Meta |
| --- | --- |
| Registros com valor inválido marcado como válido | 0 |
| Correspondência entre estado esperado e observado em C1–C5, em todas as repetições (o estado esperado considera o atraso de identificação de BR-07) | 100% |
| Ciclos sem registro produzido | 0 |
| Transições de estado sem evento emitido | 0 |
| Ciclos até a recuperação após o restabelecimento das duas fontes (C5) | 1, o primeiro ciclo com as duas válidas |
| Cenários C1–C5 cobertos por testes sem hardware | 5 de 5 |
| Cenários C1–C5 executados no ESP32 | 5 de 5 |
| Testes automatizados passando e avisos não justificados da análise estática | 100% e 0 |
| Latência por operação, RAM em execução e Flash | Medidas e relatadas, com média e dispersão; sem meta prévia |
| Reprodução | Uma pessoa de fora do projeto reproduz os testes e ao menos um cenário no ESP32 seguindo apenas o README |
| Estado reportado | Corresponde sempre à disponibilidade das fontes (BR-23), e falhas e recuperações são identificáveis pelos registros e eventos, sem inspecionar o código |
| Fontes independentes | A queda de uma fonte não interrompe a entrega dos dados da outra |
| Verificação sem hardware | O núcleo da biblioteca é verificável inteiramente no computador de desenvolvimento |
| Vídeo de demonstração | 3 a 5 min |

## Regras

<!-- IDs estáveis: cite-os no código (comentário `BR-01: motivo`) e nos testes. Não reutilize um ID removido. Os IDs D1–D14 são as decisões de projeto que o código já cita; cada uma aparece entre parênteses na regra que a define, ou em architecture.md e nas ADRs. -->

### Aquisição das fontes

| ID | Regra | Por quê |
| --- | --- | --- |
| BR-01 | Em cada ciclo, as duas fontes são tratadas de forma independente: a falha ou a lentidão de uma não impede a obtenção da outra. | Evitar que uma única fonte indisponível interrompa toda a aplicação e permitir "a continuidade controlada da operação quando possível" ([projeto de pesquisa](../memory/2026-09-17-projeto-de-pesquisa-ptt4.pdf), §2.1). |
| BR-02 | Toda tentativa de leitura termina com um resultado explícito: leitura obtida ou falha de tipo identificado. Falhas locais: `NoResponse`, `Checksum`. Falhas externas: `NoConnectivity`, `Timeout`, `ServiceError`, `MalformedResponse`. Leituras reprovadas na validação: `Invalid`, com o motivo. | O escopo mínimo exige "identificar falhas de leitura ou comunicação" (projeto de pesquisa, §6.1). |
| BR-03 | (D5c) A consulta externa tem tempo limite configurável, de 3 s por padrão. Ao excedê-lo, a fonte externa fica indisponível no ciclo. | O tempo limite "deve ser menor que o intervalo entre ciclos, para que uma consulta lenta não atrase o ciclo seguinte" (`firmware/esp32/cfg.toml.example`). O motivo do valor de 3 s: não informado. |
| BR-04 | (D5b) O ciclo local tem intervalo configurável, de 5 s por padrão, e nunca menor que 1000 ms. | O mínimo de 1000 ms é o intervalo de amostragem do DHT11 indicado no manual do sensor (`firmware/esp32/cfg.toml.example`). O motivo do valor de 5 s: não informado. |
| BR-05 | (D5a) A consulta à Open-Meteo tem intervalo configurável, independente do ciclo local, de 15 min por padrão, e respeita os limites de uso da API. | O risco previsto era o de consultas frequentes excederem os limites de uso da API ([plano da disciplina](../memory/2026-10-22-plano-da-disciplina.md)). O motivo do valor de 15 min: não informado. |
| BR-06 | Entre duas consultas bem-sucedidas, a última leitura externa válida é reutilizada, sem consultar a fonte, e o registro informa a idade dessa leitura. | Não informado. |
| BR-07 | Enquanto a leitura externa é reutilizada, uma falha da fonte só é identificada na consulta seguinte. | Consequência aceita de BR-06; a arquitetura foi mantida (D10, em BR-31, e [ADR-005](../memory/decisions/ADR-005.md)). |
| BR-08 | Uma consulta externa que falha descarta a leitura externa guardada, e a fonte só volta a ficar disponível depois de uma nova consulta bem-sucedida. | "Um valor anterior à falha apresentado como atual é exatamente o tipo de propagação silenciosa que o projeto existe para evitar" ([ADR-005](../memory/decisions/ADR-005.md)). |
| BR-09 | (D11) Enquanto indisponível, a fonte externa é consultada de novo a cada ciclo. | Não informado. |

### Validação dos dados

| ID | Regra | Por quê |
| --- | --- | --- |
| BR-10 | (D3) Uma leitura local fora de 0–50 °C ou de 20–90 %UR é inválida. | É a faixa de medição do manual do DHT11 (`sense-bridge/src/validation.rs`). |
| BR-11 | (D4) Uma leitura externa fora de −90 a 60 °C ou de 0–100 %UR é inválida. | São "limites físicos plausíveis para dados meteorológicos" (`sense-bridge/src/validation.rs`). A origem dos valores: não informado. |
| BR-12 | Valores não numéricos (NaN ou infinito) são inválidos em qualquer fonte. | Suposição, não confirmada: decorre da meta Q1, nenhum dado inválido apresentado como válido. |
| BR-13 | Uma resposta externa sem temperatura ou sem umidade é uma resposta malformada e deixa a fonte externa indisponível no ciclo. | Suposição, não confirmada: decorre da meta Q1. |
| BR-14 | Valores exatamente nos limites da faixa são válidos; valores imediatamente fora são inválidos. | Não informado. |
| BR-15 | Uma leitura inválida deixa a sua fonte indisponível no ciclo, e o motivo da invalidação (`OutOfRange`, `NotANumber`) fica registrado. | Meta Q1 e o escopo mínimo: "evitar a propagação silenciosa de valores inválidos" (projeto de pesquisa, §6.1). |
| BR-16 | A divergência entre DHT11 e Open-Meteo não altera a validade de nenhuma leitura nem o estado. A Open-Meteo é contexto, nunca referência de verdade para o DHT11, nem nas análises. | "A Open-Meteo será utilizada como fonte externa de contexto e não como padrão absoluto de verdade para as leituras do DHT11" (projeto de pesquisa, §6.3). |
| BR-17 | A validação é determinística: a mesma leitura recebe sempre a mesma classificação. | Não informado. |
| BR-18 | (D14) A validação não detecta leitura congelada. O `FreezeDetector`, opcional e desligado por padrão, marca como suspeita a leitura local válida repetida, idêntica, por N ciclos seguidos, sem alterar a sua validade nem o estado. Opcionalmente, o firmware reinicia o DHT11 pelo GPIO de alimentação, no máximo uma vez por limiar, e registra se a leitura mudou ou persistiu. | Com a resolução de 1 °C e 1 %UR do DHT11, repetições legítimas são comuns (até 64 ciclos de 5 s nas capturas), e a repetição não distingue um sensor travado de um ambiente estável. A lacuna foi apontada na validação do laboratório SENTINELA (UFRR/PPCA, 2026) ([ADR-010](../memory/decisions/ADR-010.md)). |

### Integração e registro

| ID | Regra | Por quê |
| --- | --- | --- |
| BR-19 | Cada ciclo produz exatamente um registro integrado, inclusive no estado `Error`. | Suposição, não confirmada: decorre da meta Q2, comportamento definido e observável quando as fontes falham. |
| BR-20 | Todo registro contém: identificador de ciclo que só cresce, instante relativo da aquisição, resultado de cada fonte (leitura válida ou tipo de falha), idade da leitura externa e, quando houver, a transição. O estado é derivado dos dois resultados. | O escopo mínimo pede "registrar a origem e o estado de validade dos dados" (projeto de pesquisa, §6.1). O motivo dos demais campos: não informado. |
| BR-21 | Todo valor identifica a sua origem, e os valores de cada fonte ficam separados: não há média nem substituição de uma fonte pela outra. | A aplicação deve tratar as duas fontes explicitamente, sem valor sem origem ([ADR-006](../memory/decisions/ADR-006.md)), e a fonte externa é só contexto (BR-16). |
| BR-22 | Um registro nunca apresenta como atual um valor de fonte indisponível, nem um valor obtido antes da última falha identificada dessa fonte. | O mesmo motivo de BR-08 ([ADR-005](../memory/decisions/ADR-005.md)). |

### Estados de operação

| ID | Regra | Por quê |
| --- | --- | --- |
| BR-23 | O estado de cada ciclo é função da disponibilidade das fontes naquele ciclo: as duas disponíveis, `Normal`; só a externa, `DegradedLocal`; só a local, `DegradedExternal`; nenhuma, `Error`. | A tabela de cenários do projeto de pesquisa (§6.3) define esse comportamento esperado para C1–C4. |
| BR-24 | (D1) A máquina tem quatro estados; a recuperação é o evento de transição para `Normal`, não um quinto estado. (D2) Não há histerese: a reação acontece no próprio ciclo. | Mantém o estado como função da disponibilidade do ciclo, verificável linha a linha, e sustenta "recuperação em 1 ciclo" em C5; histerese atrasaria a detecção que o experimento quer medir ([ADR-004](../memory/decisions/ADR-004.md)). |
| BR-25 | Qualquer estado pode seguir qualquer outro, inclusive `Normal` → `Error` em um ciclo. Toda transição gera um evento com o estado anterior, o novo estado e o identificador do ciclo. | Suposição, não confirmada: decorre de BR-23 e BR-24, já que o estado depende só do ciclo corrente. |
| BR-26 | O primeiro estado vem do primeiro ciclo concluído. Nenhum estado `Normal` é presumido antes dele, e o primeiro ciclo não tem transição. | Não informado. |
| BR-27 | O estado `Error` não interrompe a biblioteca: os ciclos seguintes continuam tentando as duas fontes. A recuperação é automática, sem reiniciar o dispositivo nem exigir ação da aplicação. | A meta Q2 pede "erro explícito e recuperação automática", e o cenário C5 do projeto de pesquisa, "retorno ao estado normal sem inconsistências" (§6.3). |

### Verificação e experimentos

| ID | Regra | Por quê |
| --- | --- | --- |
| BR-28 | A suíte de testes roda sem ESP32, sem DHT11 e sem rede, com fontes simuladas e respostas gravadas da Open-Meteo, e cobre as quatro combinações de disponibilidade, as transições, inclusive a recuperação, e os limites de validação. | Plano B do projeto de pesquisa para a instabilidade da rede e da Open-Meteo: "utilizar respostas simuladas ou gravadas para os testes automatizados" (§9). |
| BR-29 | (D7) Cada cenário é executado no ESP32 em 5 repetições de 10 ciclos, com ciclo de 5 s. | Não informado. |
| BR-30 | Em cada repetição de C5: iniciar com falha, mantê-la nos 3 primeiros ciclos, restabelecer as fontes e observar os 7 ciclos seguintes. O primeiro ciclo com as duas fontes válidas deve estar em `Normal` e marcado como recuperação. | Não informado. |
| BR-31 | (D10) Nos experimentos C3–C5, `external_refresh_ms = 5000`, para consultar a fonte externa aproximadamente a cada ciclo; nos demais vale o padrão de BR-05. | Observar falha e recuperação externas a cada ciclo, sem esperar a próxima consulta (`firmware/esp32/cfg.toml.example`), mantendo a arquitetura de BR-07. |
| BR-32 | As falhas são induzidas por procedimento documentado e reproduzível. C6 usa a injeção de falha no firmware (`freeze_test_from_cycle`), não a manipulação física. | O risco previsto era a indução manual pouco reproduzível ([plano da disciplina](../memory/2026-10-22-plano-da-disciplina.md)). |
| BR-33 | (D8) São medidas a latência das operações principais (leitura local, obtenção externa, validação e integração, e ciclo completo), a RAM em execução e a Flash do firmware final. O CSV registra `read_us`, `fetch_us` e `cycle_us`; como o firmware mede as fontes dentro do ciclo, a validação e a integração são `cycle_us − read_us − fetch_us` ([medição de 22/11/2026](../memory/2026-11-22-latencia-validacao-integracao.md)). RAM e Flash são relatadas para o firmware completo e pela diferença contra um firmware-base equivalente, sem a biblioteca. | O risco previsto era o custo da pilha de rede mascarar o custo próprio da biblioteca ([plano da disciplina](../memory/2026-10-22-plano-da-disciplina.md)). |
| BR-34 | Os resultados são guardados em CSV com, no mínimo: cenário, repetição, ciclo, estado esperado, estado observado, validade e tipo de falha por fonte e latências. A correspondência entre estado esperado e observado é calculada automaticamente a partir do CSV. | O projeto de pesquisa pede formato estruturado, "preferencialmente CSV, para posterior análise" (§6.3). |
| BR-35 | (D6) As coordenadas da consulta à Open-Meteo são configuráveis; os experimentos usam latitude `2.82` e longitude `-60.67` (Boa Vista/RR). | Não informado. |
| BR-36 | (D9) O código é licenciado sob `MIT OR Apache-2.0`. A titularidade e a licença foram confirmadas com o orientador e o núcleo de inovação da UFRR antes da publicação da 0.1.0. | A confirmação evitou conflito com a política de propriedade intelectual da UFRR e com o registro de software previsto ([plano da disciplina](../memory/2026-10-22-plano-da-disciplina.md)). O motivo da escolha da licença dupla: não informado. |
| BR-37 | Antes da coleta de cada cenário, o critério de descarte de uma repetição é escrito. Uma repetição descartada é refeita, e o arquivo dela é versionado junto das demais, com o motivo; uma execução interrompida durante a coleta também. O relatório informa o total de execuções e o número oficial. Em C5, a repetição é descartada quando a captura não começa com a falha induzida (BR-30). | Uma exclusão de C5 feita em outubro, sem critério prévio e sem arquivo, não pôde ser auditada pela revisão por pares (F2) ([ADR-011](../memory/decisions/ADR-011.md)). |

## Casos de uso

O ator dos casos UC-001 a UC-006 é a aplicação embarcada, que chama um ciclo e consome o registro.

| Caso | Condição | Critérios de aceite |
| --- | --- | --- |
| UC-001 — Operação normal (C1) | DHT11 respondendo; Wi-Fi e Open-Meteo acessíveis | Temperatura e umidade das duas fontes, cada valor com a sua origem; as duas válidas; estado `Normal`; idade da leitura externa presente; divergência entre as fontes não muda o estado |
| UC-002 — Degradado por falha local (C2) | Open-Meteo acessível; DHT11 sem resposta ou com leitura inválida | Estado `DegradedLocal`; nenhum valor local como válido, com o tipo da falha ou o motivo da invalidação; valores externos válidos; o ciclo seguinte volta a ler o DHT11 |
| UC-003 — Degradado por falha externa (C3) | DHT11 respondendo; Wi-Fi desligado ou API inacessível, lenta ou com resposta inválida | Estado `DegradedExternal`; registro produzido mesmo quando a consulta excede o tempo limite; tipo da falha externa identificado; valores locais válidos; nenhum valor externo anterior à falha no registro |
| UC-004 — Erro por falha simultânea (C4) | DHT11 e Open-Meteo indisponíveis | Estado `Error`; registro produzido sem nenhum valor válido; os dois tipos de falha identificados; sem pânico nem reinício, e os ciclos seguintes continuam |
| UC-005 — Recuperação (C5) | Estado degradado ou de erro há pelo menos um ciclo; as fontes são restabelecidas | `Normal` no primeiro ciclo com as duas fontes válidas; o evento traz o estado anterior, `Normal` e o ciclo, e é marcado como recuperação; os valores das fontes que estavam indisponíveis foram obtidos depois do restabelecimento; nenhuma ação da aplicação |
| UC-006 — Leitura inválida | Uma fonte devolve valor fora da faixa ou não numérico | O valor nunca aparece como válido; o motivo está identificado; o estado segue BR-23; limites aceitos e valores imediatamente fora rejeitados |
| UC-007 — Testes sem hardware | Desenvolvedor ou pesquisador com a toolchain Rust | A suíte roda sem ESP32, DHT11 ou rede e cobre o que pede BR-28; todos os testes passam; a análise estática termina sem avisos não justificados |
| UC-008 — Cenário experimental | Pesquisador com o ESP32, o DHT11, o firmware gravado, Wi-Fi e o computador de apoio | Cada linha do CSV tem o que pede BR-34; a correspondência entre esperado e observado sai automaticamente do CSV; o procedimento de cada cenário está documentado; os dados brutos ficam versionados |
| UC-009 — Reprodução pelo avaliador | Avaliador com o repositório clonado | O README lista hardware, ligação, versões das ferramentas e comandos de compilação, gravação e teste; os testes passam em ambiente limpo seguindo só o README; o repositório tem licença, dados brutos e procedimento de análise |

## Perguntas em aberto

| Desde | Pergunta ou pedido | Quem pediu |
| --- | --- | --- |
| 2026-11-22 | Qual é o motivo de: a) a faixa externa de −90 a 60 °C e 0–100 %UR (D4, BR-11); b) o intervalo externo de 15 min (D5a, BR-05); c) o ciclo de 5 s (D5b, BR-04); d) o tempo limite de 3 s (D5c, BR-03); e) as 5 repetições de 10 ciclos (D7, BR-29); f) a nova tentativa a cada ciclo (D11, BR-09); g) o ESP-IDF com `std` (D12, [architecture.md](architecture.md))? O projeto de pesquisa não traz esses motivos. | Adoção da documentação |
| 2026-11-22 | Qual é o motivo de BR-06, BR-14, BR-17, BR-26, BR-30, BR-35, dos campos do registro em BR-20, da escolha da licença dupla em BR-36 e da versão mínima Rust 1.85 ([architecture.md](architecture.md))? | Adoção da documentação |
| 2026-11-22 | Confirmar os motivos marcados como "suposição, não confirmada" em BR-12, BR-13, BR-19 e BR-25 e no invariante do caminho da consulta em [architecture.md](architecture.md). | Adoção da documentação |
| 2026-11-22 | A titularidade e a licença precisam ser confirmadas de novo antes de cada versão, ou a confirmação da 0.1.0 vale para as próximas? | Adoção da documentação |
| 2026-11-22 | A consulta à Open-Meteo deve passar a usar HTTPS? Hoje ela usa HTTP sem TLS, e o motivo não foi registrado. A mudança alteraria a latência de `fetch_us` e a Flash medidas em C1–C5 e no firmware-base (D8), então exigiria repetir essas medições. | Adoção da documentação |
| 2026-11-22 | Gravar o vídeo de demonstração de 3 a 5 min e anexá-lo à entrega de 26/11/2026, como pede o projeto de pesquisa (§10.1) e o achado F4 da [revisão por pares](../memory/2026-11-25-revisao-por-pares-s14.md). Os autores estão produzindo o vídeo. | Projeto de pesquisa, §10.1 |
| 2026-11-22 | Executar o C6 no ESP32 (5 repetições de 10 ciclos, capturas `c6-r1` a `c6-r5`) e calibrar o limiar de `local_frozen_after` com capturas longas e `--repeats`. O limiar sugerido, 360 ciclos (30 min), e o reinício por suspeita ainda não foram validados em hardware. É a próxima entrega depois de 26/11. | Autores (D14) |
| 2026-11-22 | Quando o C6 rodar no ESP32, registrar os resultados em um log datado em `docs/memory/`, sem editar o relatório entregue. | Adoção da documentação |
| 2026-11-22 | A crate não ter dependências ([architecture.md](architecture.md)) não é verificado por nenhum teste nem pela CI. Acrescentar uma verificação? | Adoção da documentação |
| 2026-11-22 | Depois da disciplina: avaliar o registro de software, a publicação do conjunto de dados experimentais e testes com outros sensores e plataformas sem alterar o núcleo da biblioteca. | Projeto de pesquisa, §10.2 |

## Fora do escopo

- Implementar o protocolo de baixo nível do DHT11 na biblioteca (projeto de pesquisa, §6.2). O firmware de referência lê o sensor com a crate `dht11-trace` (D13, [ADR-009](../memory/decisions/ADR-009.md)).
- Hardware dedicado ou nova arquitetura de processamento.
- Solução completa de monitoramento ou automação ambiental: painéis, alertas, atuadores, interface de usuário.
- Medição de consumo energético.
- Calibrar, corrigir ou fundir numericamente as leituras do DHT11 com as da Open-Meteo, ou usar a Open-Meteo como referência de verdade (BR-16).
- Metas prévias de desempenho: latência, RAM e Flash são medidas e caracterizadas, não otimizadas até um limite.
- Previsão meteorológica ou variáveis além de temperatura e umidade.
- Envio de registros a servidores ou à nuvem e armazenamento de longo prazo no dispositivo.
- Firmware e validação experimental para outras placas (por exemplo, Arduino Mega), outros sensores ou vários sensores ao mesmo tempo na entrega da disciplina: a biblioteca não depende de hardware, mas só o ESP32 com DHT11 é validado.
- Configuração de Wi-Fi em tempo de execução (provisionamento) e atualização remota de firmware (OTA).
