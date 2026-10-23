# Firmware de referência — ESP32

Caso de uso da biblioteca [`sense_bridge`](../../sense-bridge) no ESP32 clássico (ESP32-WROOM-32), com ESP-IDF e `std` (PRD, D12). O DHT11 é a fonte local, a Open-Meteo é a fonte externa, e cada ciclo emite uma linha CSV pela serial.

Este diretório fica fora do workspace da raiz: usa a toolchain Xtensa e compila apenas para o ESP32.

## Hardware

| DHT11 | ESP32 |
| --- | --- |
| VCC | 3V3 |
| DATA | GPIO4 (D4), ou o definido em `dht11_gpio` |
| GND | GND |

O firmware ativa o pull-up interno do GPIO de dados. Com o DHT11 de 4 pinos (sem placa), coloque também um resistor de 4,7–10 kΩ entre DATA e 3V3; os módulos de 3 pinos já trazem o resistor.

O GPIO de dados é configurável (ver [Configuração](#configuração)). O build aceita os GPIOs bidirecionais fora da Flash (6–11) e da UART0 (1 e 3, que levam o CSV): 0, 2, 4, 5, 12–19, 21–23, 25–27, 32 e 33. Os de 34 a 39 são só de entrada e não servem ao DHT11.

Os pinos de strapping (0, 2, 5, 12 e 15) funcionam, mas o build avisa: o nível no reset escolhe o modo de boot ou a tensão da Flash, e o pull-up do DHT11 pode impedir a gravação ou o boot. No GPIO2 (D2), por exemplo, o `espflash` não consegue conectar enquanto o DATA está ligado; desconecte-o para gravar e religue-o depois. No DevKit, o GPIO2 também aciona o LED azul da placa.

### Alimentação por GPIO (opcional)

Um DHT11 travado só volta a responder depois de perder a alimentação. Para que o firmware possa reiniciá-lo, ligue o VCC do sensor a um GPIO em vez do 3V3 e informe-o em `dht11_power_gpio`. A cada 3 falhas seguidas de leitura, o firmware desliga o sensor por 1 s, com o DATA também em nível baixo, para que o pull-up não o mantenha alimentado; depois o religa e espera mais 1 s. O reinício acontece depois da linha CSV, fora da janela medida, e cabe na espera entre ciclos; com `cycle_interval_ms` abaixo de cerca de 2100 ms, o ciclo seguinte começa atrasado. O DHT11 consome até 2,5 mA, bem abaixo do que um GPIO fornece. Valem os mesmos GPIOs do DATA, e o build avisa também sobre os pinos de strapping.

A rede Wi-Fi precisa ser de 2,4 GHz.

## Ambiente

Uma vez por máquina:

```sh
cargo install espup espflash ldproxy --locked
espup install --targets esp32
```

Em cada terminal, antes de compilar:

```sh
. ~/export-esp.sh
```

O primeiro build baixa o ESP-IDF v5.5.5 e as ferramentas para `~/.espressif`, o que leva alguns minutos.

## Configuração

```sh
cp cfg.toml.example cfg.toml
```

Preencha `wifi_ssid` e `wifi_password`. O `cfg.toml` é ignorado pelo git; as credenciais nunca vão para o repositório. O build valida os valores: um ciclo abaixo de 1000 ms (intervalo mínimo do DHT11) ou um tempo limite igual ou maior que o ciclo interrompem a compilação com uma mensagem.

| Chave | Padrão | Decisão no PRD |
| --- | --- | --- |
| `latitude`, `longitude` | 2.82, −60.67 (Boa Vista/RR) | D6, fechada |
| `cycle_interval_ms` | 5000 | D5b, fechada |
| `http_timeout_ms` | 3000 | D5c, fechada |
| `external_refresh_ms` | 900000 (15 min) | D5a, fechada |
| `dht11_gpio` | 4 | — |
| `dht11_power_gpio` | ausente (VCC no 3V3) | — |

Para observar falha e recuperação da fonte externa sem esperar 15 minutos, reduza `external_refresh_ms` para 5000, como nos experimentos C3–C5 (PRD, D10): com o ciclo de 5 s, a fonte externa passa a ser consultada aproximadamente a cada ciclo. Uma falha de rede durante a reutilização só aparece na consulta seguinte.

## Compilar e gravar

```sh
cargo build --release
espflash flash --monitor target/xtensa-esp32-espidf/release/sense_bridge_esp32
```

Ou apenas `cargo run --release`, que grava e abre o monitor serial.

## Saída

Depois dos logs de inicialização, a serial mostra o cabeçalho e uma linha por ciclo:

| Coluna | Conteúdo |
| --- | --- |
| `cycle`, `timestamp_ms` | Identificador do ciclo e instante, em ms desde o primeiro ciclo |
| `state` | `Normal`, `DegradedLocal`, `DegradedExternal` ou `Error` |
| `local_temperature_c`, `local_humidity_pct`, `local_failure` | Leitura do DHT11 ou o tipo de falha |
| `external_temperature_c`, `external_humidity_pct`, `external_age_ms`, `external_failure` | Leitura da Open-Meteo, sua idade, ou o tipo de falha |
| `transition`, `recovery` | Transição em relação ao ciclo anterior e se ela é uma recuperação |
| `read_us`, `fetch_us`, `cycle_us` | Duração da leitura local, da consulta externa (vazia quando a leitura é reutilizada) e do ciclo inteiro; validação e integração correspondem a `cycle_us - read_us - fetch_us` |
| `heap_free`, `heap_min_free`, `stack_min_free` | Heap livre, mínimo de heap livre desde o boot e folga mínima da pilha da tarefa principal, em bytes |

As linhas de dados começam por um número, e as de log por `I (`, `W (` ou `E (`. Para separar os dados de uma captura:

```sh
grep -E '^(cycle,|[0-9]+,)' captura.txt > ciclos.csv
```

As colunas do protocolo experimental (cenário, repetição, estado esperado) ainda não fazem parte desta saída.

### Console

Para acompanhar os ciclos ao vivo, o console do host ([`tools/console`](../../tools/console)) lê a saída do monitor e mostra cada ciclo em uma linha. O estado aparece em cor, com as transições destacadas e um resumo a cada 12 ciclos (contagem por estado, falhas por tipo, heap, pilha e ciclo mais longo). Da raiz do repositório:

```sh
espflash monitor --non-interactive | cargo run -q -p sense_bridge_console
```

```text
    #  tempo     estado            local         externo                  ciclo
    0  00:00:00  Normal            27.0°C 41%    29.1°C 76%  ↻ 1.46 s    1.48 s
    1  00:00:05  DegradedLocal     NoResponse    29.1°C 76%  (4 s)        22 ms
       ↳ Normal → DegradedLocal
```

`↻` marca um ciclo que consultou a Open-Meteo, com a duração da consulta; nos demais aparece a idade da leitura reutilizada. Logs de aviso e de erro do ESP-IDF e os logs do próprio firmware continuam visíveis; `--logs` mostra também os demais.

Por padrão, a captura é gravada em `capturas/<data UTC>.txt` (a saída bruta, compatível com o `grep` acima) e `.csv` (cabeçalho e ciclos), ignorados pelo git. `--out <prefixo>` escolhe o nome, por exemplo `--out capturas/c2-r1`, e `--no-save` desliga a gravação. Uma captura antiga pode ser revista com `cargo run -q -p sense_bridge_console -- --no-save < captura.txt`.

## Mapeamento de falhas

| Situação | Falha registrada |
| --- | --- |
| DHT11 não responde, quadro interrompido ou pino com erro | `NoResponse` |
| DHT11 com checksum inválido | `Checksum` |
| Leitura fora da faixa ou não numérica | `Invalid(OutOfRange)`, `Invalid(NotANumber)` |
| Wi-Fi sem conexão ou sem IP; DNS ou TCP falhando | `NoConnectivity` |
| Consulta que excede `http_timeout_ms` | `Timeout` |
| Status HTTP diferente de 200 | `ServiceError` |
| Corpo incompleto ou sem temperatura e umidade | `MalformedResponse` |

A cada queda do Wi-Fi, ou tentativa de associação frustrada, o firmware inicia outra em segundo plano, sem depender do ciclo nem bloqueá-lo. A fonte externa volta no primeiro ciclo após a reconexão.

### Diagnóstico do DHT11

A coluna `local_failure` mantém a taxonomia da biblioteca. A etapa do protocolo em que a leitura falhou vai para um log de aviso, emitido logo depois da linha CSV do ciclo e mostrado pelo console:

```text
W (40953) sense_bridge_esp32::local: DHT11: sem resposta ao pulso de início (sensor sem alimentação, DATA desconectado ou sensor travado) (falha 3 seguida)
```

| Log | Etapa | Causa provável | `local_failure` |
| --- | --- | --- | --- |
| linha DATA presa em nível baixo | Antes do pulso de início | DATA em curto com o GND ou sensor travado | `NoResponse` |
| sem resposta ao pulso de início | A linha seguia em nível alto 48 µs depois da liberação | Sensor sem alimentação, DATA desconectado ou sensor travado | `NoResponse` |
| resposta ao pulso de início incompleta | A resposta de 80 µs + 80 µs não terminou | Contato intermitente | `NoResponse` |
| quadro interrompido após N de 40 bits | Algum dos 40 bits não chegou | Contato intermitente ou interferência | `NoResponse` |
| checksum inválido no quadro [..] | Quadro completo, com os 5 bytes recebidos | Interferência ou bit lido errado | `Checksum` |
| erro no GPIO | Driver do pino | — | `NoResponse` |

O protocolo é lido pela crate [`dht11-trace`](https://crates.io/crates/dht11-trace), publicada à parte no crates.io, que distingue essas etapas; `src/local.rs` adapta o resultado à biblioteca e escreve as mensagens acima. As durações dos pulsos vêm do contador de ciclos da CPU, com as interrupções mascaradas durante o quadro por uma seção crítica de ISR.
