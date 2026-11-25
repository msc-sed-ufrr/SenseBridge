# Adendo ao relatório técnico do SenseBridge

**Resposta à revisão por pares (S14).** 25/11/2026, aprovado pelos dois autores para a entrega de 26/11/2026. Versão do repositório: tag `ptt4-entrega-2026-11-26`.

Este adendo complementa o [relatório técnico entregue em 03/11/2026](2026-11-03-entrega-final-ptt4.pdf) e não o substitui: o relatório entregue continua valendo onde este documento não diz o contrário. Ele responde aos achados das duas duplas que revisaram o projeto em 24/11/2026, sobre a tag `sense_bridge-v0.1.1`, e que recomendaram "Revisões maiores", sem achado Bloqueador. As referências a seções (§), tabelas e quadros são as do relatório entregue. A conferência de cada achado está no [registro da revisão](2026-11-25-revisao-por-pares-s14.md); os dados e os comandos, em [`docs/evidencias/`](../evidencias/README.md).

## 1. Resumo das respostas

| Achado | Severidade | Resposta | Onde verificar |
| --- | --- | --- | --- |
| F1. As capturas de C1–C5 não foram feitas com o firmware descrito no relatório | Maior | Identificada a origem de cada conjunto de dados; a lógica da biblioteca nas capturas é a da 0.1.0; C1 repetido com o firmware da tag `sense_bridge-v0.1.1`, com o mesmo tempo de leitura (seção 2) | [Condições de cada captura](../evidencias/README.md), [C1 com o firmware atual](2026-11-25-c1-firmware-atual.md) |
| F2. Uma execução de C5 foi excluída sem critério prévio e não está versionada | Maior | O arquivo se perdeu. Um novo C5 foi executado com critério de descarte definido antes da coleta e com a tentativa interrompida versionada (seção 3) | [C5 com a fonte externa](2026-11-25-c5-externa.md) |
| F3. A recuperação da fonte externa nunca foi observada na placa | Maior | C5 executado com a queda e a volta da rede: 5/5 recuperações no primeiro ciclo depois de a placa obter IP (seção 4) | [C5 com a fonte externa](2026-11-25-c5-externa.md) |
| F4. Vídeo de demonstração ausente | Maior | O vídeo de demonstração acompanha a entrega de 26/11/2026 como anexo | Anexo da entrega |
| F5. RAM em execução não relatada por cenário | Menor | Já relatada na Tabela 5; o comando que a gera foi acrescentado ao console (seção 5) | `sense_bridge_console --analyze` |
| F6. Números divergentes entre documentos | Menor | Errata (seção 6) | Seção 6 |
| A3 da dupla 1. A `dht11-trace` como desvio do §6.2 do projeto | Menor | Posição dos autores na seção 2 | [ADR-009](decisions/ADR-009.md) |

## 2. Origem de cada conjunto de dados (F1)

O §6.1 diz que o firmware das capturas é o do commit `3725e20`, "registrado em cada captura", e o §4.5 descreve o firmware lendo o DHT11 com a crate `dht11-trace`. As duas afirmações não valem para o mesmo firmware. O rótulo `App version` do boot não identifica o commit: as 25 capturas oficiais mostram `3725e20` e o mesmo horário de compilação em três binários diferentes, um por configuração do `cfg.toml`. O que identifica o firmware é o log. Desde o commit `6e03d74`, de 06/10/2026, o firmware registra a etapa do protocolo em que cada leitura do DHT11 falhou, e as capturas oficiais têm 116 ciclos com falha local sem nenhum desses registros. Como `6e03d74` é o único commit depois de `3725e20` nessa linha, o firmware das capturas é o código de `3725e20`, salvo alterações locais não commitadas.

| Conjunto de dados | Firmware | Leitura do DHT11 | Placa (revisão do chip) |
| --- | --- | --- | --- |
| C1–C5 oficiais (Tabelas 1, 2, 3 e 5) | código de `3725e20` | crate `dht-sensor` 0.3 | ESP32 `v1.0` |
| Flash e RAM estática, D8 (Tabela 4) | `cff24c3` | crate `dht11-trace` 0.1 | (medição do binário) |
| C1 complementar, 25/11/2026 | `5c653f2`, igual à tag `sense_bridge-v0.1.1` | crate `dht11-trace` 0.1 | ESP32 `v3.1` |
| C5 com a fonte externa, 25/11/2026 | `5c653f2`, igual à tag `sense_bridge-v0.1.1` | crate `dht11-trace` 0.1 | ESP32 `v3.1` |

