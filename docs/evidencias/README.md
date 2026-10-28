# Evidências

Dados brutos e registros de verificação que sustentam a [avaliação experimental](../03-avaliacao-experimental.md) e o relatório técnico.

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

O cenário e a repetição vêm do nome do arquivo. Em C1–C4, todos os ciclos devem estar no estado do cenário. Em C5, cada ciclo deve estar no estado que corresponde à disponibilidade observada das fontes, a captura deve começar com falha e o primeiro ciclo com as duas fontes válidas deve estar em `Normal`, marcado como recuperação. O relatório mostra, por captura e por cenário, os ciclos no estado esperado e as divergências, e o comando termina com código 1 se alguma captura divergir.

Para cada captura, registrar nesta tabela as condições da execução:

| Arquivo | Data | Commit do firmware | `external_refresh_ms` | Observação |
| --- | --- | --- | --- | --- |
| `c1-r1` | | | 900000 | |

## Pendências

- [x] Versionar as 25 capturas oficiais (`c1-r1` a `c5-r5`, `.txt` e `.csv`). O conjunto oficial foi validado automaticamente: **25/25 capturas e 250/250 ciclos no estado esperado**.
- [x] Registrar o commit do firmware e a versão da toolchain `esp` usados nos experimentos e na medição de Flash e RAM. As 25 capturas oficiais registram `App version: 3725e20` (`3725e2016747999582f5d94229695f7d051c341a`). A medição final D8 foi feita na árvore local em `cff24c3c8fa118a3cf23dd2e5601b1ba6727b88b`, com `rustc +esp 1.90.0-nightly (abf50ae2e 2025-09-16) (1.90.0.0)` e `cargo +esp 1.90.0-nightly (840b83a10 2025-07-30) (1.90.0.0)`.
- [x] Versionar o código do firmware-base equivalente (D8) e as saídas do `xtensa-esp32-elf-size` do baseline e do firmware SenseBridge.
- [x] Repetir a medição D8 com baseline equivalente ao firmware atual. SenseBridge: `text` 931.968 B, `data` 192.906 B, `bss` 19.891 B; baseline: `text` 921.620 B, `data` 179.162 B, `bss` 19.883 B. Diferença: **+24.092 B de Flash (+2,19%)** e **+13.752 B de RAM estática (+6,91%)**.
- [x] Calcular automaticamente a correspondência entre estado esperado e observado (PRD, UC-008): `sense_bridge_console --analyze`, com o cenário e a repetição tirados do nome do arquivo.
