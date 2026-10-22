# Evidências

Dados brutos e registros de verificação que sustentam a [avaliação experimental](../03-avaliacao-experimental.md) e o relatório técnico.

## Conteúdo

```text
docs/evidencias/
├── README.md            # este arquivo
├── verificacao.md       # testes, análise estática, documentação e pacote (gerado)
├── experimentos/        # capturas oficiais de C1–C5 (a versionar)
│   ├── c1-r1.txt        # saída bruta da serial, gravada pelo console
│   ├── c1-r1.csv        # cabeçalho e ciclos da mesma captura
│   ├── ...
│   ├── c5-r5.csv
│   └── excluidas/       # execuções fora do conjunto oficial, com o motivo
└── firmware/            # tamanho do firmware e do firmware-base, D8 (a versionar)
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

- [ ] Versionar as 25 capturas oficiais (`c1-r1` a `c5-r5`, `.txt` e `.csv`) e a execução de C5 excluída do conjunto oficial ([avaliação, §2.5](../03-avaliacao-experimental.md#25-c5--recuperação)).
- [ ] Registrar o commit do firmware e a versão da toolchain `esp` usados nos experimentos e na medição de Flash e RAM.
- [ ] Versionar o código do firmware-base (D8) e a saída do `xtensa-esp32-elf-size` dele e do firmware medido.
- [ ] Decidir se os números de Flash e RAM da avaliação continuam valendo. O firmware atual, com o driver próprio do DHT11, tem `text` 939.016 B, `data` 192.642 B e `bss` 19.883 B (1.151.541 B), contra 930.068 B, 191.962 B e 19.891 B (1.141.921 B) na avaliação.
- [x] Calcular automaticamente a correspondência entre estado esperado e observado (PRD, UC-008): `sense_bridge_console --analyze`, com o cenário e a repetição tirados do nome do arquivo.
