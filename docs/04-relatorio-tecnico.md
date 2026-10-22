# Relatório Técnico — SenseBridge

> **Rascunho de 21/10/2026**, montado a partir do [projeto de pesquisa](00-Projeto%20de%20Pesquisa%20-%20PTT4.pdf), do [PRD](01-prd.md), da [arquitetura](02-arch.md) e da [avaliação experimental](03-avaliacao-experimental.md). Trechos marcados com **[revisar]** pedem decisão ou complemento dos autores antes da entrega.

**Título:** Biblioteca Rust para Sistemas IoT Embarcados Context-Aware: Integração de Dados Ambientais e Tolerância a Falhas  
**Discentes:** Ezequias Carlos Ferreira da Silva e Marcelo Gomes Barbosa  
**Orientador:** Prof. Herbert Oliveira Rocha  
**Disciplina:** Arquitetura e Organização de Computadores — PPCA/UFRR, 2026.2  
**Trilha:** PTT4 — Biblioteca de software embarcado  
**Repositório:** <https://github.com/msc-sed-ufrr/SenseBridge>

## Resumo

**[revisar]** Este relatório apresenta o SenseBridge, uma biblioteca em Rust para ESP32 que integra dados ambientais de uma fonte local, o sensor DHT11, e de uma fonte externa de contexto, a API Open-Meteo. A cada ciclo, a biblioteca valida as leituras, identifica a origem e o tipo de falha de cada fonte e determina um de quatro estados de operação: normal, degradado por falha local, degradado por falha externa ou erro. A biblioteca é `no_std`, não aloca memória e não tem dependências; o acesso ao hardware fica em traits implementadas pelo firmware. A verificação reúne 62 testes automatizados executados sem hardware e análise estática sem avisos. No ESP32, os cenários de operação normal, falha local, falha externa, falha simultânea e recuperação foram executados em 5 repetições de 10 ciclos cada, todas aprovadas. Os ciclos sem consulta externa duraram entre 18 e 22 ms, e a solução acrescentou cerca de 23,3 KiB de Flash e 13,2 KiB de RAM estática em relação a um firmware-base sem a biblioteca.

**Palavras-chave:** sistemas embarcados; Rust; ESP32; tolerância a falhas; integração de dados.

## 1. Introdução

### 1.1 Contexto

Sistemas IoT embarcados coletam dados do ambiente por sensores, e a integração de fontes diferentes traz desafios de heterogeneidade, qualidade, consistência e veracidade dos dados (DING et al., 2019; CHHETRI et al., 2024). Informações externas podem servir de contexto para as leituras locais (DUQUE et al., 2025), mas combinar fontes não garante, por si só, dados melhores: o sensor pode produzir leituras inválidas ou ausentes, e a fonte externa pode ficar indisponível ou divergir. Em um dispositivo conectado, essas falhas exigem comportamento definido, que evite a propagação silenciosa de dados inválidos e mantenha a operação possível com a fonte que restar.

Rust oferece segurança de memória e abstrações de baixo nível adequadas a dispositivos com recursos restritos. O trabalho investiga como uma biblioteca em Rust pode integrar dados locais e externos de forma controlada e quanto isso custa em latência e memória no ESP32.

### 1.2 Problema de pesquisa

Como projetar e avaliar uma biblioteca embarcada em Rust para ESP32 capaz de integrar dados ambientais locais e externos, manter comportamento definido diante da indisponibilidade parcial das fontes e apresentar custos mensuráveis de latência e memória?

### 1.3 Questões norteadoras

- **Q1.** Quais mecanismos de validação e integração são necessários para assegurar a corretude de dados ambientais provenientes de fontes locais e externas em uma biblioteca embarcada?
- **Q2.** Como a biblioteca deve tratar a indisponibilidade de uma das fontes para manter uma operação definida, incluindo modo degradado e recuperação?
- **Q3.** Quais evidências experimentais permitem avaliar a corretude dos registros, a detecção e recuperação de falhas, a latência das operações e o uso de RAM e Flash da biblioteca no ESP32?