A troca do driver veio de melhorias na experiência de desenvolvimento feitas por um dos autores: o `dht-sensor` só informava tempo esgotado ou checksum inválido, e o firmware dependia de um adaptador que contava com a ordem interna das chamadas da crate para proteger a leitura da preempção pelo Wi-Fi. O driver foi reescrito no firmware e extraído para a crate `dht11-trace`, adotada em 22/10/2026 ([ADR-009](decisions/ADR-009.md)).

**Sobre o §6.2 do projeto de pesquisa.** O projeto deixa fora do escopo a implementação do protocolo de baixo nível do DHT11, e a dupla 1 pede que a `dht11-trace` seja registrada como desvio. A posição dos autores é que o escopo da biblioteca foi mantido: a `sense_bridge` não implementa o protocolo e conhece o sensor só pela trait `LocalSource`. A `dht11-trace` é uma crate separada, usada pelo firmware de referência para ler o sensor, e qualquer outro driver que implemente a trait serve à biblioteca, como mostram as capturas oficiais, feitas com a `dht-sensor`, de terceiros. Fica registrado, porém, que o firmware de referência deixou de usar apenas bibliotecas existentes para o sensor, como o §6.2 previa.

**O que isso muda nos resultados.**

- **Comportamento (Tabelas 1 e 2).** Fora comentários, documentação e testes, a biblioteca entre `3725e20` e a tag `sense_bridge-v0.1.0` ganhou só os métodos `as_str()`, com os nomes estáveis de estados e falhas, e um atributo de lint (`git diff 3725e20 sense_bridge-v0.1.0 -- sense-bridge/src`). A lógica de ciclo, de validação e de estado avaliada nas capturas é a da 0.1.0, e os 250 ciclos valem para ela.
- **Latência (Tabela 3).** C1 foi repetido com o firmware da tag `sense_bridge-v0.1.1`, que lê o sensor com a `dht11-trace`: `read_us` de 22,11 ± 0,20 ms, contra 22,06 ± 0,18 ms nas capturas oficiais. A troca de driver não muda o tempo de leitura. O trabalho próprio da biblioteca por ciclo (`cycle_us − read_us − fetch_us`) passou de 0,145 para 0,190 ms, ainda abaixo de 1% do ciclo; placa e firmware mudaram juntos, e a medição não separa a causa.
- **Memória (Tabela 4).** O firmware SenseBridge e o firmware-base foram medidos com o mesmo driver, então a diferença continua valendo.

## 3. Execução excluída de C5 (F2)

O §6.3 e o §7.4 dizem que uma execução de C5, prevista como repetição 4, teve um `Timeout` externo durante a falha local, foi excluída e preservada à parte. O arquivo dessa execução se perdeu e não pode ser versionado. Em outubro, C5 teve, portanto, 6 execuções: as 5 da Tabela 2 e a excluída, que ninguém pode mais auditar. Não havia um critério de exclusão definido antes da coleta. Pela regra de estados (BR-23), um `Timeout` real com a fonte local em falha leva a `Error`, que seria o comportamento esperado, e não um motivo de exclusão.

O C5 com a fonte externa, executado em 25/11/2026, seguiu o procedimento que faltou: o critério de descarte foi definido antes da coleta (a repetição é descartada e refeita se a placa encontrar a rede no boot, porque não começaria com falha), e nenhuma repetição caiu nele. Antes das 5 repetições, uma tentativa foi interrompida durante a coleta, porque a rede escolhida não chegou a sumir para a placa. Ela está versionada inteira, com o motivo, e o total de execuções é 6. A placa estava associada a um repetidor da rede, e só o roteador principal foi desligado; a falha foi de indução, não do firmware. Esse procedimento passou a ser regra para os próximos experimentos (BR-37, [ADR-011](decisions/ADR-011.md)).

## 4. Recuperação da fonte externa na placa (F3)

No Quadro 11, C5 recupera só o DHT11; a fonte externa está disponível em todos os ciclos, e a recuperação externa só era verificada pelos testes sem hardware. Em 25/11/2026, C5 foi executado com a falha na rede: o Wi-Fi do roteador desligado antes do reinício da placa e religado depois do ciclo 2 (em r4, depois do ciclo 3, por uma linha perdida na serial), com o DHT11 disponível e `external_refresh_ms = 5000` (D10).

