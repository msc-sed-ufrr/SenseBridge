# Latência de validação e integração — 22/11/2026

A BR-33 ([product.md](../knowledge/product.md)) pede a latência de quatro operações: leitura local, obtenção externa, validação e integração, e ciclo completo. O CSV do firmware tem três colunas de tempo, `read_us`, `fetch_us` e `cycle_us`. Esta nota mostra que a quarta operação sai das três e registra o seu valor nas capturas oficiais de C1–C5. Ela não altera a [avaliação experimental](2026-10-24-avaliacao-experimental.md) nem o relatório entregue.

## Método

No firmware (`firmware/esp32/src/main.rs`), `cycle_us` mede a chamada inteira a `SenseBridge::cycle`. As fontes são envolvidas por `Timed` (`firmware/esp32/src/timed.rs`), e a biblioteca as chama de dentro do ciclo. Por isso `read_us` e `fetch_us` estão contidos em `cycle_us`. O restante é o que a biblioteca faz fora das fontes: validação, decisão de reuso da leitura externa, estado, transição e montagem do `Record`.

```text
validação e integração = cycle_us − read_us − fetch_us    (fetch_us = 0 quando a leitura externa é reutilizada)
```

O resultado é um limite superior. Ele inclui as duas chamadas a `Instant::now` e `elapsed` de cada `Timed`, que pertencem à instrumentação, não à biblioteca.

Comando, rodado na raiz do repositório sobre os arquivos de `docs/evidencias/experimentos/` (troque `c1` pelo cenário):

```sh
awk -F, 'FNR==1{for(i=1;i<=NF;i++)h[$i]=i;next}
  {r=$h["cycle_us"]-$h["read_us"]-($h["fetch_us"]==""?0:$h["fetch_us"]); n++; s+=r; ss+=r*r
   if(n==1||r<mn)mn=r; if(n==1||r>mx)mx=r}
  END{m=s/n; printf "n=%d média=%.2f DP=%.2f mín=%d máx=%d\n",n,m,sqrt(ss/n-m*m),mn,mx}' \
  docs/evidencias/experimentos/c1-r*.csv
```

## Resultado

São 50 ciclos por cenário, de 5 repetições com 10 ciclos cada. O desvio-padrão é populacional, como na avaliação experimental.

| Cenário | Média ± DP | Mínimo | Máximo |
| --- | ---: | ---: | ---: |
| C1 | 144,82 ± 13,61 µs | 131 µs | 180 µs |
| C2 | 121,96 ± 12,55 µs | 99 µs | 155 µs |
| C3 | 144,30 ± 5,89 µs | 119 µs | 151 µs |
| C4 | 116,90 ± 3,40 µs | 99 µs | 123 µs |
| C5 | 202,20 ± 64,65 µs | 122 µs | 593 µs |
| C1–C5 | 146,04 ± 42,79 µs | 99 µs | 593 µs |

Validação e integração custam de 0,1 a 0,2 ms por ciclo, em média menos de 1% do ciclo sem consulta externa (18 a 22 ms, dominado pela leitura do DHT11). Os cenários com falha local (C2, C4) ficam de 20 a 30 µs abaixo dos de leitura local válida (C1, C3). O máximo de C5, 593 µs, é um único ciclo, o 6 de `c5-r3`, em `Normal` e sem transição. Todos os outros ciclos de C5 ficam em 250 µs ou menos, e a causa desse ciclo não foi investigada.
