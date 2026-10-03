# Documento de Requisitos de Produto — SenseBridge

> **Base:** Projeto de Pesquisa PTT4 — *Biblioteca Rust para Sistemas IoT Embarcados Context-Aware: Integração de Dados Ambientais e Tolerância a Falhas* (PPCA/UFRR, semestre 2026.2).
> **Atualizado em:** 23/09/2026 · **Entrega final:** 26/11/2026

---

## 1. Contexto

**SenseBridge** é uma biblioteca de software embarcado em Rust, independente de hardware e com implementação de referência no ESP32, que integra dados ambientais (temperatura e umidade) vindos de duas fontes: uma **fonte local**, o sensor DHT11, e uma **fonte externa de contexto**, a API meteorológica Open-Meteo. A cada ciclo, a biblioteca entrega à aplicação um registro com a origem e a validade de cada valor e com o estado de operação explícito (normal, degradado ou erro). Ela continua funcionando de forma controlada quando uma das fontes falha e retorna ao normal quando as fontes se restabelecem. O projeto segue a trilha **PTT4 — Biblioteca de software embarcado** da disciplina de Arquitetura e Organização de Computadores.

SenseBridge **não** é uma aplicação de monitoramento ou automação ambiental, **não** implementa o protocolo de baixo nível do DHT11, **não** usa a Open-Meteo como referência de verdade para calibrar o sensor e **não** mede consumo energético.

---

## 2. Personas

| Persona | Necessidade central | Interfaces |
| --- | --- | --- |
| Desenvolvedor de firmware (usuário da biblioteca) | Obter, a cada ciclo, dados ambientais com origem, validade e estado explícitos, sem tratar manualmente as falhas de cada fonte | API pública da biblioteca, documentação da API, exemplos |
| Pesquisadores (discentes autores) | Executar os cenários C1–C5 de forma controlada e coletar corretude, latência, RAM e Flash | Firmware de experimento, saída estruturada (CSV), suíte de testes |
| Avaliador (orientador e docente da disciplina) | Verificar se o comportamento observado corresponde ao especificado e reproduzir os resultados | Repositório, README de reprodutibilidade, dados brutos, relatório técnico, vídeo de demonstração |
| Comunidade embedded Rust (pós-disciplina) | Reutilizar a biblioteca em outros projetos embarcados | Repositório público e, futuramente, crate publicada |

---

## 3. Problema

Sistemas IoT embarcados que combinam leituras de sensores locais com informações externas precisam lidar com fontes heterogêneas e pouco confiáveis. O sensor pode não responder ou devolver leituras inválidas, a rede Wi-Fi ou a API externa podem ficar indisponíveis, e as duas fontes podem divergir. Sem um comportamento explicitamente definido, essas falhas espalham valores inválidos sem que ninguém perceba ou interrompem toda a aplicação por causa de uma única fonte indisponível. O tratamento costuma ser feito caso a caso dentro de cada aplicação, sem estados de operação bem definidos, sem registro da origem e da validade dos dados e sem medição do custo dessas proteções (latência, RAM e Flash) em uma plataforma de recursos restritos como o ESP32. O projeto responde à pergunta: *como projetar e avaliar uma biblioteca embarcada em Rust para ESP32 que integre dados ambientais locais e externos, mantenha comportamento definido diante da indisponibilidade parcial das fontes e tenha custos de latência e memória mensuráveis?*

---

## 4. Objetivos

### Metas

- **(Q1)** Garantir que todo valor entregue à aplicação tenha passado por validação e tenha origem identificada, de modo que nenhum dado inválido seja apresentado como válido.
- **(Q2)** Garantir comportamento definido e observável quando uma fonte, ou as duas, estiverem indisponíveis, com operação degradada, erro explícito e recuperação automática.
- **(Q3)** Produzir evidência experimental reproduzível, obtida no ESP32, sobre corretude dos registros, detecção e recuperação de falhas, latência das operações e uso de RAM e Flash.
- Organizar a biblioteca em responsabilidades separadas (aquisição, validação, integração e estados de operação), usando abstrações de hardware compatíveis com o ecossistema embedded Rust.
- Verificar o núcleo da biblioteca com testes automatizados e análise estática, sem depender de hardware nem de rede.
- Entregar um artefato reutilizável: API documentada, exemplos de uso, suíte de testes e repositório licenciado.

### Não-metas

