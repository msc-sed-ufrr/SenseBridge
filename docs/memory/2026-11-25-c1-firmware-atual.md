# C1 com o firmware atual, 25/11/2026

Medição complementar, feita em resposta à [revisão por pares](2026-11-25-revisao-por-pares-s14.md) (F1). As capturas oficiais de C1–C5 usaram o firmware anterior a `6e03d74`, que lê o DHT11 com a crate `dht-sensor`. Esta medição repete C1 com o firmware atual, que lê o sensor com a `dht11-trace` ([ADR-009](decisions/ADR-009.md)), para saber se a troca de driver muda a latência. Não substitui o conjunto oficial.

## O que mudou entre o firmware das capturas e a `sense_bridge` 0.1.0

Na biblioteca, nada que afete o comportamento. Fora comentários, documentação e testes, o diff entre `3725e20` e a tag `sense_bridge-v0.1.0` em `sense-bridge/src` acrescenta só os métodos `as_str()` com os nomes estáveis de estados e falhas e o atributo `#![warn(missing_docs)]`:

```sh
git diff -U0 3725e20 sense_bridge-v0.1.0 -- sense-bridge/src
```

Entre `3725e20` e `6e03d74` há um único commit, o próprio `6e03d74`, que não toca a biblioteca; o firmware das capturas oficiais é, portanto, o código de `3725e20`, salvo alterações locais não commitadas. A lógica de ciclo, de validação e de estado avaliada nas capturas oficiais é a da 0.1.0. O que mudou foi o firmware: o driver do DHT11 e o registro da etapa em que a leitura falhou.

A troca de driver veio de melhorias na experiência de desenvolvimento (DX) feitas por um dos autores. O `dht-sensor` só informava tempo esgotado ou checksum inválido, e o firmware precisava de um adaptador de atraso que dependia da ordem interna das chamadas da crate para proteger o quadro da preempção pelo Wi-Fi. Primeiro o firmware passou a registrar a etapa do protocolo em que a leitura falhou (`6e03d74`, 06/10/2026); depois o driver foi reescrito no próprio firmware e extraído para a crate `dht11-trace`, publicada à parte (`c22d3d2` e `07204c9`, 22/10/2026). Os motivos estão no [ADR-009](decisions/ADR-009.md).

## Condições

| Item | Valor |
| --- | --- |
| Firmware | commit `5c653f2`, compilado depois de `cargo clean --release -p esp-idf-sys`; o boot mostra `App version: 5c653f2` nas 5 capturas ([lições](lessons.md)) |
| Driver do DHT11 | `dht11-trace` 0.1 |
| ELF | `4d3126626…`, o mesmo nas 5 capturas |
| Placa | ESP32 com `Chip rev: v3.1`, a placa de Marcelo; as capturas oficiais usaram outra placa, `v1.0` |
| Configuração | a de C1: ciclo de 5 s, tempo limite de 3 s, `external_refresh_ms = 900000`, detector de leitura congelada desligado |
| Protocolo | 5 repetições de 10 ciclos registrados (BR-29), cada uma a partir de um reinício da placa |
| Dados | [`docs/evidencias/complementares/2026-11-25-c1-firmware-atual/`](../evidencias/complementares/2026-11-25-c1-firmware-atual/), o CSV e a saída bruta descaracterizada de cada repetição |

## Resultados

`--analyze` sobre os 5 CSVs: 5/5 capturas aprovadas, 50/50 ciclos em `Normal`.

```sh
cargo run -q -p sense_bridge_console -- --analyze docs/evidencias/complementares/2026-11-25-c1-firmware-atual/*.csv
```

Latência em µs, média e desvio-padrão sobre os ciclos das 5 repetições, comparada com o C1 oficial:

| Medida | C1 oficial (`dht-sensor`, placa `v1.0`) | C1 atual (`dht11-trace`, placa `v3.1`) |
| --- | ---: | ---: |
| `read_us` (50 ciclos) | 22.064 ± 176 | 22.108 ± 198 |
| `cycle_us` sem consulta externa (45 ciclos) | 22.211 ± 182 | 22.300 ± 215 |
| `fetch_us` (5 consultas, uma por repetição) | 1.097.987 ± 105.681 | 905.471 ± 255.130 |
| Trabalho próprio, `cycle_us − read_us − fetch_us` (50 ciclos) | 145 ± 14 | 190 ± 19 |

RAM em execução, pelo `--analyze`: menor `heap_free` 206.356 B, menor `heap_min_free` 201.024 B e menor `stack_min_free` 12.212 B, contra 206.508 B, 196.712 B e 12.380 B no C1 oficial.

## Leitura

- **A troca de driver não muda a latência da leitura.** O `read_us` médio difere em 44 µs (0,2%), menos que um desvio-padrão. A leitura continua dominada pelo protocolo do sensor, cerca de 22 ms, e as médias de `read_us` do relatório valem para o firmware atual.
- **O trabalho próprio por ciclo subiu cerca de 45 µs**, de 0,145 para 0,190 ms, e continua abaixo de 1% do ciclo sem consulta externa. Placa e firmware mudaram ao mesmo tempo, então esta medição não separa a causa.
- **`fetch_us` depende da rede e do serviço**, como o relatório já aponta (§7.4); as duas redes são diferentes, e a comparação não diz nada sobre a biblioteca.

## Limitações

Só C1 foi repetido; C2–C5 continuam com o firmware anterior. A ligação serial desta placa perdeu 5 linhas de ciclo, que chegaram corrompidas e foram descartadas pelo console: o ciclo 9 em r2, os ciclos 4 e 10 em r3 e o ciclo 3 em r4 e em r5. Os ciclos aconteceram (a numeração segue), só não foram gravados; cada CSV tem 10 ciclos registrados, até o ciclo 10 ou 11, e as estatísticas acima usam esses 50. As capturas oficiais não têm nenhuma linha corrompida. As duas medições usaram placas e redes Wi-Fi diferentes. A saída bruta da serial (`.txt`) está versionada descaracterizada: o SSID virou `<rede-A>`, o MAC da placa virou `<mac-placa>`, os MACs dos pontos de acesso viraram `<bssid-N>`, os IPs viraram `<ip-N>` e o tamanho da senha no aviso do Wi-Fi virou `<omitido>`. Cada valor distinto tem o seu marcador, nas mesmas pastas do dia; o resto das linhas, inclusive as corrompidas, ficou como chegou. O CSV não tem dados de rede e não foi alterado.
