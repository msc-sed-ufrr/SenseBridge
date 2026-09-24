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

Preencha `wifi_ssid` e `wifi_password`. O `cfg.toml` é ignorado pelo git; as credenciais nunca vão para o repositório. O build valida os valores: um ciclo abaixo de 1000 ms (intervalo mínimo do DHT11) ou um tempo limite maior que o ciclo interrompem a compilação com uma mensagem.

| Chave | Padrão | Decisão no PRD |
| --- | --- | --- |
| `latitude`, `longitude` | 2.82, −60.67 (Boa Vista/RR) | D6, em aberto |
| `cycle_interval_ms` | 5000 | D5b, em aberto |
| `http_timeout_ms` | 3000 | D5c, em aberto |
| `external_refresh_ms` | 900000 (15 min) | D5a, fechada |
| `dht11_gpio` | 4 | — |

Para observar falha e recuperação da fonte externa sem esperar 15 minutos, reduza `external_refresh_ms` (por exemplo, para 30000). Uma falha de rede durante a reutilização só aparece na consulta seguinte (PRD, D10).

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

As colunas do protocolo experimental (cenário, repetição, estado esperado) dependem de D7 e ainda não fazem parte desta saída.

## Mapeamento de falhas

| Situação | Falha registrada |
| --- | --- |
| DHT11 não responde ou pino com erro | `NoResponse` |
| DHT11 com checksum inválido | `Checksum` |
| Leitura fora da faixa ou não numérica | `Invalid(OutOfRange)`, `Invalid(NotANumber)` |
| Wi-Fi sem conexão ou sem IP; DNS ou TCP falhando | `NoConnectivity` |
| Consulta que excede `http_timeout_ms` | `Timeout` |
| Status HTTP diferente de 200 | `ServiceError` |
| Corpo incompleto ou sem temperatura e umidade | `MalformedResponse` |

A cada queda do Wi-Fi, ou tentativa de associação frustrada, o firmware inicia outra em segundo plano, sem depender do ciclo nem bloqueá-lo. A fonte externa volta no primeiro ciclo após a reconexão.
