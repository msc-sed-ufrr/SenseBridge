#!/bin/sh
# Gera docs/evidencias/verificacao.md: formatação, análise estática, testes, documentação e
# pacote da biblioteca, com o commit e as versões usadas.
#
# Com --firmware, também verifica e compila o firmware do ESP32 e registra o tamanho dos
# segmentos. Requer a toolchain do espup carregada no terminal (`. ~/export-esp.sh`).
set -eu

cd "$(dirname "$0")/.."
destino=docs/evidencias/verificacao.md
firmware=false
case "${1:-}" in
    --firmware) firmware=true ;;
    "") ;;
    *) echo "uso: $0 [--firmware]" >&2; exit 2 ;;
esac

falhas=0

# Remove o que muda a cada execução sem dizer nada sobre o resultado: progresso da
# compilação, tempos e o hash dos binários de teste.
filtrar() {
    grep -vE '^ *(Compiling|Checking|Finished|Documenting|Generated|Packaging|Verifying|Updating|Locking|Blocking|Downloaded|Downloading|Running `)' |
        sed -E 's/; finished in [0-9.]+s//; s/-[0-9a-f]{16}\)/)/'
}

registrar() {
    printf '\n## %s\n\n```text\n$ %s\n' "$1" "$2"
    if saida=$(sh -c "$2" 2>&1); then
        resultado=ok
    else
        resultado=FALHOU
        falhas=$((falhas + 1))
    fi
    if [ -n "$saida" ]; then
        printf '%s\n' "$saida" | filtrar
    fi
    printf '```\n\nResultado: **%s**\n' "$resultado"
}

alteracoes=$(git status --porcelain -- . ':!docs/evidencias')

{
    echo "# Verificação do SenseBridge"
    echo
    echo "Gerado por \`scripts/evidencias.sh${1:+ $1}\`. Não edite à mão."
    echo
    echo "| Item | Valor |"
    echo "| --- | --- |"
    echo "| Data (UTC) | $(date -u +%Y-%m-%dT%H:%M:%SZ) |"
    echo "| Commit | \`$(git rev-parse --short HEAD)\`${alteracoes:+ (com alterações locais)} |"
    echo "| Sistema | $(uname -sm) |"
    echo "| Rust | $(rustc --version) |"
    if $firmware; then
        echo "| Rust do firmware | $(cd firmware/esp32 && rustc --version) |"
        echo "| ESP-IDF | $(sed -n 's/^ESP_IDF_VERSION = "\(.*\)"/\1/p' firmware/esp32/.cargo/config.toml) |"
    fi

    registrar "Formatação" "cargo fmt --check"
    registrar "Análise estática" "cargo clippy --all-targets -- -D warnings"
    registrar "Testes" "cargo test"
    registrar "Documentação da API" "RUSTDOCFLAGS='-D warnings' cargo doc -p sense_bridge --no-deps"
    registrar "Pacote para o crates.io" "cargo package -p sense_bridge --allow-dirty"

    if $firmware; then
        registrar "Formatação do firmware" \
            "rustfmt --edition 2024 --check firmware/esp32/src/main.rs firmware/esp32/build.rs"
        registrar "Análise estática do firmware" "cd firmware/esp32 && cargo clippy --release"
        registrar "Tamanho do firmware (release)" \
            "cd firmware/esp32 && cargo build --release && xtensa-esp32-elf-size target/xtensa-esp32-espidf/release/sense_bridge_esp32"
    fi
} >"$destino"

echo "$destino: $falhas etapa(s) com falha"
[ "$falhas" -eq 0 ]
