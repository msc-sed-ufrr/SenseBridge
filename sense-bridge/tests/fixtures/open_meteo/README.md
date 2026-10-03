# Respostas gravadas da Open-Meteo

Corpos de resposta reais, gravados sem alteração, para testar a interpretação sem rede.

| Arquivo | Status | Consulta | Gravado em |
| --- | --- | --- | --- |
| `current.json` | 200 | `/v1/forecast?latitude=2.82&longitude=-60.67&current=temperature_2m,relative_humidity_2m` | 23/09/2026 |
| `error_latitude.json` | 400 | a mesma consulta com `latitude=200` | 23/09/2026 |

Para gravar uma nova resposta, salve apenas o corpo, sem cabeçalhos nem quebra de linha final:

```sh
curl -sS -o current.json \
  "https://api.open-meteo.com/v1/forecast?latitude=2.82&longitude=-60.67&current=temperature_2m,relative_humidity_2m"
```
