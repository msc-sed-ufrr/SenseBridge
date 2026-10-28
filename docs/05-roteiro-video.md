# Roteiro do vídeo de demonstração

> **Versão preparada em 28/10/2026.** Duração prevista: cerca de 4 min 30 s (o limite é de 3 a 5 min).

## Preparação

- `cfg.toml` com o Wi-Fi de um roteador de celular, que pode ser desligado durante a gravação, e `external_refresh_ms = 5000`, para que a fonte externa seja consultada a cada ciclo. Gravar o firmware antes de começar.
- DHT11 ligado ao ESP32 com jumpers fáceis de soltar (ligação no [README](../README.md#2-firmware-no-esp32)).
- Rodar `cargo test` uma vez antes, para que a gravação não mostre a compilação.
- Duas janelas: a câmera sobre a placa e o terminal com o console, com fonte grande.
- Ensaiar a demonstração ao vivo inteira uma vez. Depois que o roteador volta, a reassociação do Wi-Fi leva alguns segundos.

## Cenas

| Tempo | Cena | O que mostrar | Fala (pontos principais) |
| --- | --- | --- | --- |
| 0:00–0:20 | Abertura | Título, autores, orientador, disciplina | Biblioteca em Rust para ESP32 que integra um sensor local e uma fonte meteorológica externa, com comportamento definido quando as fontes falham. |
| 0:20–0:50 | Problema e arquitetura | Diagrama de topologia da [arquitetura](02-arch.md#topologia) | Sem tratamento explícito, um valor inválido passa silenciosamente. A biblioteca recebe as fontes por traits, valida cada leitura e deriva um de quatro estados: `Normal`, `DegradedLocal`, `DegradedExternal` e `Error`. |
| 0:50–1:20 | Verificação sem hardware | `cargo test` (62 testes aprovados) e `cargo run -p sense_bridge --example simulate` | O núcleo é testado no computador, sem placa nem rede, com fontes simuladas e respostas reais gravadas da Open-Meteo. A CI repete isso a cada push. |
| 1:20–1:40 | Montagem | Placa na câmera e o esquema de ligação | ESP32, DHT11 no GPIO4, Wi-Fi de 2,4 GHz. |
| 1:40–2:00 | C1 ao vivo | `espflash monitor --non-interactive \| cargo run -q -p sense_bridge_console -- --no-save`; ciclos em `Normal` | Cada linha é um ciclo de 5 s: leitura local, leitura externa e duração do ciclo. |
| 2:00–2:25 | C2 e recuperação | Soltar o DATA do DHT11: `DegradedLocal` e `NoResponse`. Religar: transição para `Normal` marcada como recuperação | A fonte externa continua chegando enquanto a local falha. A volta acontece no primeiro ciclo com as duas fontes válidas. |
| 2:25–2:50 | C3 | Desligar o roteador: `DegradedExternal` e `NoConnectivity` | O sensor local segue válido; a leitura externa antiga é descartada, e não apresentada como atual. |
| 2:50–3:10 | C4 | Com o roteador desligado, soltar o DHT11: `Error` | Mesmo em erro, o firmware continua tentando a cada ciclo. |
| 3:10–3:30 | Recuperação total | Religar o DHT11 e o roteador: volta a `Normal`. Mostrar a linha de resumo do console | Recuperação automática, sem reiniciar o dispositivo. O resumo conta ciclos por estado e falhas por tipo. |
| 3:30–4:10 | Resultados | Tabela de resultados do [README](../README.md#resultados) | C1–C5 com 5 repetições de 10 ciclos, todas aprovadas; ciclos de 18 a 22 ms sem consulta externa; cerca de 23,5 KiB de Flash e 13,4 KiB de RAM estática a mais que o firmware-base equivalente. |
| 4:10–4:30 | Encerramento | Página do repositório no GitHub | Código, dados, documentação e passo a passo de reprodução no repositório, sob licença MIT ou Apache-2.0. |

## Observações

- A demonstração ao vivo desliga o roteador para mostrar a falha externa; nos experimentos oficiais, C3 foi induzido com uma rede inexistente em `cfg.toml`. Dizer isso na cena de C3 evita confundir o procedimento do vídeo com o do protocolo.
- A cena de C2 pode mostrar também o aviso do firmware com a etapa do protocolo em que a leitura falhou, identificada pela crate `dht11-trace`.
