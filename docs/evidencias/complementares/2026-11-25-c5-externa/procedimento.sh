#!/bin/bash
# C5 com a fonte externa (BR-30): a rede do ESP32 começa desligada, volta depois do ciclo 2.
# Uso: c5-rede.sh <rep inicial> <rep final>
# Avisos por voz (say): DESLIGUE antes de cada repetição, LIGUE depois do ciclo 2.
ROOT=/Users/bonus/Projects/SenseBridge
DIR=$ROOT/capturas/2026-11-25-c5-externa
PORT=/dev/cu.usbserial-0001
CONSOLE=$ROOT/target/debug/sense_bridge_console
OFF_WAIT=${OFF_WAIT:-25}
mkdir -p "$DIR"
LOG=$DIR/procedimento.log
now() { python3 -I -c 'import datetime as d;print(d.datetime.now(d.timezone.utc).isoformat(timespec="milliseconds"))'; }
rows() { cat "$1.csv" 2>/dev/null | grep -cE '^[0-9]+,'; }
note() { echo "$(now) $*" | tee -a "$LOG"; }

r=${1:-1}
while [ "$r" -le "${2:-5}" ]; do
  out=$DIR/c5-r$r
  rm -f "$out".*
  note "r$r: pedido para DESLIGAR a rede; reinício em ${OFF_WAIT}s"
  say "Repetição $r. Desligue a rede do ESP32 agora." &
  sleep "$OFF_WAIT"
  ( espflash monitor --port $PORT --non-interactive 2>&1 | $CONSOLE --out "$out" > "$out.console" 2>&1 ) &
  pid=$!
  note "r$r: placa reiniciada"
  # A rede precisa estar fora no boot; senão a repetição não começa com falha.
  boot=""
  for i in $(seq 1 120); do
    sleep 0.5
    if grep -q 'rede disponível após' "$out.txt" 2>/dev/null; then boot=up; break; fi
    if grep -q 'rede indisponível após' "$out.txt" 2>/dev/null; then boot=down; break; fi
  done
  if [ "$boot" != down ]; then
    pkill -f "espflash monitor --port $PORT"; wait $pid 2>/dev/null
    note "r$r: rede DISPONÍVEL no boot ($boot); repetição descartada, refazendo"
    mv "$out.txt" "$out.descartada-$(date +%H%M%S).txt" 2>/dev/null
    say "A rede ainda estava ligada. Vamos refazer a repetição $r."
    continue
  fi
  note "r$r: rede indisponível no boot, como previsto"
  signaled=""; recovered=""; last=0
  for i in $(seq 1 1200); do
    sleep 0.5
    n=$(rows "$out")
    if [ "$n" -gt "$last" ]; then note "r$r: ciclo $((n-1)) recebido"; last=$n; fi
    if [ -z "$signaled" ] && [ "$n" -ge 3 ]; then
      note "r$r: pedido para LIGAR a rede (depois do ciclo 2)"
      say "Ligue a rede agora." &
      signaled=1
    fi
    if [ -z "$recovered" ] && awk -F, 'NR>1 && $12=="true"{f=1} END{exit !f}' "$out.csv" 2>/dev/null; then
      recovered=$(awk -F, 'NR>1 && $12=="true"{print $1; exit}' "$out.csv")
      note "r$r: recuperação no ciclo $recovered"
    fi
    # Protocolo: 10 ciclos; se a rede demorar, segue até a recuperação e mais 2 ciclos (no máximo 40).
    if [ "$n" -ge 10 ] && [ -n "$recovered" ] && [ "$n" -ge $((recovered + 3)) ]; then break; fi
    if [ "$n" -ge 40 ]; then note "r$r: 40 ciclos sem recuperação"; break; fi
  done
  pkill -f "espflash monitor --port $PORT"; wait $pid 2>/dev/null
  note "r$r: fim, $(rows "$out") ciclos"
  say "Repetição $r concluída."
  r=$((r + 1))
done
note "fim das repetições"
"$CONSOLE" --analyze "$DIR"/c5-r*.csv | tee -a "$LOG"
say "Capturas de C5 concluídas."