### 1.4 Objetivos

O objetivo geral é desenvolver e validar uma biblioteca embarcada em Rust para ESP32 que integre dados ambientais locais e externos, com validação, operação degradada e recuperação explícitas, avaliando experimentalmente a corretude dos registros, a latência e o uso de memória. Os objetivos específicos são: especificar requisitos e propriedades de validade e tratamento de falhas; projetar uma arquitetura modular que separe aquisição, validação, integração e estados; implementar a biblioteca e integrá-la ao ESP32 com DHT11 e Open-Meteo; verificá-la com testes automatizados e análise estática; e validá-la experimentalmente nos cenários C1–C5.

## 2. Fundamentação e vínculo com a ementa

- **E/S e interrupções.** O DHT11 usa um protocolo serial de um fio em que cada bit é codificado pela duração de um pulso (cerca de 26 µs para 0 e 70 µs para 1). No ESP32, a tarefa do Wi-Fi roda no mesmo núcleo e pode preemptar a leitura por mais de 1 ms, o que corrompe o quadro de 40 bits. O firmware mascara as interrupções durante o quadro e mede os pulsos pelo contador de ciclos da CPU, que continua contando com as interrupções desligadas.
- **Mapeamento e abstração de hardware.** A biblioteca não acessa periféricos: a fonte local e a externa entram pelas traits `LocalSource` e `ExternalSource`, implementadas no firmware sobre o `esp-idf-svc`. Essa inversão de dependência permite verificar o núcleo no computador de desenvolvimento e reutilizá-lo em outra placa.
- **Organização do processador e aritmética.** O ESP32 usa núcleos Xtensa LX6 com unidade de ponto flutuante de precisão simples; por isso temperatura e umidade são `f32`, e a validação rejeita valores não finitos (NaN e infinito) antes de qualquer comparação de faixa.
- **Memória.** A Flash guarda o código e os dados constantes (`text` e `data`); a RAM estática soma `data` e `bss`, e o heap e a pilha variam em execução. A biblioteca não aloca, e o custo dela é medido como diferença em relação a um firmware-base.
- **Máquinas de estados.** O estado de operação é uma função da disponibilidade das duas fontes no ciclo, com quatro estados e transições registradas como eventos; a recuperação é a transição para o estado normal.

## 3. Metodologia

### 3.1 Requisitos e propriedades

Os requisitos estão no [PRD](01-prd.md). As propriedades centrais, usadas como critério de teste, são:

- nenhum valor reprovado na validação aparece como válido;
- cada valor identifica a sua origem, e cada falha, o seu tipo;
- as fontes não se misturam: não há média, substituição nem correção de uma pela outra, e a divergência entre elas não muda a validade nem o estado;
- o estado de cada ciclo é determinado pela disponibilidade das fontes naquele ciclo:

| DHT11 | Open-Meteo | Estado |
| --- | --- | --- |
| Disponível | Disponível | `Normal` |
| Indisponível | Disponível | `DegradedLocal` |
| Disponível | Indisponível | `DegradedExternal` |
| Indisponível | Indisponível | `Error` |

- uma leitura externa obtida antes da última falha nunca reaparece como atual;
- a recuperação acontece no primeiro ciclo em que as duas fontes voltam, sem reiniciar o dispositivo.

### 3.2 Arquitetura

