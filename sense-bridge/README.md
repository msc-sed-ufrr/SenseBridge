# sense_bridge

Integração resiliente de dados ambientais de uma **fonte local** (por exemplo, o sensor DHT11) e de uma **fonte externa** (por exemplo, a API Open-Meteo) em sistemas embarcados.

A cada ciclo, a biblioteca consulta as duas fontes, valida as leituras e entrega um registro com:

- a leitura validada ou o motivo da falha de cada fonte, sempre identificando a origem;
- o estado de operação resultante;
- a transição em relação ao ciclo anterior, indicando quando houve recuperação.

Valores reprovados na validação nunca aparecem como válidos. A queda de uma fonte não interrompe a entrega de dados da outra.

A crate é `no_std`, não aloca memória, não tem dependências e não depende de hardware: o acesso aos periféricos fica nas implementações das traits de fonte.

## Estados de operação

| Fonte local | Fonte externa | Estado |
| --- | --- | --- |
| Disponível | Disponível | `Normal` |
| Indisponível | Disponível | `DegradedLocal` |
| Disponível | Indisponível | `DegradedExternal` |
| Indisponível | Indisponível | `Error` |

Uma fonte é considerada disponível quando fornece, no ciclo, uma leitura aprovada na validação.

## Uso

Implemente `LocalSource` e `ExternalSource` para as fontes da sua plataforma e chame `cycle` com o instante de um relógio monotônico:

```rust
use sense_bridge::{
    Config, ExternalFailure, ExternalReading, ExternalSource, LocalFailure, LocalReading,
    LocalSource, OperationState, SenseBridge,
};

struct Dht11 {/* driver do sensor */}

impl LocalSource for Dht11 {
    fn read(&mut self) -> Result<LocalReading, LocalFailure> {
        // Ler o sensor e converter o erro do driver em `LocalFailure`.
        Ok(LocalReading::new(25.0, 60.0))
    }
}

struct OpenMeteo {/* cliente HTTP */}

impl ExternalSource for OpenMeteo {
    fn fetch(&mut self) -> Result<ExternalReading, ExternalFailure> {
        // Consultar a API aplicando o tempo limite.
        Err(ExternalFailure::Timeout)
    }
}

let mut bridge = SenseBridge::new(Dht11 {}, OpenMeteo {}, Config::default());

let record = bridge.cycle(0);
assert_eq!(record.state(), OperationState::DegradedExternal);
assert_eq!(record.local, Ok(LocalReading::new(25.0, 60.0)));
assert_eq!(record.external, Err(ExternalFailure::Timeout));
```

`Config` define as faixas aceitas para cada fonte e o intervalo entre consultas externas. Dentro desse intervalo, a última leitura externa válida é reutilizada e o registro informa a sua idade.

Para serializar registros, `OperationState`, `LocalFailure` e `ExternalFailure` têm nomes estáveis em `as_str()`, como `DegradedLocal`, `NoResponse` e `Invalid(OutOfRange)`. Ao contrário da saída de `Debug`, esses nomes fazem parte da API.

## Open-Meteo

O módulo `open_meteo` monta o caminho da consulta e interpreta a resposta da API, sem alocar memória. A implementação de `ExternalSource` fica apenas com o transporte: envia a requisição, aplica o tempo limite e entrega o status e o corpo a `interpret`.

```rust
use sense_bridge::{ExternalFailure, ExternalReading, ExternalSource, open_meteo};

struct OpenMeteo {/* cliente HTTP */}

impl ExternalSource for OpenMeteo {
    fn fetch(&mut self) -> Result<ExternalReading, ExternalFailure> {
        // GET em `open_meteo::HOST`, no caminho escrito por `open_meteo::write_path`.
        // Sem rede: `NoConnectivity`; tempo limite excedido: `Timeout`.
        let status = 200;
        let body = br#"{"current":{"temperature_2m":27.5,"relative_humidity_2m":70}}"#;
        open_meteo::interpret(status, body)
    }
}

assert_eq!(OpenMeteo {}.fetch(), Ok(ExternalReading::new(27.5, 70.0)));
```

Um status diferente de 200 resulta em `ServiceError`. Um corpo sem `current.temperature_2m` e `current.relative_humidity_2m` numéricos resulta em `MalformedResponse`. A faixa dos valores continua sendo verificada no ciclo, como para qualquer fonte externa.

## Testes sem hardware

O módulo `sim` oferece fontes simuladas cujo resultado pode ser alterado entre ciclos, permitindo reproduzir falhas e recuperações em testes no computador de desenvolvimento.

## Status

Em desenvolvimento no Programa de Pós-Graduação em Computação Aplicada da Universidade Federal de Roraima (PPCA/UFRR). A API pode mudar antes da versão 1.0. O firmware de referência para ESP32 e os experimentos estão no [repositório do projeto](https://github.com/msc-sed-ufrr/SenseBridge).

## Licença

Este projeto é licenciado sob a [Licença Apache 2.0](LICENSE-APACHE) ou a [Licença MIT](LICENSE-MIT), à escolha de quem o utiliza.

Salvo declaração explícita em contrário, qualquer contribuição submetida intencionalmente para inclusão neste trabalho, conforme definido na licença Apache-2.0, será licenciada duplamente como acima, sem termos ou condições adicionais.
