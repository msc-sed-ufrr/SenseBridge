# Lições

Armadilhas que não cabem num teste nem num comentário. Só recebe acréscimos, com a data.

## 2026-11-25 — O `App version` do boot fica preso a um commit antigo

O ESP-IDF imprime no boot `App version` e `Compile time`, e a tentação é usá-los para identificar o firmware de uma captura. No build pelo `esp-idf-sys`, esses valores são gravados quando a parte do ESP-IDF é compilada e não mudam nos builds incrementais seguintes, mesmo depois de novos commits. As 25 capturas oficiais de C1–C5 mostram `3725e20` em três binários diferentes; em 25/11/2026, o firmware compilado no computador de Marcelo, no commit `5c653f2`, ainda carregava `10de03d`, de 21/10.

`cargo clean --release -p esp-idf-sys`, em `firmware/esp32/`, seguido de `cargo build --release`, recompila o ESP-IDF (cerca de 1 min) e corrige o rótulo. Para conferir antes de gravar, sem a placa: `strings target/xtensa-esp32-espidf/release/sense_bridge_esp32 | grep -B1 libespidf` mostra a versão gravada no binário.

Mesmo com o rótulo certo, ele não diz qual `cfg.toml` foi compilado: os valores do `cfg.toml` entram no binário pelo `build.rs`, e o boot mostra só parte deles: o ciclo, o tempo limite, o intervalo da consulta externa e o GPIO do DHT11, mas não a rede nem as coordenadas. Ver a [revisão por pares de 25/11/2026](2026-11-25-revisao-por-pares-s14.md).
