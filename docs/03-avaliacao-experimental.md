# Avaliação Experimental do SenseBridge

## 1. Objetivo

A avaliação experimental do SenseBridge teve como objetivo verificar:

- o comportamento da biblioteca diante da disponibilidade e indisponibilidade das fontes de dados;
- as transições entre os estados operacionais;
- a recuperação após falhas;
- a integração com a fonte externa Open-Meteo;
- a latência de execução;
- o tamanho do firmware;
- o uso de memória no ESP32.

Foram considerados os cenários experimentais C1 a C5.

---

## 2. Cenários C1–C5

| Cenário | Condição | Estado esperado | Resultado |
|---|---|---|---|
| C1 | Fonte local e externa disponíveis | `Normal` | Aprovado |
| C2 | Falha da fonte local | `DegradedLocal` | Aprovado |
| C3 | Falha da fonte externa | `DegradedExternal` | Aprovado |
| C4 | Falha simultânea das duas fontes | `Error` | Aprovado |
| C5 | Recuperação das fontes | retorno para `Normal` | Aprovado |

Os cenários C1, C3, C4 e C5 foram exercitados utilizando as fontes simuladas da própria biblioteca.

Essa abordagem permite introduzir falhas de forma determinística e verificar as transições de estado sem depender de eventos externos imprevisíveis.

### 2.1 C1 — Operação normal

As duas fontes permanecem disponíveis.

Resultado esperado:

```text
Normal
```

O SenseBridge integra corretamente as leituras local e externa.

### 2.2 C2 — Falha da fonte local

A fonte local retorna:

```text
LocalFailure::NoResponse
```

A fonte externa continua disponível.

Resultado esperado:

```text
DegradedLocal
```

Esse cenário também foi observado diretamente no ESP32.

Foram capturados 14 ciclos consecutivos em que:

- o DHT11 permaneceu indisponível;
- a falha local foi registrada como `NoResponse`;
- a fonte Open-Meteo continuou operacional;
- o sistema permaneceu em `DegradedLocal`.

Assim, a indisponibilidade da fonte local não interrompeu o funcionamento do sistema.

### 2.3 C3 — Falha da fonte externa

A fonte local permanece válida e a fonte externa retorna:

```text
ExternalFailure::Timeout
```

Resultado esperado:

```text
DegradedExternal
```

A leitura local é preservada.

### 2.4 C4 — Falha simultânea

As duas fontes são configuradas para falhar simultaneamente.

Resultado esperado:

```text
Error
```

Mesmo nessa condição, o SenseBridge continua realizando novas tentativas nos ciclos seguintes.

### 2.5 C5 — Recuperação

Após uma condição de falha, as duas fontes tornam-se novamente disponíveis.

Resultado esperado:

```text
Normal
```

O sistema retorna ao estado normal no primeiro ciclo em que ambas as fontes voltam a produzir leituras válidas.

---

## 3. Testes automatizados

A implementação foi submetida aos seguintes conjuntos de testes:

| Conjunto | Testes |
|---|---:|
| Testes unitários da biblioteca | 18 |
| Integração Open-Meteo | 7 |
| Cenários experimentais | 12 |
| Console de captura | 14 |
| Doctests | 2 |
| **Total** | **53** |

Resultado:

```text
53 testes aprovados
0 falhas
```

Além dos cenários C1–C5, os testes verificam:

- reutilização da leitura externa dentro do intervalo configurado;
- descarte de leitura externa anterior após falha;
- rejeição de valores inválidos;
- comportamento diante de respostas inválidas da Open-Meteo;
- geração de transições somente quando o estado realmente muda;
- incremento dos identificadores dos ciclos;
- geração e interpretação dos registros do console.

---

## 4. Latência

A latência foi medida utilizando o campo:

```text
cycle_us
```

registrado pelo próprio firmware durante o cenário C2.

### 4.1 Todos os ciclos

Foram analisados 14 ciclos.