| Repetição | Recuperação no ciclo | Da placa obter IP ao ciclo de recuperação |
| --- | ---: | ---: |
| r1 | 6 | 2,3 s |
| r2 | 6 | 4,6 s |
| r3 | 6 | 4,5 s |
| r4 | 7 | 2,3 s |
| r5 | 6 | 4,1 s |

As 5 capturas foram aprovadas pelo `--analyze`, com 50/50 ciclos no estado esperado. A recuperação aconteceu sempre no primeiro ciclo depois de a rede voltar, menos de um ciclo (5 s) depois de a placa obter IP. A conclusão do §7.2, "recuperação no primeiro ciclo com as duas fontes válidas", vale agora para as duas fontes, na placa. A recuperação cai no ciclo 6 ou 7, e não no 3, porque a rede só fica disponível para a placa de 15 a 18 s depois de religada.

O tempo para *identificar* uma queda da rede durante a operação normal não foi medido, porque o protocolo (BR-30) começa com a falha. Ele decorre da regra BR-07: com `external_refresh_ms = 5000`, a queda é identificada na consulta seguinte, até um ciclo depois; com o intervalo padrão de 15 min, até 15 min depois.

## 5. RAM em execução por cenário (F5)

A Tabela 5 do relatório entregue já traz a RAM em execução por cenário, tirada das 25 capturas oficiais. A partir deste adendo, o comando abaixo reproduz a tabela inteira:

```sh
cargo run -q -p sense_bridge_console -- --analyze docs/evidencias/experimentos/*.csv
```

## 6. Errata

| Onde | Diz | Deve dizer |
| --- | --- | --- |
| §6.1, Software | "firmware no commit 3725e20, registrado em cada captura" | firmware com o código de `3725e20`, que lê o DHT11 com a crate `dht-sensor` 0.3; o rótulo do boot não identifica o commit (seção 2) |
| Fonte da Tabela 4: avaliação experimental, §5.1 | `bss` do firmware SenseBridge: 19.883 bytes | 19.891 bytes, como na própria Tabela 4; 19.883 é o `bss` do firmware-base |
| `docs/evidencias/verificacao.md` | 55 testes, gerado em 22/10/2026 | gerado de novo em 25/11/2026, com 84 testes: os 62 do §5.1 e os acrescentados depois da entrega (detector de leitura congelada, C6 e tabela de RAM do console) |

Os caminhos que o relatório cita como fonte foram reorganizados em 22/11/2026 e não abrem mais na branch `main`. Todos existem na tag [`sense_bridge-v0.1.0`](https://github.com/msc-sed-ufrr/SenseBridge/tree/sense_bridge-v0.1.0), a versão avaliada pelo relatório, e o conteúdo atual está em:

| Caminho citado | Hoje |
| --- | --- |
| `docs/01-prd.md` | [`docs/knowledge/product.md`](../knowledge/product.md) |
| `docs/02-arch.md` | [`docs/knowledge/architecture.md`](../knowledge/architecture.md) |
| `docs/03-avaliacao-experimental.md` | [`docs/memory/2026-10-24-avaliacao-experimental.md`](2026-10-24-avaliacao-experimental.md) |
| `docs/04-relatorio-tecnico.md` | [`docs/memory/2026-10-28-relatorio-tecnico.md`](2026-10-28-relatorio-tecnico.md), uma versão anterior ao relatório entregue; onde os dois divergem, vale o entregue |
| `docs/05-roteiro-video.md` | não é mais versionado; está na tag `sense_bridge-v0.1.0` |

## 7. Limitações acrescentadas ao §7.4

- C2–C5 não foram repetidos com o firmware atual; o tempo de leitura das capturas oficiais é o do driver `dht-sensor`, que a repetição de C1 mostrou equivalente.
- As medições complementares usaram outra placa (`v3.1`) e outra rede Wi-Fi que as capturas oficiais.
- A ligação serial da placa das medições complementares perdeu 6 linhas de ciclo nas 10 capturas de C1 e de C5, que chegaram corrompidas e foram descartadas pelo console; os ciclos aconteceram, só não foram gravados. As capturas oficiais não têm nenhuma linha perdida.
- O arquivo da execução de C5 excluída em outubro se perdeu (seção 3).