- Não é uma solução completa de monitoramento ou automação ambiental (painéis, alertas, atuadores).
- Não implementa o protocolo de comunicação de baixo nível do DHT11; usa bibliotecas e abstrações existentes.
- Não usa a Open-Meteo para corrigir, calibrar ou substituir as leituras do DHT11. A fonte externa serve apenas de contexto.
- Não fixa metas de desempenho de antemão: latência, RAM e Flash são **medidas e caracterizadas**, não otimizadas até um limite.
- Não mede consumo energético.
- Não inclui firmware nem validação experimental para outras placas, outros sensores ou múltiplos sensores simultâneos na entrega da disciplina. A biblioteca não depende de hardware, mas apenas o ESP32 com DHT11 é validado.

---

## 5. Escopo

### Estado atual

| Capacidade                                                                              | Status | Notas                                                                                                                                                                                                            |
| --------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Separação entre biblioteca e caso de uso no repositório                                 | 🔶 70% | Workspace com a biblioteca em `sense-bridge/`, pronta para empacotamento no crates.io; `firmware/` fora do workspace, com um projeto por placa (`firmware/esp32/`, só esqueleto, marcado para não ser publicado) |
| Núcleo independente de hardware                                                         | 🔶 80% | Não usa a biblioteca padrão nem dependências, mas `lib.rs` ainda não declara `#![no_std]`, então a restrição não é verificada pelo compilador; compilação para o alvo ESP32 ainda não verificada                                                                                                            |
| Modelo das leituras e das falhas por fonte                                              | ✅ 100% | Tipos distintos para local e externa; falhas com tipo identificado (sem resposta, integridade, sem conectividade, tempo limite, erro do serviço, resposta malformada, leitura inválida)                          |
| Estados de operação                                                                     | ✅ 100% | Normal, Degradado — falha local, Degradado — falha externa e Erro                                                                                                                                                |
| Transições de estado e recuperação                                                      | ✅ 100% | Estado anterior guardado; transição emitida só quando o estado muda; recuperação identificada; sem transição no primeiro ciclo                                                                                   |
| Validação de dados                                                                      | 🔶 70% | Faixa (limites inclusivos) e valores não numéricos implementados; faixas provisórias (D3, D4); completude da resposta externa depende da interpretação da resposta                                               |
| Registro integrado (origem, validade, estado, ciclo)                                    | ✅ 100% | Um registro por ciclo com identificador, instante, leitura ou falha de cada fonte, idade da leitura externa e transição; estado derivado das próprias leituras                                                   |
| Intervalo de consulta externa e descarte após falha                                     | ✅ 100% | Leitura externa reutilizada dentro do intervalo; falha descarta a leitura anterior; intervalo padrão de 15 min fechado (D5a); ciclo local e tempo limite em aberto (D5b, D5c)                                                                                             |
| Fontes simuladas para testes sem hardware                                               | ✅ 100% | Resultado ajustável entre ciclos e contagem de consultas                                                                                                                                                         |
| Interpretação da resposta da Open-Meteo e respostas gravadas                            | 🔴 0%  | Planejada na biblioteca, para ser testada sem rede                                                                                                                                                               |
| Leitura do DHT11 no ESP32                                                               | 🔴 0%  | Firmware só com esqueleto; toolchain e alvo ESP32 ainda não configurados                                                                                                                                         |
| Consulta à Open-Meteo via Wi-Fi (com tempo limite)                                      | 🔴 0%  | Responsabilidade da fonte externa do firmware; a biblioteca já representa a falha por tempo limite                                                                                                               |
| Testes automatizados                                                                    | 🔶 60% | 22 testes passando sem hardware: 10 unitários e 12 de cenários (C1–C5, validação, transições, intervalo de consulta); os exemplos da documentação e do README não rodam como doc-test, porque `lib.rs` não inclui o README; faltam respostas gravadas e testes no ESP32     |
| Análise estática                                                                        | 🔶 50% | Clippy sem avisos com avisos tratados como erro; ainda não automatizado                                                                                                                                          |
| Instrumentação de latência                                                              | 🔴 0%  | A biblioteca recebe o instante do ciclo; a medição fica no firmware                                                                                                                                              |
| Medição de RAM e Flash                                                                  | 🔴 0%  | Inexistente                                                                                                                                                                                                      |
| Registro dos experimentos em CSV                                                        | 🔶 15% | Exemplo de simulação imprime um registro por ciclo em CSV, ainda sem as colunas do protocolo (repetição, estado esperado, latências)                                                                             |
| Documentação da API e exemplos                                                          | 🔶 50% | API pública documentada; README próprio da crate com exemplo de uso; exemplos da documentação e do README ainda não verificados automaticamente; exemplo `simulate` (C1–C5); falta exemplo de uso no firmware                 |
| Entregáveis-base (licença, README de reprodutibilidade, dados brutos, vídeo, relatório) | 🔶 25% | Licença dupla MIT/Apache-2.0; README do projeto descreve a estrutura e os comandos de teste; faltam instruções de hardware, dados, vídeo e relatório                                                             |

