# Revisão por pares do design (S14), 24/11/2026

Duas duplas da disciplina revisaram o SenseBridge pelo formulário de design review de PTT, tomando como referência o [projeto de pesquisa](2026-09-17-projeto-de-pesquisa-ptt4.pdf). As duas avaliaram o commit `ceddc98` (tag `sense_bridge-v0.1.1`), em 24/11/2026, e recomendaram **Revisões maiores**, sem nenhum achado Bloqueador. Os pareceres em PDF ficam com os autores, fora do repositório.

- **Dupla 1:** Matheus Naranjo e Pedro Ribeiro. Cinco achados: A1 a A5.
- **Dupla 2:** Filipe Matias e Guilherme Lucas. Três achados: A1 a A3.

As duas duplas reproduziram no host o que não depende do ESP32: `cargo test`, com 83 testes aprovados, e `--analyze`, com 25/25 capturas e 250/250 ciclos no estado esperado. A dupla 2 também reproduziu as médias de latência do relatório a partir dos CSVs. Nenhuma das duas tinha ESP32, então a parte na placa foi avaliada pelas capturas versionadas.

## Achados, consolidados

Os achados das duas duplas se sobrepõem. Abaixo, cada um foi conferido no repositório em 25/11/2026.

| # | Achado | Origem | Severidade | Conferência |
| --- | --- | --- | --- | --- |
| F1 | As capturas não foram feitas com o firmware que o relatório descreve | Dupla 2 A1; dupla 1 A3 | Maior | Procede (seção abaixo) |
| F2 | Uma execução de C5 foi excluída sem critério definido antes da coleta e não está versionada | Dupla 1 A2; dupla 2 A2 | Maior | Procede |
| F3 | A recuperação da fonte externa nunca foi observada na placa | Dupla 1 A1 | Maior | Procede |
| F4 | O vídeo de demonstração não existe; o relatório traz `[inserir link do vídeo]` | Dupla 1 A5 | Maior | Procede |
| F5 | A RAM em execução não é relatada por cenário | Dupla 2 A3 | Menor | Já atendido no relatório entregue (§6.5, Tabela 5); a dupla leu o Markdown de 28/10. O comando pedido foi acrescentado ao console |
| F6 | Números divergentes entre os documentos | Dupla 1 A4 | Menor | Procede em parte (errata abaixo) |

### F1. Firmware das capturas

As 25 capturas oficiais registram `App version: 3725e20` e `Compile time: Oct  3 2026 11:28:54`, mas trazem três ELF diferentes, um por grupo de configuração:

```sh
grep -h 'ELF file SHA256' docs/evidencias/experimentos/*.txt | sort | uniq -c
#  10 1db5893aa  (C3, C4)    10 8b6adf2ea  (C1, C2)    5 08774303e  (C5)
```

Três binários com o mesmo rótulo e o mesmo horário de compilação mostram que o rótulo ficou preso a um build anterior e não identifica o commit. O log identifica: desde `6e03d74` (06/10/2026), o firmware escreve `DHT11: <etapa> (falha N seguida)` depois de cada ciclo com falha local (`firmware/esp32/src/local.rs`, `after_cycle`). As capturas têm 116 ciclos com `local_failure = NoResponse` e nenhuma linha `DHT11:`, embora os avisos `W (...)` do Wi-Fi apareçam. Em `3725e20`, `local.rs` lê o sensor com a `dht-sensor` 0.3 e não registra falha nenhuma.

```sh
grep -c 'DHT11:' docs/evidencias/experimentos/*.txt            # 0 em todas
git show 3725e20:firmware/esp32/Cargo.toml | grep dht            # dht-sensor = "0.3"
git grep -n 'warn!("DHT11' 6e03d74 -- firmware/esp32/src/local.rs
```

Conclusão: o comportamento (C1–C5) e a latência (`read_us`, `fetch_us`, `cycle_us`) foram medidos com o firmware anterior a `6e03d74`, com `dht-sensor`. A Flash e a RAM estática (D8) foram medidas com o firmware de `cff24c3`, já com `dht11-trace` ([ADR-009](decisions/ADR-009.md)). O relatório entregue diz, no §6.1, "firmware no commit 3725e20" e descreve, no §4.5, a leitura pela `dht11-trace`. A biblioteca também mudou entre `3725e20` e `sense_bridge-v0.1.0` (`git diff --stat 3725e20 sense_bridge-v0.1.0 -- sense-bridge/src`, 8 arquivos). A tabela de condições de cada captura está no [README das evidências](../evidencias/README.md).

### F2. Execução de C5 excluída

