# Changelog

Mudanças relevantes da crate `sense_bridge`. O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), e as versões seguem o [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [0.1.1] - 2026-11-22

A lacuna da leitura congelada foi apontada durante a validação de outro trabalho, o laboratório SENTINELA (UFRR/PPCA, Arquitetura e Organização de Computadores, 2026), cujo intertravamento trata leitura congelada como falha do sensor.

O cenário C6, que exercita o detector no ESP32 por injeção de falha, está no firmware de referência e no console do repositório. Ele ainda não foi executado em hardware, e o limiar sugerido de 360 ciclos ainda não foi validado.

### Adicionado

- `FreezeDetector`, opcional, que marca como suspeita de congelamento a leitura local válida repetida, idêntica, por um número configurável de ciclos. Não altera a validação, o `Record` nem o estado de operação.
- `FreezeDetector::reached_threshold`, verdadeiro no ciclo em que a suspeita começa e a cada limiar seguinte sem mudança, para limitar uma reação à suspeita, como reiniciar o sensor.

## [0.1.0] - 2026-10-28

Primeira versão, publicada no crates.io.

### Adicionado

- Ciclo de aquisição `SenseBridge::cycle`, que integra uma fonte local (`LocalSource`) e uma externa (`ExternalSource`) e produz um `Record` por ciclo, com o resultado de cada fonte e a transição de estado.
- Validação de faixa e de finitude (`Range`, `Limits`), com as faixas `Limits::DHT11` e `Limits::PLAUSIBLE`.
- Estados de operação `Normal`, `DegradedLocal`, `DegradedExternal` e `Error`, com transições e identificação da recuperação (`Transition::is_recovery`).
- Reuso da última leitura externa válida dentro de `Config::external_refresh_ms`, descartada na primeira falha.
- Nomes estáveis de estados e falhas em `as_str()`, para serializar registros.
- Módulo `open_meteo`: caminho da consulta e interpretação da resposta da API Open-Meteo, sem alocação.
- Módulo `sim`: fontes simuladas para testes sem hardware.
