# Changelog

Mudanças relevantes da crate `sense_bridge`. O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), e as versões seguem o [Versionamento Semântico](https://semver.org/lang/pt-BR/).

## [Não publicado]

Primeira versão, a ser publicada como 0.1.0.

### Adicionado

- Ciclo de aquisição `SenseBridge::cycle`, que integra uma fonte local (`LocalSource`) e uma externa (`ExternalSource`) e produz um `Record` por ciclo, com o resultado de cada fonte e a transição de estado.
- Validação de faixa e de finitude (`Range`, `Limits`), com as faixas `Limits::DHT11` e `Limits::PLAUSIBLE`.
- Estados de operação `Normal`, `DegradedLocal`, `DegradedExternal` e `Error`, com transições e identificação da recuperação (`Transition::is_recovery`).
- Reuso da última leitura externa válida dentro de `Config::external_refresh_ms`, descartada na primeira falha.
- Nomes estáveis de estados e falhas em `as_str()`, para serializar registros.
- Módulo `open_meteo`: caminho da consulta e interpretação da resposta da API Open-Meteo, sem alocação.
- Módulo `sim`: fontes simuladas para testes sem hardware.
