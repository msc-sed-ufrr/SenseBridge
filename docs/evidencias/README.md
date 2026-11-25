# Evidências

Dados brutos e registros de verificação que sustentam a [avaliação experimental](../memory/2026-10-24-avaliacao-experimental.md) e o relatório técnico.

## Conteúdo

```text
docs/evidencias/
├── README.md            # este arquivo
├── verificacao.md       # testes, análise estática, documentação e pacote (gerado)
├── experimentos/        # 25 capturas oficiais de C1–C5 versionadas
│   ├── c1-r1.txt        # saída bruta da serial, gravada pelo console
│   ├── c1-r1.csv        # cabeçalho e ciclos da mesma captura
│   ├── ...
│   ├── c5-r5.csv
│   └── c5-r5.txt
└── firmware/            # baseline equivalente e saídas de tamanho da D8, versionados
```

## Verificação do código

`verificacao.md` registra, para um commit, a formatação, o clippy, os testes, a documentação da API e o pacote da crate. Para gerar de novo:

```sh
scripts/evidencias.sh               # biblioteca e console
. ~/export-esp.sh && scripts/evidencias.sh --firmware   # inclui o firmware e o tamanho dele
```

O tamanho do firmware em `verificacao.md` é o do commit registrado nele, não o da medição D8 (`cff24c3`, em `firmware/`): o firmware mudou depois dela, com o detector de leitura congelada (C6), e os dois números não devem ser comparados.

A CI do GitHub (`.github/workflows/ci.yml`) roda as mesmas verificações de host a cada push e pull request.

## Capturas dos experimentos

Cada repetição é gravada pelo console com o prefixo `c<cenário>-r<repetição>`:

```sh
espflash monitor --non-interactive | cargo run -q -p sense_bridge_console -- --out docs/evidencias/experimentos/c2-r1
```