### Bloqueios para a entrega

| Bloqueio | Severidade | Área |
| --- | --- | --- |
| Pilha do ESP32 não escolhida (D12); firmware só com esqueleto, toolchain e alvo não configurados | Crítica | Plataforma |
| Nenhuma leitura real do DHT11 | Crítica | Aquisição local |
| Nenhuma integração com a Open-Meteo (consulta, tempo limite, interpretação da resposta) | Crítica | Aquisição externa |
| Instrumentação de latência, RAM e Flash inexistente | Alta | Experimentos |
| Saída CSV no formato do protocolo inexistente | Alta | Experimentos |
| Faixa do DHT11 não confirmada no manual e faixa externa sem fonte declarada (D3, D4); ciclo local e tempo limite sem valor (D5b, D5c) | Média | Validação |
| Respostas externas gravadas ausentes (Plano B contra instabilidade de rede) | Média | Testes |

---

## 6. Regras de Negócio

### Definições

- **Ciclo de aquisição:** execução completa que obtém o estado das duas fontes, valida as leituras, determina o estado de operação e produz um registro integrado.
- **Fonte disponível:** fonte com leitura aprovada na validação no ciclo corrente, obtida no próprio ciclo ou, no caso da fonte externa, reutilizada dentro do intervalo de consulta.
- **Registro integrado:** resultado de um ciclo entregue à aplicação.
- **Transição:** mudança do estado de operação entre dois ciclos consecutivos.
- **Recuperação:** transição de Degradado ou Erro para Normal.

### Aquisição das fontes

- Em cada ciclo, as duas fontes são tratadas de forma independente: a falha ou lentidão de uma não impede a obtenção da outra.
- Toda tentativa de leitura termina com um resultado explícito: leitura obtida ou falha com tipo identificado. Tipos locais: sem resposta, erro de integridade. Tipos externos: sem conectividade, tempo limite, resposta de erro do serviço, resposta malformada.
- A consulta externa tem tempo limite configurável. Ao excedê-lo, a fonte externa é considerada indisponível no ciclo.
- As leituras do DHT11 respeitam o intervalo mínimo de amostragem indicado no manual do sensor.
- A consulta à Open-Meteo tem intervalo configurável, independente do intervalo do ciclo local, e deve respeitar os limites de uso da API.
- Entre duas consultas externas bem-sucedidas, a última leitura externa válida pode ser reutilizada, e o registro informa a idade dessa leitura.
- Enquanto a leitura externa é reutilizada, a fonte não é consultada; uma falha ocorrida nesse período só é identificada na consulta seguinte (ver D10).
- Uma consulta externa que falha torna a fonte externa indisponível a partir daquele ciclo e descarta a leitura externa anterior. A fonte só volta a ficar disponível após uma nova consulta bem-sucedida.
- Enquanto indisponível, a fonte externa é consultada novamente a cada ciclo (D11).

### Validação de dados