A [avaliação experimental](2026-10-24-avaliacao-experimental.md) (§2.5) e o relatório entregue (§6.3 e §7.4) dizem que uma execução prevista como repetição 4 teve um `Timeout` externo durante a falha local, foi excluída e "preservada separadamente". O arquivo não está no repositório nem na pasta de trabalho `capturas/` do computador de Marcelo, que vai até 07/10/2026; as capturas oficiais vieram do computador de Ezequias. Nenhum documento define um critério de exclusão anterior à coleta. A dupla 2 observa que um `Timeout` real com a fonte local em falha deveria levar a `Error` (BR-23), o que seria comportamento correto e não motivo de descarte.

### F3. Recuperação da fonte externa

O projeto de pesquisa define C5 com as duas fontes "Recuperado" (§6.3), e BR-30 manda "restabelecer as fontes". Nas cinco capturas de C5, só o DHT11 falha e volta; a fonte externa está disponível em todos os ciclos. Em C3 e C4, o firmware já sobe sem rede (`rede indisponível após 30 s`) e ela não volta durante a captura. A recuperação externa só é verificada sem hardware, no teste [`c5_recovery_returns_to_normal_on_first_cycle_with_both_sources`](../../sense-bridge/tests/scenarios.rs), que sai de `Error`, com a fonte externa em `NoConnectivity`, para `Normal`. A dupla 1 aponta ainda que, com o intervalo padrão de 15 min (BR-05), uma queda da rede leva até 15 min para ser identificada; isso é a consequência aceita em BR-07 ([ADR-005](decisions/ADR-005.md)), mas o relatório não a cita entre as limitações.

### F5. RAM em execução por cenário

O relatório entregue já traz a RAM em execução por cenário, tirada das capturas oficiais (§6.5, Tabela 5). A dupla 2 se baseou no relatório de 28/10 ([`2026-10-28-relatorio-tecnico.md`](2026-10-28-relatorio-tecnico.md), §4.3), que cita só uma captura inicial de C2, fora do conjunto oficial (201.120 B de heap, 11.980 B de pilha). O que faltava era o comando que gera a tabela: desde este registro, `sense_bridge_console --analyze` termina com ela.

```sh
cargo run -q -p sense_bridge_console -- --analyze docs/evidencias/experimentos/*.csv
```

| Cenário | Ciclos | Menor `heap_free` | Menor `heap_min_free` | Menor `stack_min_free` |
| --- | ---: | ---: | ---: | ---: |
| C1 | 50 | 206.508 B | 196.712 B | 12.380 B |
| C2 | 50 | 206.464 B | 199.244 B | 12.380 B |
| C3 | 50 | 208.300 B | 208.240 B | 12.864 B |
| C4 | 50 | 208.300 B | 208.240 B | 13.404 B |
| C5 | 50 | 204.816 B | 196.880 B | 12.380 B |

Os valores coincidem com a Tabela 5 e com os que a dupla 2 calculou. É a RAM do firmware inteiro, sem um firmware-base rodando os mesmos cenários, então não isola o custo da biblioteca.

### F6. Errata

- **`bss` na avaliação experimental, §5.1:** onde está 19.883 bytes para o firmware SenseBridge, o valor medido é 19.891 bytes, como na §5.3, em [`sensebridge-size.txt`](../evidencias/firmware/sensebridge-size.txt) e na RAM estática da própria §5.1 (192.906 + 19.891 = 212.797 B). 19.883 é o `bss` do firmware-base.
- **Contagem de testes:** o relatório entregue cita 62 testes. Em `ceddc98` são 83, e o `verificacao.md` versionado até este registro era de 22/10/2026 (`86055dc`), com 55. O `verificacao.md` foi gerado de novo nesta revisão, com 84: os 83 e o teste da tabela de RAM do console.
- **Duas versões do relatório:** [`2026-10-28-relatorio-tecnico.md`](2026-10-28-relatorio-tecnico.md) é o relatório de 28/10/2026; a versão entregue é a de 03/11/2026 ([PDF](2026-11-03-entrega-final-ptt4.pdf), [DOCX](2026-11-03-entrega-final-ptt4.docx)). `docs/memory/` só recebe acréscimos, então o Markdown não foi reescrito: onde os dois divergem, vale o entregue. A dupla 2 avaliou o Markdown, e é daí que vem F5.

## O que ficou em aberto

As decisões que cabem aos autores (capturar de novo ou declarar a limitação em F1 e F3, o critério de descarte de F2, a leitura do ADR-009 diante do §6.2 e a resposta formal aos revisores) estão em [product.md › Perguntas em aberto](../knowledge/product.md).
