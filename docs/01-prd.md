# Documento de Requisitos de Produto — SenseBridge

> **Base:** Projeto de Pesquisa PTT4 — *Biblioteca Rust para Sistemas IoT Embarcados Context-Aware: Integração de Dados Ambientais e Tolerância a Falhas* (PPCA/UFRR, semestre 2026.2).
> **Atualizado em:** 04/10/2026 · **Entrega final:** 26/11/2026

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

### Estado atual — 04/10/2026

| Capacidade | Status | Notas |
| --- | --- | --- |
| Separação entre biblioteca e firmware | ✅ 100% | Biblioteca em `sense-bridge/`; firmware de referência em `firmware/esp32/`; console experimental em `tools/console/` |
| Núcleo independente de hardware | ✅ 100% | O núcleo é testado no computador sem ESP32, sensor ou rede; `no_std` declarado e doctests executáveis |
| Modelo das leituras e das falhas por fonte | ✅ 100% | Tipos distintos para fonte local e externa, com falhas explicitamente identificadas |
| Estados de operação | ✅ 100% | `Normal`, `DegradedLocal`, `DegradedExternal` e `Error` |
| Transições de estado e recuperação | ✅ 100% | Transições emitidas apenas quando há mudança de estado; recuperação identificada no retorno a `Normal` |
| Validação de dados | 🔶 90% | Validação implementada e testada; permanecem pendentes apenas as referências formais das faixas D3 e D4 |
| Registro integrado por ciclo | ✅ 100% | Ciclo, instante, estado, leituras/falhas, idade da leitura externa, transição e recuperação |
| Intervalo de consulta externa e cache | ✅ 100% | `external_refresh_ms = 900000` ms (15 min); leitura externa válida pode ser reutilizada |
| Fontes simuladas | ✅ 100% | Utilizadas nos testes determinísticos dos cenários C1–C5 |
| Interpretação da Open-Meteo | ✅ 100% | Parser implementado; respostas válidas, truncadas, inválidas e de erro cobertas por testes |
| Consulta real à Open-Meteo via Wi-Fi | ✅ 100% | Executada com sucesso no ESP32, com HTTP e tempo limite |
| Firmware Rust no ESP32 | ✅ 100% | Toolchain configurada, firmware compilado, gravado e executado no hardware |
| Integração do DHT11 | ✅ 100% | Driver, abstração e leitura real do DHT11 concluídos e validados; SB-10 encerrada |
| Testes automatizados | ✅ 100% | 53 testes aprovados: 18 unitários, 7 Open-Meteo, 12 cenários, 14 console e 2 doctests |
| Análise estática | 🔶 75% | Estrutura preparada; execução final deve ser repetida antes da entrega |
| Instrumentação de latência | ✅ 100% | `read_us`, `fetch_us` e `cycle_us` registrados pelo firmware |
| Medição de RAM | ✅ 100% | Heap e stack monitorados em execução; segmentos estáticos também medidos |
| Medição de Flash | 🔶 85% | Firmware completo medido; falta produzir o firmware-base previsto em D8 para obter o custo incremental |
| Registro experimental em CSV | 🔶 75% | Console grava CSV e TXT; C2 real foi capturado; ainda falta padronizar cenário, repetição e estado esperado no CSV final |
| Cenários C1–C5 automatizados | ✅ 100% | Todos os cinco cenários aprovados |
| Cenários C1–C5 em hardware | 🔶 30% | A leitura real do DHT11 já foi validada e C2 foi observado diretamente no ESP32; ainda falta executar o protocolo físico final e repetido de C1–C5 |
| Documentação da API e exemplos | 🔶 80% | README, documentação da crate, exemplos e doctests existentes; revisão final de reprodutibilidade pendente |
| Avaliação experimental | ✅ 100% | Documento `docs/03-avaliacao-experimental.md` criado com resultados atuais |
| Entregáveis finais | 🔶 65% | Código, testes e resultados principais existentes; faltam fechamento experimental, README final, relatório completo e vídeo |

### Resultados já obtidos

A implementação atual apresenta:

- 53 testes automatizados aprovados e nenhuma falha;
- validação dos cenários C1–C5 com fontes simuladas;
- integração real do ESP32 com Wi-Fi e Open-Meteo;
- observação real do cenário C2 (`DegradedLocal`) em hardware;
- captura experimental estruturada em CSV/TXT;
- latência estável média no C2 de aproximadamente 18,59 ms;
- medição de Flash, RAM estática, heap e stack;
- documentação específica da avaliação experimental.

### Pendências para a entrega

| Pendência | Severidade | Área |
| --- | --- | --- |
| Executar e registrar C1–C5 no ESP32 segundo o protocolo final | Alta | Experimentos |
| Definir formalmente o número de repetições por cenário (D7) | Alta | Protocolo |
| Fechar D10 sobre identificação de falha externa durante período de reutilização | Média | Resiliência |
| Confirmar e referenciar formalmente as faixas de validação D3 e D4 | Média | Validação |
| Produzir firmware-base sem SenseBridge para comparação de RAM/Flash prevista em D8 | Média | Métricas |
| Completar as colunas finais do CSV experimental | Média | Evidências |
| Executar análise estática final | Média | Qualidade |
| Atualizar README completo de reprodutibilidade | Média | Documentação |
| Consolidar relatório técnico e vídeo de demonstração | Alta | Entrega |

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

### Estado atual — 04/10/2026

O desenvolvimento está adiantado em relação ao cronograma original.

O núcleo da biblioteca, os estados de operação, as transições, a recuperação, as fontes simuladas, a integração com a Open-Meteo, o firmware ESP32, o console de captura e a instrumentação já estão implementados.

A suíte atual possui 53 testes aprovados.