| Métrica | Resultado |
|---|---:|
| Média | 63.566,36 µs |
| Mínimo | 18.535 µs |
| Máximo | 648.301 µs |

O maior valor ocorreu no primeiro ciclo, que inclui a consulta HTTP à API Open-Meteo.

### 4.2 Regime estável

Excluindo o primeiro ciclo, foram analisados 13 ciclos.

| Métrica | Resultado |
|---|---:|
| Média | **18.586,77 µs** |
| Mínimo | 18.535 µs |
| Máximo | 18.662 µs |

Assim, em regime estável, o ciclo do SenseBridge apresentou latência aproximada de:

```text
18,6 ms
```

---

## 5. Tamanho do firmware

O firmware em modo `release` foi analisado com:

```text
xtensa-esp32-elf-size
```

Resultado:

| Segmento | Tamanho |
|---|---:|
| `text` | 929.628 bytes |
| `data` | 192.026 bytes |
| `bss` | 19.891 bytes |
| Total reportado | 1.141.545 bytes |

Estimativas derivadas:

```text
Flash aproximada (text + data):
1.121.654 bytes

RAM estática aproximada (data + bss):
211.917 bytes
```

Esses valores caracterizam os segmentos do executável e não representam, isoladamente, toda a memória física efetivamente utilizada em execução.

---

## 6. Memória durante a execução

O firmware registrou indicadores de memória durante o cenário C2.

| Indicador | Valor |
|---|---:|
| Heap livre mínimo observado | 206.856 bytes |
| Menor heap livre desde o boot | 201.120 bytes |
| Stack mínima livre | 11.980 bytes |

Os resultados indicam que permaneceu disponível memória dinâmica durante a execução observada.

---

## 7. Ambiente experimental

O experimento em hardware utilizou:

- ESP32;
- Rust;
- ESP-IDF v5.5.5;
- Open-Meteo como fonte externa;
- DHT11 como fonte local prevista;
- Wi-Fi;
- firmware correspondente ao commit `3725e20`;
- ciclo de execução configurado para 5 segundos;
- tempo limite HTTP de 3 segundos;
- atualização externa configurada para 900 segundos.

---

## 8. Limitação observada no DHT11

O módulo disponível para o experimento foi um shield:

```text
ESP-01S DHT11 v1.0
```

originalmente projetado para utilização com ESP-01/ESP8266.

A alimentação do shield foi confirmada por meio do LED de energia. Entretanto, durante os testes com o ESP32, não foi obtida resposta válida do DHT11 na linha de dados.

Foram realizados testes temporários de diagnóstico, incluindo:

- geração do pulso de inicialização;
- monitoramento da linha DATA;
- busca por resposta em nível lógico LOW após o comando de início.

Nenhuma resposta válida foi detectada.

Por esse motivo, a leitura local real permaneceu classificada como:

```text
NoResponse
```

Essa limitação não impediu a avaliação da lógica de tolerância a falhas, pois permitiu observar diretamente em hardware o cenário C2.

Entretanto, esse resultado não deve ser interpretado como validação de uma leitura DHT11 bem-sucedida no ESP32.

Como trabalho complementar, a integração física deverá ser repetida utilizando um módulo DHT11 convencional ou outro sensor diretamente compatível com o ESP32.

---

## 9. Síntese

Os experimentos demonstraram que o SenseBridge:

- representa explicitamente os estados das fontes;
- continua operando quando somente uma fonte falha;
- identifica falha simultânea das duas fontes;
- continua tentando obter novas leituras mesmo após falhas;
- retorna automaticamente ao estado `Normal` após a recuperação;
- integra a fonte externa Open-Meteo;
- possui testes automatizados para os principais comportamentos;
- registra informações de latência e memória no ESP32.

Foram executados:

```text
53 testes
53 aprovados
0 falhas
```

Os cinco cenários C1–C5 foram validados.

A execução em hardware também demonstrou o funcionamento real do estado:

```text
DegradedLocal
```

permanecendo pendente apenas a conclusão da leitura física normal do sensor local.