- Leituras locais fora da faixa de medição do manual do DHT11 são inválidas (referência: 0–50 °C e 20–90 %UR; ver D3 em [Decisões de projeto](#12-decisões-de-projeto)).
- Valores não numéricos (NaN ou infinito) são inválidos em qualquer fonte.
- Leituras externas fora da faixa física plausível configurada são inválidas.
- Uma resposta externa incompleta (temperatura ou umidade ausente) é tratada como resposta malformada e torna a fonte externa indisponível no ciclo.
- Valores exatamente nos limites da faixa são válidos; valores imediatamente fora são inválidos.
- Uma leitura inválida torna sua fonte indisponível no ciclo, e o motivo da invalidação (fora de faixa, não numérico) fica registrado.
- Nenhum registro apresenta como válido um valor reprovado na validação.
- A divergência entre DHT11 e Open-Meteo não altera a validade de nenhuma das leituras nem o estado de operação.
- A validação é determinística: a mesma leitura produz sempre a mesma classificação.

### Integração e registro

- Cada ciclo produz exatamente um registro integrado, inclusive no estado Erro.
- Todo valor presente no registro identifica sua origem (local ou externa).
- Todo registro contém: identificador de ciclo que só cresce, instante relativo da aquisição, estado de operação, validade de cada fonte e, quando indisponível, o tipo de falha de cada fonte.
- Os valores de cada fonte são mantidos separados no registro. Não há fusão numérica (média, substituição de uma fonte pela outra).
- Um registro nunca apresenta um valor de fonte indisponível, nem um valor obtido antes da última falha identificada dessa fonte, como se fosse atual.

### Estados de operação

- O estado de cada ciclo é determinado pela disponibilidade das fontes naquele ciclo:

  | Fonte local (DHT11) | Fonte externa (Open-Meteo) | Estado |
  | --- | --- | --- |
  | Disponível | Disponível | Normal |
  | Indisponível | Disponível | Degradado — falha local |
  | Disponível | Indisponível | Degradado — falha externa |
  | Indisponível | Indisponível | Erro |

- Transições entre quaisquer dois estados são permitidas, inclusive Normal → Erro em um único ciclo.
- Toda transição gera um evento contendo o estado anterior, o novo estado e o identificador do ciclo. A transição para Normal a partir de Degradado ou Erro é identificada como recuperação.
- O primeiro estado reportado resulta do primeiro ciclo concluído. Nenhum estado Normal é presumido antes disso.
- O estado Erro não interrompe a biblioteca: os ciclos seguintes continuam tentando obter as duas fontes.
- A recuperação é automática e não exige reinicializar o dispositivo nem intervenção da aplicação.

### Verificação e experimentos

- A suíte de testes automatizados executa sem ESP32, sem DHT11 e sem rede, usando fontes simuladas e respostas externas gravadas.
- A suíte cobre as 4 combinações de disponibilidade, as transições (inclusive recuperação) e os limites de validação.
- Os cenários C1–C5 são executados no ESP32 com repetições em número definido no protocolo experimental.
- As falhas são induzidas por procedimento documentado e reproduzível (por exemplo, desconexão do sensor, desativação do Wi-Fi ou bloqueio do acesso à API).
- A latência é medida para as operações principais: leitura local, obtenção externa, validação e integração, e ciclo completo.
- A RAM é medida durante a execução, e a Flash corresponde ao tamanho final do firmware.
- Os resultados são armazenados em CSV com, no mínimo: cenário, repetição, ciclo, estado esperado, estado observado, validade e tipo de falha por fonte, e latências.
- A Open-Meteo é tratada como contexto nas análises, nunca como referência de verdade para o DHT11.

---

## 7. Casos de Uso

### UC-001 — Obter registro integrado em operação normal (C1)

**Ator:** Aplicação embarcada

**Pré-condições:** Biblioteca inicializada; DHT11 conectado e respondendo; Wi-Fi conectado; Open-Meteo acessível.

**Fluxo:**
1. A aplicação solicita um ciclo de aquisição.
2. A biblioteca lê o DHT11.
3. A biblioteca obtém a leitura externa (consulta ou reutilização dentro do intervalo de consulta).
4. A biblioteca valida as duas leituras.
5. A biblioteca determina o estado e produz o registro integrado.
6. A aplicação consome o registro.

**Critérios de aceite:**
- O registro contém temperatura e umidade das duas fontes, cada valor identificado pela origem.
- As duas fontes aparecem como válidas e o estado é Normal.
- A idade da leitura externa aparece no registro.
- Uma divergência entre os valores das fontes não altera o estado.

---

### UC-002 — Operar em modo degradado por falha local (C2)

**Ator:** Aplicação embarcada

**Pré-condições:** Open-Meteo acessível; DHT11 desconectado, sem resposta ou com leitura inválida.

**Fluxo:**
1. A aplicação solicita um ciclo de aquisição.
2. A leitura do DHT11 falha ou é reprovada na validação.
3. A biblioteca obtém e valida a leitura externa.
4. A biblioteca produz o registro com estado Degradado — falha local.
5. Se o estado anterior era outro, a biblioteca emite o evento de transição.

**Critérios de aceite:**
- Estado igual a Degradado — falha local.
- Nenhum valor local aparece como válido; o tipo de falha local ou o motivo da invalidação está identificado.
- Os valores externos estão presentes, válidos e identificados como externos.
- O ciclo seguinte volta a tentar ler o DHT11.

---

### UC-003 — Operar em modo degradado por falha externa (C3)

**Ator:** Aplicação embarcada

**Pré-condições:** DHT11 respondendo; Wi-Fi desconectado, API inacessível, lenta ou com resposta inválida.

**Fluxo:**
1. A aplicação solicita um ciclo de aquisição.
2. A biblioteca lê e valida o DHT11.
3. A obtenção externa falha, excede o tempo limite ou é reprovada na validação.
4. A biblioteca descarta a leitura externa anterior e produz o registro com estado Degradado — falha externa.
5. Se o estado anterior era outro, a biblioteca emite o evento de transição.

**Critérios de aceite:**
- Estado igual a Degradado — falha externa.
- O registro é produzido mesmo quando a consulta externa excede o tempo limite.
- O tipo de falha externa (sem conectividade, tempo limite, resposta de erro, resposta malformada) está identificado.
- Os valores locais estão presentes, válidos e identificados como locais.
- Depois de a falha ser identificada, nenhum valor externo obtido antes dela aparece no registro.

---

### UC-004 — Sinalizar erro por falha simultânea (C4)

**Ator:** Aplicação embarcada

**Pré-condições:** DHT11 e Open-Meteo indisponíveis.

**Fluxo:**
1. A aplicação solicita um ciclo de aquisição.
2. As obtenções local e externa falham ou são reprovadas na validação.
3. A biblioteca produz o registro com estado Erro.
4. Se o estado anterior era outro, a biblioteca emite o evento de transição.

**Critérios de aceite:**
- Estado igual a Erro.
- O registro é produzido e não contém nenhum valor marcado como válido.
- Os tipos de falha das duas fontes estão identificados.
- A biblioteca não entra em pânico, não reinicia o dispositivo e continua executando os ciclos seguintes.

---

### UC-005 — Recuperar a operação normal (C5)

**Ator:** Aplicação embarcada

**Pré-condições:** Biblioteca em estado Degradado ou Erro há pelo menos um ciclo.

**Fluxo:**
1. As fontes indisponíveis são restabelecidas.
2. A aplicação continua solicitando ciclos.
3. A biblioteca obtém leituras válidas das duas fontes.
4. A biblioteca produz o registro com estado Normal e emite o evento de recuperação.

**Critérios de aceite:**
- O estado passa a Normal no primeiro ciclo em que as duas fontes fornecem leituras válidas.
- O evento de transição traz o estado anterior, o estado Normal e o identificador do ciclo, e é identificado como recuperação.
- No primeiro registro Normal, os valores das fontes que estavam indisponíveis foram obtidos após o restabelecimento.
- Nenhuma reinicialização do dispositivo nem ação da aplicação é necessária.

---

### UC-006 — Rejeitar leitura inválida

**Ator:** Aplicação embarcada

**Pré-condições:** Uma fonte retorna valor fora da faixa ou não numérico.

**Fluxo:**
1. A biblioteca recebe a leitura da fonte.
2. A validação classifica a leitura como inválida e identifica o motivo.
3. A fonte é tratada como indisponível no ciclo.
4. O estado e o registro refletem a indisponibilidade.

**Critérios de aceite:**
- O valor inválido nunca aparece como válido no registro.
- O motivo (fora de faixa, não numérico) está identificado.
- O estado resultante segue a tabela de estados de operação.
- Valores exatamente nos limites da faixa são aceitos; valores imediatamente fora são rejeitados.

---

### UC-007 — Executar testes automatizados sem hardware

**Ator:** Desenvolvedor ou pesquisador

**Pré-condições:** Ambiente de desenvolvimento com toolchain Rust instalada.

**Fluxo:**
1. O ator executa a suíte de testes no computador de desenvolvimento.
2. A suíte reproduz C1–C5 e os casos de validação com fontes simuladas e respostas externas gravadas.
3. O ator executa a análise estática.

**Critérios de aceite:**
- A suíte executa sem ESP32, sem DHT11 e sem acesso à rede.
- Estão cobertas as 4 combinações de disponibilidade, as transições (inclusive recuperação) e os limites de validação.
- Todos os testes passam.
- A análise estática termina sem avisos não justificados.

---

### UC-008 — Executar cenário experimental e coletar métricas

**Ator:** Pesquisador

**Pré-condições:** ESP32 com DHT11 montado e firmware de experimento gravado; rede Wi-Fi disponível; computador de apoio conectado.

**Fluxo:**
1. O pesquisador seleciona um cenário (C1–C5).
2. O pesquisador induz a condição das fontes seguindo o procedimento documentado.
3. O firmware executa o número definido de ciclos e repetições.
4. O firmware emite registros, eventos de transição e latências.
5. Os dados são salvos em CSV.
6. A RAM é medida durante a execução e a Flash é obtida do firmware final.

**Critérios de aceite:**
- Cada linha do CSV identifica cenário, repetição, ciclo, estado esperado, estado observado, validade e tipo de falha por fonte, e latências.
- A correspondência entre estado esperado e observado pode ser calculada automaticamente a partir do CSV.
- O procedimento de indução de falhas de cada cenário está documentado.
- Os dados brutos ficam versionados no repositório.

---

### UC-009 — Reproduzir os resultados a partir do repositório

**Ator:** Avaliador

**Fluxo:**
1. O avaliador clona o repositório.
2. Segue o README para preparar o ambiente, montar o circuito e gravar o firmware.
3. Executa a suíte de testes.
4. Executa ao menos um cenário experimental.
5. Compara os resultados com os dados brutos e o relatório técnico.

**Critérios de aceite:**
- O README lista hardware, esquema de ligação, versões das ferramentas e comandos de compilação, gravação e teste.
- Os testes passam em um ambiente limpo seguindo apenas o README.
- O repositório tem licença, dados brutos e procedimento de análise.

---

## 8. Riscos e Mitigações

| Risco | Probabilidade | Impacto | Mitigação |
| --- | --- | --- | --- |
| Dificuldades na integração do Rust com o ESP32 | Média | Alto | Restringir a implementação às abstrações e funcionalidades essenciais, usando HALs e bibliotecas existentes e consolidadas; configurar o alvo ESP32 logo no início |
| Instabilidade da Open-Meteo ou do Wi-Fi durante os testes | Média | Médio | Usar respostas simuladas ou gravadas nos testes automatizados; manter a API real apenas na validação experimental |
| Tempo insuficiente para implementação, testes e validação | Média | Alto | Priorizar o núcleo funcional, os testes e os cenários C1–C5; adiar funcionalidades complementares |
| Falhas intermitentes do DHT11 (temporização, integridade) distorcerem C1 | Média | Médio | Registrar o tipo de cada falha; respeitar o intervalo mínimo de amostragem; relatar a taxa de falhas espontâneas separadamente das induzidas |
| Instrumentação alterar a latência medida | Média | Médio | Guardar as medições em memória e exportar fora da janela medida; quantificar o custo da própria instrumentação |
| Custo de RAM/Flash da pilha de rede mascarar o custo próprio da biblioteca | Alta | Médio | Medir também um firmware de referência sem a biblioteca e relatar a diferença |
| Indução manual de falhas pouco reproduzível | Média | Médio | Documentar o procedimento passo a passo; quando possível, induzir falhas de forma controlada por software |
| Consultas frequentes excederem os limites de uso da Open-Meteo | Baixa | Baixo | Intervalo de consulta externa configurável e conservador, independente do ciclo local; revisar a nova tentativa a cada ciclo durante indisponibilidade (D11) |
| Consulta externa lenta atrasar o ciclo inteiro, afetando a cadência do DHT11 e a latência medida em C3 | Média | Médio | Tempo limite curto; relatar a latência de C3 separadamente; medir a duração do ciclo sob tempo limite |
| Licença aberta conflitar com a política de propriedade intelectual da UFRR ou com o registro de software previsto | Baixa | Médio | Confirmar titularidade e licença com o orientador e o núcleo de inovação antes de tornar o repositório público |

---

## 9. Roadmap

### Estado atual (setembro de 2026 — fase de revisão e protocolo)

O repositório está organizado como workspace, com a biblioteca em `sense-bridge/` separada do caso de uso (`firmware/esp32/`, por enquanto só um esqueleto que não será publicado; outras placas entrariam como projetos irmãos em `firmware/`). A biblioteca já tem os metadados, o README próprio e a licença dupla MIT/Apache-2.0 exigidos para publicação no crates.io.

Parte do núcleo previsto para a fase de implementação foi antecipada. A biblioteca, independente de hardware, já implementa o modelo de leituras e falhas por fonte, a validação por faixa e por valores não numéricos, os estados de operação com transições e recuperação, o registro integrado por ciclo, a reutilização da leitura externa dentro do intervalo de consulta e as fontes simuladas. São 22 testes passando no computador de desenvolvimento, cobrindo os cenários C1–C5, e a análise estática não aponta avisos. As faixas de validação continuam provisórias (D3, D4); o intervalo entre consultas externas foi fechado em 15 min (D5a), enquanto o intervalo do ciclo local e o tempo limite seguem sem valor (D5b, D5c). Ainda não há firmware funcional, alvo ESP32 configurado, aquisição real, interpretação da resposta da Open-Meteo nem instrumentação.

### Cronograma

Datas aproximadas, contadas para trás a partir da entrega de 26/11/2026 (11 semanas).

| Fase | Semanas | Período aproximado | Atividades principais |
| --- | --- | --- | --- |
| Revisão e protocolo | 1–2 | 10/09 – 23/09 | Revisão técnica; definição da API, requisitos, estados, cenários e métricas |
| Implementação | 3–5 | 24/09 – 14/10 | Leitura do DHT11; integração com a Open-Meteo; modelo de dados; estados e recuperação |
| Testes e instrumentação | 6–7 | 15/10 – 28/10 | Testes automatizados; instrumentação de latência; levantamento de RAM e Flash; validação dos cenários |
| Execução dos experimentos | 8–9 | 29/10 – 11/11 | Execução repetida de C1–C5; coleta dos dados |
| Análise e redação | 10 | 12/11 – 18/11 | Análise das métricas; documentação da API; README e relatório técnico |
| Vídeo e entrega final | 11 | 19/11 – 26/11 | Demonstração funcional; revisão do repositório; entrega |

### Curto prazo (semanas 2–7)

- Fechar as decisões que continuam em aberto — ver [Decisões de projeto](#12-decisões-de-projeto) — D3, D4, D5b, D5c, D6, D7, D10 e D12, e o protocolo experimental.
- Criar o firmware em `firmware/esp32/` com toolchain e alvo ESP32, compilando a biblioteca para o alvo.
- Implementar na biblioteca a interpretação da resposta da Open-Meteo, testada com respostas gravadas.
- Integrar no firmware a leitura do DHT11 e a consulta à Open-Meteo com tempo limite.
- Definir as colunas do CSV do protocolo e gerá-lo no firmware.
- Instrumentar latência e medir RAM e Flash.
- Automatizar a execução dos testes e da análise estática.
- Declarar `#![no_std]` na biblioteca e incluir o README da crate como doc-test, para que o compilador verifique o que a documentação afirma.

### Médio prazo (semanas 8–11)

- Executar C1–C5 repetidamente no ESP32 e versionar os CSVs brutos.
- Analisar corretude, detecção de falhas, recuperação, latência, RAM e Flash.
- Escrever a documentação da API, os exemplos de uso e o README de reprodutibilidade.
- Escrever o relatório técnico e gravar o vídeo de demonstração (3–5 min).

### Longo prazo (pós-disciplina)

- Publicar o código em repositório público e, se a maturidade permitir, publicar a biblioteca como crate.
- Avaliar o registro de software e a publicação do conjunto de dados experimentais.

---

## 10. Critérios de Sucesso

### Qualitativos

- Em nenhum cenário um valor inválido aparece como válido para a aplicação.
- O estado reportado corresponde sempre à disponibilidade das fontes definida nas regras de negócio, e falhas e recuperações podem ser identificadas pelos registros e eventos, sem inspecionar o código.
- A queda de uma fonte não interrompe a entrega de dados da outra.
- O núcleo da biblioteca pode ser verificado inteiramente no computador de desenvolvimento, sem hardware.
- Uma pessoa externa ao projeto consegue reproduzir os testes e ao menos um cenário experimental seguindo apenas o README.

### Quantitativos

| Critério | Meta |
| --- | --- |
| Registros com valor inválido marcado como válido | 0 |
| Correspondência entre estado esperado e observado (C1–C5, todas as repetições; estado esperado considerando o atraso de identificação de D10) | 100% |
| Ciclos sem registro produzido | 0 |
| Transições de estado sem evento emitido | 0 |
| Ciclos até a recuperação após o restabelecimento das duas fontes (C5) | 1 (primeiro ciclo com ambas válidas) |
| Cenários cobertos por testes automatizados sem hardware | 5 de 5 |
| Cenários executados no ESP32 | 5 de 5 |
| Testes automatizados passando | 100% |
| Avisos não justificados da análise estática | 0 |
| Repetições por cenário | A definir no protocolo |
| Latência por operação (leitura local, obtenção externa, validação/integração, ciclo) | Medir e relatar (média e dispersão); sem meta prévia |
| RAM em execução | Medir e relatar; sem meta prévia |
| Flash (tamanho do firmware) | Medir e relatar; sem meta prévia |
| Duração do vídeo de demonstração | 3–5 min |

---

## 11. Fora do Escopo

- Implementação do protocolo de comunicação de baixo nível do DHT11.
- Criação de hardware dedicado ou de nova arquitetura de processamento.
- Solução completa de automação ou monitoramento ambiental: painéis, alertas, atuadores, interface de usuário.
- Medição de consumo energético.
- Calibração, correção ou fusão numérica das leituras do DHT11 com dados da Open-Meteo.
- Uso da Open-Meteo como referência absoluta de verdade.
- Previsão meteorológica ou variáveis além de temperatura e umidade.
- Envio de registros a servidores ou serviços em nuvem e armazenamento de longo prazo no dispositivo.
- Firmware e validação experimental para outras placas (por exemplo, Arduino Mega), outros sensores ou múltiplos sensores simultâneos na entrega da disciplina.
- Configuração de Wi-Fi em tempo de execução (provisionamento) e atualização remota de firmware (OTA).
- Publicação da crate e registro de software dentro do prazo da disciplina (ficam para depois).

---

## 12. Decisões de projeto

Registro das decisões que moldam o comportamento da biblioteca e o protocolo experimental. As fechadas ficam aqui para consulta e não devem ser reabertas sem revisar as regras de negócio e os critérios de sucesso que dependem delas.

### Decisões fechadas

| # | Decisão | Resolução | Onde está sustentada |
| --- | --- | --- | --- |
| D1 | "Recuperação" deve ser um estado explícito da máquina de estados ou apenas um evento de transição para Normal? | Evento de transição; a máquina mantém 4 estados | Definições e regras de estados de operação (§6); critério "transições de estado sem evento emitido = 0" (§10); `state.rs`, com `Transition::is_recovery()` e teste dedicado |
| D2 | Adotar histerese (N ciclos consecutivos) para declarar falha ou recuperação? | Não; reação no próprio ciclo (N = 1) | Regra "o estado de cada ciclo é determinado pela disponibilidade das fontes naquele ciclo" (§6); meta de recuperação em 1 ciclo (§10); teste de recuperação de C5 |
| D5a | Intervalo entre consultas à Open-Meteo | 15 min, configurável em `Config::external_refresh_ms` | Regra de reutilização da leitura externa (§6); `bridge.rs`; testes de reuso dentro do intervalo e de descarte após falha |
| D8 | Referência para RAM e Flash: firmware completo ou diferença em relação a um firmware base sem a biblioteca? | Relatar os dois | Mitigação do risco de a pilha de rede mascarar o custo próprio da biblioteca (§8); regra de medição de RAM e Flash (§6) |
| D9 | Licença do repositório | Licença dupla `MIT OR Apache-2.0` | `sense-bridge/Cargo.toml`, `LICENSE-MIT` e `LICENSE-APACHE` na raiz, README do projeto e da crate |
| D11 | Com que frequência tentar de novo enquanto a fonte externa está indisponível? | A cada ciclo | Regra de nova tentativa (§6); critério de aceite de UC-002; meta de recuperação em 1 ciclo (§10); `bridge.rs` e o teste de C4, que confirma uma consulta por ciclo durante a indisponibilidade |

Se a Open-Meteo passar a limitar requisições, o ajuste é no intervalo do ciclo local (D5b), não na frequência de nova tentativa: reduzir a frequência quebraria a meta de recuperação em 1 ciclo.

### Decisões em aberto

| # | Decisão | Proposta inicial | O que falta |
| --- | --- | --- | --- |
| D3 | Faixas de validade do DHT11 | 0–50 °C e 20–90 %UR | Confirmar na versão do manual referenciada; hoje implementada como provisória em `Limits::DHT11` |
| D4 | Faixas físicas plausíveis para a leitura externa | −90 a 60 °C e 0–100 %UR | Declarar a fonte que sustenta esses limites; hoje implementada como provisória em `Limits::PLAUSIBLE` |
| D5b | Intervalo do ciclo local | A definir | Valor e justificativa. Fica no firmware, não na biblioteca; condiciona a cadência do DHT11 e o volume de consultas à API |
| D5c | Tempo limite da consulta à Open-Meteo | Curto, valor a definir | Valor e justificativa. Fica no firmware; condiciona a latência medida em C3 |
| D6 | Coordenadas usadas na consulta à Open-Meteo | Coordenadas do local dos experimentos | Definir latitude e longitude e onde elas ficam configuradas |
| D7 | Número de repetições e duração de cada execução por cenário | A definir | Fechar junto com o protocolo experimental; bloqueia a execução de C1–C5 |
| D10 | Uma falha externa durante a reutilização só é identificada na consulta seguinte. Com intervalo de 15 min, a queda da rede pode levar até 15 min para aparecer, e C3 fica sem identificação imediata. Aceitar e medir esse atraso, usar intervalo curto nos experimentos, ou permitir que o firmware invalide a leitura reutilizada ao detectar perda de conexão? | Permitir a invalidação pelo firmware e medir o tempo de identificação | Decidir antes de definir o estado esperado de C3; hoje a biblioteca só identifica na consulta seguinte, e a meta de correspondência de §10 depende dessa escolha |
| D12 | Pilha do ESP32: `esp-hal` (`no_std`) ou `esp-idf-hal`/`esp-idf-svc` (com `std`)? | A definir | Decide como entram Wi-Fi, TLS e HTTP no firmware e se há alocador disponível; bloqueia toda a fase de implementação |