O repositório separa a biblioteca (`sense-bridge/`), independente de hardware, o firmware de referência (`firmware/esp32/`) e o console de host (`tools/console/`). A biblioteca expõe `SenseBridge::cycle(now_ms)`, que lê a fonte local, obtém a externa (ou reutiliza a última leitura válida dentro do intervalo de consulta), valida as duas, determina o estado e devolve um `Record` com o resultado de cada fonte e a transição. As decisões que sustentam o desenho estão registradas como ADRs na [arquitetura](02-arch.md#8-decisões-de-arquitetura):

- fontes como traits e núcleo sem hardware (ADR-001);
- crate sem `std`, sem alocação e sem dependências (ADR-002);
- instante do ciclo fornecido pelo chamador, para que a biblioteca não leia relógio nem afete a latência medida (ADR-003);
- recuperação como evento de transição, sem histerese (ADR-004);
- descarte da leitura externa reutilizada na primeira falha (ADR-005);
- um `Result` por fonte no registro, sem fusão (ADR-006).

### 3.3 Implementação

- **Biblioteca.** Módulos de leitura e falhas, validação, estados, registro e ciclo, além das fontes simuladas (`sim`) e da interpretação da Open-Meteo (`open_meteo`), com um leitor JSON próprio, sem alocação e com profundidade limitada. Todos os itens públicos são documentados.
- **Firmware.** Implementa a fonte local (DHT11) e a externa (cliente HTTP da Open-Meteo com tempo limite sobre Wi-Fi), fornece o relógio monotônico e a cadência de 5 s, mede a duração da leitura local, da consulta externa e do ciclo, e emite uma linha CSV por ciclo pela serial, depois do trecho medido, com heap livre, heap mínimo e folga mínima da pilha.
- **Console de host.** Lê a serial, mostra os ciclos com o estado em cor e as transições destacadas, resume os ciclos por estado e por tipo de falha e grava a captura em TXT e CSV.

**[revisar]** O firmware atual lê o DHT11 com driver próprio, para identificar a etapa do protocolo em que a leitura falhou e reiniciar o sensor travado. O projeto de pesquisa (§6.2) exclui do escopo a implementação do protocolo de baixo nível do DHT11. Decidir com o orientador se o driver próprio fica, e com qual justificativa, ou se o firmware de referência volta à crate `dht-sensor`.

### 3.4 Verificação

- **Testes automatizados:** 62, executados sem ESP32, sensor ou rede — 20 unitários da biblioteca, 7 de interpretação de respostas reais gravadas da Open-Meteo, 12 de cenários C1–C5 e propriedades do ciclo, 21 do console e 2 exemplos da documentação.
- **Análise estática:** clippy com avisos tratados como erro na biblioteca, no console e no firmware.
- **Integração contínua:** a cada push, formatação, clippy, testes, documentação e empacotamento da crate, além da compilação com Rust 1.85 (a versão mínima declarada) e para um alvo sem `std`.
- O registro da verificação de um commit fica em [docs/evidencias/verificacao.md](evidencias/verificacao.md).

### 3.5 Protocolo experimental

| Parâmetro | Valor |
| --- | --- |
| Repetições | 5 por cenário, com 10 ciclos cada (250 ciclos no total) |
| Ciclo | 5 s |
| Tempo limite HTTP | 3 s |
| Intervalo de consulta externa | 15 min; 5 s em C3–C5, para observar a fonte externa a cada ciclo (D10) |
| Coordenadas da Open-Meteo | latitude 2,82; longitude −60,67 (Boa Vista/RR) |

| Cenário | Indução |
| --- | --- |
| C1 — Operação normal | Nenhuma falha |
| C2 — Falha local | Alimentação e sinal do DHT11 retirados |
| C3 — Falha externa | Firmware configurado para uma rede Wi-Fi inexistente |
| C4 — Falha simultânea | C2 e C3 combinados |
| C5 — Recuperação | DHT11 desligado nos ciclos iniciais e religado depois do ciclo 2 |

As métricas são a correspondência entre o estado esperado e o observado, as falhas por tipo, as transições, a duração da leitura local, da consulta externa e do ciclo, o heap e a pilha em execução e o tamanho do firmware. A Flash e a RAM estática foram medidas com `xtensa-esp32-elf-size` no firmware completo e em um firmware-base com a mesma infraestrutura (ESP-IDF, Wi-Fi, DHT11, HTTP, temporização e métricas), mas sem a biblioteca (D8).

## 4. Resultados

### 4.1 Cenários no ESP32

| Cenário | Estado esperado | Resultado |
| --- | --- | --- |
| C1 | `Normal` | 50/50 ciclos em `Normal` |
| C2 | `DegradedLocal` | 50/50 ciclos em `DegradedLocal`; 50/50 falhas locais `NoResponse` |
| C3 | `DegradedExternal` | 50/50 ciclos em `DegradedExternal`; 50/50 falhas externas `NoConnectivity` |
| C4 | `Error` | 50/50 ciclos em `Error`, com as duas falhas em todos os ciclos |
| C5 | Recuperação | 5/5 recuperações; o primeiro ciclo em `Normal` foi o 3 em quatro repetições e o 4 em uma, quando o DHT11 só voltou a responder nesse ciclo |

Uma execução de C5 teve um `Timeout` externo inesperado durante a falha local; foi excluída do conjunto oficial, preservada como evidência de interferência externa e repetida com sucesso.

### 4.2 Latência

| Cenário | Duração típica do ciclo |
| --- | --- |
| C1 | 21–22 ms nos ciclos sem consulta externa |
| C2 | 18–19 ms; média de 18.586,77 µs no regime estável da medição inicial |
| C3 | cerca de 22 ms |
| C4 | cerca de 18 ms |
| C5 | 0,9–1,3 s na maior parte dos ciclos, que consultam a Open-Meteo |

O ciclo é dominado pela consulta HTTP quando ela acontece: na medição inicial de C2, o primeiro ciclo, com a primeira consulta, levou 648.301 µs.

**[revisar]** O PRD (§10) pede média e dispersão por operação. A avaliação traz média, mínimo e máximo apenas para a medição inicial de C2; para os demais cenários, faixas típicas. Calcular média e desvio padrão de `read_us`, `fetch_us` e `cycle_us` por cenário a partir das capturas oficiais quando elas estiverem em `docs/evidencias/`.

### 4.3 Memória

| Métrica | SenseBridge | Firmware-base | Diferença |
| --- | ---: | ---: | ---: |
| Flash (`text` + `data`) | 1.122.030 B | 1.098.142 B | +23.888 B (+2,18%) |
| RAM estática (`data` + `bss`) | 211.853 B | 198.317 B | +13.536 B (+6,83%) |
| `bss` | 19.891 B | 19.891 B | 0 B |

Em execução, na captura inicial de C2, o menor heap livre desde o boot foi de 201.120 B e a menor folga da pilha da tarefa principal, de 11.980 B. Nos experimentos finais, o firmware não foi interrompido por falta de memória.

**[revisar]** O firmware atual, com o driver próprio do DHT11, é maior que o medido: `text` 939.016 B, `data` 192.642 B e `bss` 19.883 B. Registrar o commit medido ou repetir a medição, inclusive do firmware-base.

## 5. Discussão

**Q1 — validação e integração.** Três mecanismos sustentaram a corretude nos experimentos e nos testes. Primeiro, cada fonte tem a sua faixa de validade e a sua taxonomia de falhas, e um valor fora da faixa ou não finito torna a fonte indisponível no ciclo, com o motivo registrado. Segundo, o registro guarda um `Result` por fonte: o tipo não permite apresentar um valor sem origem nem um valor marcado como inválido. Terceiro, a leitura externa reutilizada é descartada na primeira falha, de modo que um valor anterior à falha nunca reaparece como atual. A recusa em fundir as fontes mantém a Open-Meteo como contexto, sem corrigir o DHT11.

**Q2 — indisponibilidade e recuperação.** Derivar o estado da disponibilidade de cada ciclo, sem histerese, tornou a tabela de estados verificável linha a linha e garantiu a recuperação no primeiro ciclo com as duas fontes válidas (C5). O custo é a oscilação de estado diante de falhas intermitentes, visível nos registros. A fonte externa indisponível é consultada de novo a cada ciclo; dentro do intervalo de reuso, porém, uma queda só é percebida na consulta seguinte, o que levou a usar 5 s de intervalo nos experimentos de falha externa (D10).

**Q3 — evidências.** O registro por ciclo, com estado, resultado de cada fonte, transição, durações e memória, permitiu comparar estado esperado e observado sem inspecionar o código. A medição fora do trecho medido evitou que a própria instrumentação distorcesse a latência, e a comparação com um firmware-base separou o custo da biblioteca do custo da pilha de rede.

### 5.1 Limitações e ameaças à validade

- Uma única placa, um único sensor e uma única localização foram usados.
- As falhas foram induzidas manualmente; a indução de C3 por rede inexistente não exercita tempo limite nem erro do serviço, cobertos apenas nos testes automatizados.
- O custo de memória medido é o da integração da solução no firmware, não o da crate isolada.
- A duração do ciclo com consulta externa depende da rede e do serviço, fora do controle do experimento.
- **[revisar]** Desvio de escopo do driver do DHT11 e diferença entre o firmware medido e o atual (seções 3.3 e 4.3).

## 6. Conclusão

**[revisar]** O SenseBridge mostrou que uma biblioteca `no_std`, sem alocação e sem dependências pode integrar uma fonte local e uma externa com comportamento definido diante de falhas parciais e totais, recuperando a operação normal no primeiro ciclo possível. Todos os cenários foram aprovados no ESP32, com ciclos de 18 a 22 ms sem consulta externa e um custo de cerca de 23,3 KiB de Flash e 13,2 KiB de RAM estática.

Como trabalhos futuros, ficam a publicação da biblioteca como crate (preparação em [docs/06](06-publicacao-crates-io.md)), o registro de software, a publicação do conjunto de dados experimentais e a validação com outros sensores e plataformas sem alterar o núcleo.

## Referências

AOSONG (GUANGZHOU) ELECTRONICS CO., LTD. **DHT11 Product Manual: Temperature and Humidity Module**. [S.l.]: Aosong, [s.d.]. Disponível em: <https://projectpoint.in/datasheets/pdf/DHT11.pdf>. Acesso em: 9 set. 2026.

CHHETRI, Tek Raj et al. Enabling privacy-aware interoperable and quality IoT data sharing with context. **Future Generation Computer Systems**, v. 157, p. 164–179, 2024. DOI: 10.1016/j.future.2024.03.039.

DING, Wenxiu et al. A survey on data fusion in internet of things: towards secure and privacy-preserving fusion. **Information Fusion**, v. 51, p. 129–144, 2019. DOI: 10.1016/j.inffus.2018.12.001.

DUQUE, Hector et al. Context-aware data driven sensor data analysis: with application to H2S concentration prediction in urban drainage networks. **Water Research X**, v. 28, 100346, 2025. DOI: 10.1016/j.wroa.2025.100346.

ESPRESSIF SYSTEMS. **ESP32 Technical Reference Manual**. Disponível em: <https://www.espressif.com/en/document-type/reference>. Acesso em: 9 set. 2026.

OPEN-METEO. **Open-Meteo API Documentation**. Disponível em: <https://open-meteo.com/en/docs>. Acesso em: 9 set. 2026.

RUST FOUNDATION. **The Embedded Rust Book**. Disponível em: <https://doc.rust-lang.org/stable/embedded-book/>. Acesso em: 9 set. 2026.

STALLINGS, William. **Arquitetura e organização de computadores**. 10. ed. São Paulo: Pearson Education do Brasil, 2017.

## Apêndice — Reprodutibilidade

O passo a passo para executar os testes, gravar o firmware e repetir os cenários está no [README do projeto](../README.md#como-reproduzir); as evidências, em [docs/evidencias](evidencias/README.md).
