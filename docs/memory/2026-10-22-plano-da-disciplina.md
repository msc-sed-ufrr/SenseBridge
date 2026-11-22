# Plano da disciplina — situação em 22/10/2026

> Retirado do PRD (`docs/01-prd.md`, seções 8, 9 e 12) na adoção de `docs/knowledge/`, em 22/11/2026. É histórico: o estado atual está em [product.md](../knowledge/product.md), e o que falta, em "Perguntas em aberto". A tabela de status do PRD (seção 5) não foi trazida; ela continua no histórico do git.

## Riscos e mitigações

| Risco | Probabilidade | Impacto | Mitigação |
| --- | --- | --- | --- |
| Dificuldades na integração do Rust com o ESP32 | Média | Alto | Restringir a implementação às abstrações e funcionalidades essenciais, usando HALs e bibliotecas existentes e consolidadas; configurar o alvo ESP32 logo no início |
| Instabilidade da Open-Meteo ou do Wi-Fi durante os testes | Média | Médio | Usar respostas simuladas ou gravadas nos testes automatizados; manter a API real apenas na validação experimental |
| Tempo insuficiente para implementação, testes e validação | Média | Alto | Priorizar o núcleo funcional, os testes e os cenários C1–C5; adiar funcionalidades complementares |
| Falhas intermitentes do DHT11 (temporização, integridade) distorcerem C1 | Média | Médio | Registrar o tipo de cada falha; respeitar o intervalo mínimo de amostragem; relatar a taxa de falhas espontâneas separadamente das induzidas |
| Instrumentação alterar a latência medida | Média | Médio | Guardar as medições em memória e exportar fora da janela medida; quantificar o custo da própria instrumentação |
| Custo de RAM/Flash da pilha de rede mascarar o custo próprio da biblioteca | Alta | Médio | Medir também um firmware de referência sem a biblioteca e relatar a diferença |
| Indução manual de falhas pouco reproduzível | Média | Médio | Documentar o procedimento passo a passo; quando possível, induzir falhas de forma controlada por software |
| Consultas frequentes excederem os limites de uso da Open-Meteo | Baixa | Baixo | Intervalo de consulta externa configurável e conservador, independente do ciclo local; revisar a nova tentativa a cada ciclo durante indisponibilidade (D11) |
| Consulta externa lenta atrasar o ciclo inteiro, afetando a cadência do DHT11 e a latência medida em C3 | Média | Médio | Tempo limite curto; relatar a latência de C3 separadamente; medir a duração do ciclo sob tempo limite |
| DHT11 travado em um valor plausível passar pela validação | Baixa | Médio | Detector opcional de leitura congelada, que marca a suspeita sem mudar o estado; limiar bem acima das repetições legítimas observadas (D14) |
| Licença aberta conflitar com a política de propriedade intelectual da UFRR ou com o registro de software previsto | Baixa | Médio | Confirmar titularidade e licença com o orientador e o núcleo de inovação antes de tornar o repositório público; confirmadas antes da publicação da crate |

## Cronograma de fechamento

O desenvolvimento estava adiantado em relação ao cronograma original do projeto de pesquisa (11 semanas, entrega em 26/11/2026).

| Fase | Situação em 22/10 | Próxima ação |
| --- | --- | --- |
| Revisão e protocolo | Concluída | Decisões D1–D13 fechadas |
| Implementação do núcleo | Concluída | Apenas correções da revisão final |
| Firmware ESP32 | Concluído | Manter configuração reproduzível |
| Open-Meteo | Concluída | Apenas documentação final |
| DHT11 físico | Concluído | Leitura real validada; SB-10 encerrada |
| Testes automatizados | 62/62 | Reexecutar na versão final |
| Instrumentação | Concluída | Baseline de RAM/Flash (D8) medido |
| Experimentos | Concluídos | C1–C5 no ESP32: 25 repetições, 250 ciclos |
| Análise de resultados | Avançada | Versionar os dados brutos e registrar o firmware medido |
| Documentação | Concluída | README de reprodutibilidade, PRD e arquitetura atualizados |
| Relatório | Rascunho | Revisar o relatório técnico |
| Vídeo | Roteiro pronto | Gravar seguindo o roteiro |

Próximas atividades listadas em 22/10:

1. Versionar as capturas oficiais de C1–C5 e o firmware-base em `docs/evidencias/`.
2. Registrar o commit do firmware medido ou remedir Flash e RAM com o firmware atual.
3. Conferir as capturas versionadas com `sense_bridge_console --analyze` (UC-008).
4. Revisar o rascunho do relatório técnico.
5. Gravar o vídeo de demonstração de 3–5 minutos.

As quatro primeiras foram concluídas; a crate `sense_bridge` 0.1.0 foi publicada no crates.io em 28/10/2026.

## Registro experimental do DHT11

A integração e a leitura real do DHT11 foram concluídas e validadas, o que encerrou a SB-10. Em uma montagem posterior, um shield ESP-01S DHT11 v1.0 usado em outro conjunto físico recebeu alimentação, mas não produziu resposta válida na linha de dados quando ligado ao ESP32. O episódio fica apenas como evidência de uma condição específica de hardware e montagem; não é pendência do SenseBridge nem reabre a SB-10.