Os cenários C1–C5 estão validados de forma automatizada. O cenário C2 também foi observado em execução real no ESP32, quando o sensor local permaneceu indisponível e a fonte Open-Meteo continuou operacional.

A integração real do DHT11 já foi concluída e validada. Em uma montagem posterior, um shield ESP-01S DHT11 v1.0 específico recebeu alimentação, mas não respondeu ao protocolo de leitura; esse fato é mantido apenas como registro experimental e não representa bloqueio do SenseBridge.

### Cronograma de fechamento

| Fase | Situação em 04/10 | Próxima ação |
| --- | --- | --- |
| Revisão e protocolo | ✅ Concluída em grande parte | Decisões D3, D4, D7 e D10 fechadas |
| Implementação do núcleo | ✅ Concluída | Apenas correções identificadas na revisão final |
| Firmware ESP32 | ✅ Concluído | Manter configuração reproduzível |
| Open-Meteo | ✅ Concluída | Apenas documentação final |
| DHT11 físico | ✅ Concluído | Leitura real validada; SB-10 encerrada |
| Testes automatizados | ✅ 53/53 | Reexecutar na versão final |
| Instrumentação | ✅ Concluída | Completar baseline de RAM/Flash |
| Experimentos | 🔶 Parcial | Executar protocolo físico final C1–C5 |
| Análise de resultados | 🔶 Avançada | Consolidar todas as repetições |
| Documentação | 🔶 Avançada | Atualizar README, PRD e arquitetura |
| Relatório | 🔶 Em andamento | Incorporar resultados finais |
| Vídeo | 🔴 Pendente | Produzir após congelamento do firmware |

### Próximas atividades

1. Revisar PRD e arquitetura contra a implementação real.
2. Completar a integração física da fonte local.
4. Definir o número de repetições do protocolo experimental.
5. Executar C1–C5 em hardware conforme o protocolo final.
6. Gerar os CSVs finais com cenário, repetição, estado esperado e estado observado.
7. Produzir firmware-base e calcular o custo incremental de RAM/Flash.
8. Executar a análise estática final.
9. Consolidar resultados no relatório técnico.
10. Atualizar o README de reprodutibilidade.
11. Produzir o vídeo de demonstração de 3–5 minutos.

### Pós-disciplina

- avaliar publicação da biblioteca como crate;
- avaliar registro de software;
- avaliar publicação do conjunto de dados experimental;
- testar outros sensores e plataformas sem alterar o núcleo da biblioteca.

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

Registro das decisões que moldam o comportamento da biblioteca e o protocolo experimental.

### Decisões fechadas

| # | Decisão | Resolução |
| --- | --- | --- |
| D1 | Recuperação como estado ou evento | Recuperação é evento de transição para `Normal`; a máquina possui quatro estados |
| D2 | Histerese para falha/recuperação | Não; reação no próprio ciclo (`N = 1`) |
| D3 | Faixas de validade do DHT11 | 0–50 °C e 20–90 %UR |
| D4 | Faixas físicas plausíveis da fonte externa | −90 a 60 °C e 0–100 %UR |
| D5a | Intervalo normal entre consultas à Open-Meteo | 15 min (`900000 ms`), configurável |
| D5b | Intervalo do ciclo local | 5 s (`5000 ms`) |
| D5c | Tempo limite da Open-Meteo | 3 s (`3000 ms`) |
| D6 | Coordenadas da Open-Meteo | Configuráveis em `cfg.toml`; experimento atual usa latitude `2.82` e longitude `-60.67` |
| D7 | Repetições do protocolo | 5 repetições por cenário, com 10 ciclos por repetição |
| D8 | Referência para RAM e Flash | Relatar firmware completo e diferença contra firmware-base sem a biblioteca |
| D9 | Licença | Licença dupla `MIT OR Apache-2.0` |
| D10 | Falha externa durante reutilização | Não alterar a arquitetura; nos experimentos C3–C5 usar `external_refresh_ms = 5000` para verificar a fonte externa aproximadamente a cada ciclo |
| D11 | Nova tentativa após falha externa | Nova tentativa em cada ciclo enquanto a fonte estiver indisponível |
| D12 | Pilha ESP32 | `esp-idf` / `esp-idf-svc`, com `std`, Wi-Fi, TLS e HTTP |

### Protocolo experimental consolidado

Para os cenários C1–C5 serão utilizadas:

- 5 repetições por cenário;
- 10 ciclos por repetição;
- intervalo de ciclo de 5 s;
- 50 ciclos por cenário;
- 250 ciclos no conjunto C1–C5.

Nos experimentos que exigem observação rápida da fonte externa, será usado:

```toml
external_refresh_ms = 5000
```

A configuração normal da aplicação permanece:

```toml
external_refresh_ms = 900000
```

### Procedimento específico do C5

Em cada repetição do cenário de recuperação:

1. iniciar em condição degradada ou de erro;
2. manter a falha durante os 3 primeiros ciclos;
3. restabelecer as fontes;
4. observar os 7 ciclos seguintes;
5. o primeiro ciclo em que ambas as fontes estiverem válidas deve apresentar estado `Normal` e recuperação.

### Registro experimental do DHT11 disponível

A integração e a leitura real do DHT11 já foram concluídas e validadas no projeto, permitindo o encerramento da SB-10.

Em uma montagem posterior, o shield ESP-01S DHT11 v1.0 utilizado em outro conjunto físico recebeu alimentação, mas não produziu resposta válida na linha de dados quando conectado ao ESP32.

Esse episódio é mantido apenas como evidência de uma condição específica de hardware/montagem e não representa pendência da implementação do SenseBridge nem reabre a SB-10.

### Configuração atual do firmware

```toml
cycle_interval_ms = 5000
http_timeout_ms = 3000
external_refresh_ms = 900000
dht11_gpio = 4
```