O procedimento de cada cenário (indução da falha, número de ciclos, `external_refresh_ms`) está no [README do projeto](../../README.md#3-cenários-c1c5-no-esp32) e no PRD (§12).

Para rever uma captura com o resumo por estado e por tipo de falha:

```sh
cargo run -q -p sense_bridge_console -- --no-save < docs/evidencias/experimentos/c2-r1.txt
```

Para conferir todas as capturas com o estado esperado em cada cenário (PRD, UC-008):

```sh
cargo run -q -p sense_bridge_console -- --analyze docs/evidencias/experimentos/*.csv
```

O cenário e a repetição vêm do nome do arquivo. Em C1–C4, todos os ciclos devem estar no estado do cenário. Em C5, cada ciclo deve estar no estado que corresponde à disponibilidade observada das fontes, a captura deve começar com falha e o primeiro ciclo com as duas fontes válidas deve estar em `Normal`, marcado como recuperação. O relatório mostra, por captura e por cenário, os ciclos no estado esperado e as divergências, e o comando termina com código 1 se alguma captura divergir. Ao final, uma tabela traz a RAM em execução de cada cenário, tirada das colunas do CSV: o menor `heap_free`, o menor `heap_min_free` e o menor `stack_min_free` entre todos os ciclos, os mesmos valores da Tabela 5 do relatório entregue. É a RAM do firmware inteiro; o custo próprio da biblioteca fica na diferença de RAM estática contra o firmware-base (D8).

### Condições de cada captura

Tiradas da saída bruta (`.txt`) de cada captura: o ELF e o `external_refresh_ms` vêm do boot, e a recuperação, do `--analyze`. A data de cada execução não consta no repositório; as capturas foram versionadas em 24/10/2026, no commit `14c0300`.

| Arquivo | ELF SHA256 (prefixo no boot) | `external_refresh_ms` | Rede no boot | Observação |
| --- | --- | --- | --- | --- |
| `c1-r1` | `8b6adf2ea` | 900000 | rede disponível após 5705 ms | — |
| `c1-r2` | `8b6adf2ea` | 900000 | rede disponível após 5820 ms | — |
| `c1-r3` | `8b6adf2ea` | 900000 | rede disponível após 5714 ms | — |
| `c1-r4` | `8b6adf2ea` | 900000 | rede disponível após 5692 ms | — |
| `c1-r5` | `8b6adf2ea` | 900000 | rede disponível após 5830 ms | — |
| `c2-r1` | `8b6adf2ea` | 900000 | rede disponível após 5704 ms | — |
| `c2-r2` | `8b6adf2ea` | 900000 | rede disponível após 5715 ms | — |
| `c2-r3` | `8b6adf2ea` | 900000 | rede disponível após 5714 ms | — |
| `c2-r4` | `8b6adf2ea` | 900000 | rede disponível após 5692 ms | — |
| `c2-r5` | `8b6adf2ea` | 900000 | rede disponível após 5702 ms | — |
| `c3-r1` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c3-r2` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c3-r3` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c3-r4` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c3-r5` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c4-r1` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c4-r2` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c4-r3` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c4-r4` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c4-r5` | `1db5893aa` | 5000 | rede indisponível após 30 s | — |
| `c5-r1` | `08774303e` | 5000 | rede disponível após 5714 ms | recuperação no ciclo 3 |
| `c5-r2` | `08774303e` | 5000 | rede disponível após 5703 ms | recuperação no ciclo 3 |
| `c5-r3` | `08774303e` | 5000 | rede disponível após 5715 ms | recuperação no ciclo 4 |
| `c5-r4` | `08774303e` | 5000 | rede disponível após 5704 ms | recuperação no ciclo 3 |
| `c5-r5` | `08774303e` | 5000 | rede disponível após 5715 ms | recuperação no ciclo 3 |

**Firmware das capturas.** As 25 capturas mostram `App version: 3725e20` e o mesmo `Compile time` (`Oct  3 2026 11:28:54`), mas três ELF diferentes, um por grupo de configuração (C1–C2, C3–C4, C5). Três binários diferentes com o mesmo rótulo e o mesmo horário de compilação mostram que o rótulo ficou preso a um build anterior: ele não identifica o commit do firmware. O que identifica é o log: desde `6e03d74` (06/10/2026), o firmware escreve `DHT11: <etapa> (falha N seguida)` a cada leitura local com falha, e as capturas têm 116 ciclos com `NoResponse` e nenhuma dessas linhas, embora os avisos do Wi-Fi apareçam. As capturas foram feitas, portanto, com o firmware anterior a `6e03d74`, que lê o DHT11 com a crate `dht-sensor` 0.3, como em `3725e20`, e não com a `dht11-trace` (D13, [ADR-009](../memory/decisions/ADR-009.md)). A medição de Flash e RAM estática (D8) usou outro firmware, o de `cff24c3`, já com a `dht11-trace`. A análise completa está na [revisão por pares de 25/11/2026](../memory/2026-11-25-revisao-por-pares-s14.md).

## Pendências

- [x] Versionar as 25 capturas oficiais (`c1-r1` a `c5-r5`, `.txt` e `.csv`). O conjunto oficial foi validado automaticamente: **25/25 capturas e 250/250 ciclos no estado esperado**.
- [ ] Registrar o commit exato do firmware usado nas capturas e a data de cada execução. As 25 capturas oficiais registram `App version: 3725e20` (`3725e2016747999582f5d94229695f7d051c341a`), mas esse rótulo não identifica o commit: o log mostra só que o firmware é anterior a `6e03d74`, com `dht-sensor` (ver Condições de cada captura). O commit e as datas precisam ser confirmados por quem executou as capturas.
- [x] Versionar o código do firmware-base equivalente (D8) e as saídas do `xtensa-esp32-elf-size` do baseline e do firmware SenseBridge.
- [x] Registrar a toolchain `esp` da medição de Flash e RAM: feita em `cff24c3c8fa118a3cf23dd2e5601b1ba6727b88b`, com `rustc +esp 1.90.0-nightly (abf50ae2e 2025-09-16) (1.90.0.0)` e `cargo +esp 1.90.0-nightly (840b83a10 2025-07-30) (1.90.0.0)`.
- [x] Repetir a medição D8 com baseline equivalente ao firmware atual. SenseBridge: `text` 931.968 B, `data` 192.906 B, `bss` 19.891 B; baseline: `text` 921.620 B, `data` 179.162 B, `bss` 19.883 B. Diferença: **+24.092 B de Flash (+2,19%)** e **+13.752 B de RAM estática (+6,91%)**.
- [x] Calcular automaticamente a correspondência entre estado esperado e observado (PRD, UC-008): `sense_bridge_console --analyze`, com o cenário e a repetição tirados do nome do arquivo.
