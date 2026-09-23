//! Consulta e interpretação da API [Open-Meteo](https://open-meteo.com), sem alocação.
//!
//! O transporte (rede, HTTP, tempo limite) fica com quem implementa
//! [`ExternalSource`](crate::ExternalSource): este módulo só monta o caminho da consulta e
//! transforma a resposta em [`ExternalReading`] ou [`ExternalFailure`]. A validação de faixa
//! continua no ciclo, como para qualquer fonte externa.

use core::fmt;

use crate::reading::{ExternalFailure, ExternalReading};

/// Servidor da API de previsão.
pub const HOST: &str = "api.open-meteo.com";

/// Profundidade máxima de aninhamento aceita na resposta. Limita a recursão do interpretador,
/// que roda na pilha de um dispositivo embarcado.
const MAX_DEPTH: u8 = 16;

/// Escreve o caminho da consulta das condições atuais de temperatura e umidade relativa nas
/// coordenadas informadas, a ser requisitado em [`HOST`].
pub fn write_path<W: fmt::Write>(out: &mut W, latitude: f32, longitude: f32) -> fmt::Result {
    write!(
        out,
        "/v1/forecast?latitude={latitude}&longitude={longitude}\
         &current=temperature_2m,relative_humidity_2m"
    )
}

/// Interpreta a resposta de uma consulta montada com [`write_path`].
///
/// - Status diferente de 200: [`ExternalFailure::ServiceError`].
/// - Corpo que não é um objeto JSON completo, ou sem `current.temperature_2m` e
///   `current.relative_humidity_2m` numéricos: [`ExternalFailure::MalformedResponse`].
///
/// Os valores não são validados aqui: um número fora da faixa ou infinito é devolvido como
/// leitura e reprovado pela validação do ciclo.
pub fn interpret(status: u16, body: &[u8]) -> Result<ExternalReading, ExternalFailure> {
    if status != 200 {
        return Err(ExternalFailure::ServiceError);
    }
    parse_current(body).ok_or(ExternalFailure::MalformedResponse)
}

fn parse_current(body: &[u8]) -> Option<ExternalReading> {
    let mut json = Json {
        bytes: body,
        pos: 0,
    };
    let mut reading = None;
    json.object(MAX_DEPTH, |json, key, depth| {
        if key == b"current" {
            reading = Some(json.current(depth)?);
            Some(())
        } else {
            json.skip_value(depth)
        }
    })?;
    // Conteúdo depois do objeto raiz indica resposta concatenada ou corrompida.
    json.skip_whitespace();
    if json.pos != json.bytes.len() {
        return None;
    }
    reading
}

/// Leitor JSON mínimo: percorre o documento sem alocar e devolve `None` ao primeiro erro.
struct Json<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Json<'a> {
    /// Lê `temperature_2m` e `relative_humidity_2m` do objeto `current`, ignorando os demais
    /// membros. Um valor ausente, nulo ou não numérico torna a resposta malformada.
    fn current(&mut self, depth: u8) -> Option<ExternalReading> {
        let (mut temperature, mut humidity) = (None, None);
        self.object(depth, |json, key, depth| {
            match key {
                b"temperature_2m" => temperature = Some(json.number()?),
                b"relative_humidity_2m" => humidity = Some(json.number()?),
                _ => json.skip_value(depth)?,
            }
            Some(())
        })?;
        Some(ExternalReading::new(temperature?, humidity?))
    }

    /// Percorre os membros de um objeto, entregando cada chave a `member`, que deve consumir o
    /// valor correspondente.
    fn object(
        &mut self,
        depth: u8,
        mut member: impl FnMut(&mut Self, &'a [u8], u8) -> Option<()>,
    ) -> Option<()> {
        let depth = depth.checked_sub(1)?;
        self.expect(b'{')?;
        if self.eat(b'}') {
            return Some(());
        }
        loop {
            let key = self.string()?;
            self.expect(b':')?;
            member(self, key, depth)?;
            if !self.eat(b',') {
                return self.expect(b'}');
            }
        }
    }

    fn array(&mut self, depth: u8) -> Option<()> {
        let depth = depth.checked_sub(1)?;
        self.expect(b'[')?;
        if self.eat(b']') {
            return Some(());
        }
        loop {
            self.skip_value(depth)?;
            if !self.eat(b',') {
                return self.expect(b']');
            }
        }
    }

    fn skip_value(&mut self, depth: u8) -> Option<()> {
        match self.peek()? {
            b'"' => self.string().map(drop),
            b'{' => self.object(depth, |json, _, depth| json.skip_value(depth)),
            b'[' => self.array(depth),
            b't' => self.literal(b"true"),
            b'f' => self.literal(b"false"),
            b'n' => self.literal(b"null"),
            _ => self.number().map(drop),
        }
    }

    /// Conteúdo de uma string, sem as aspas e sem decodificar escapes.
    fn string(&mut self) -> Option<&'a [u8]> {
        self.expect(b'"')?;
        let start = self.pos;
        loop {
            match *self.bytes.get(self.pos)? {
                b'"' => break,
                b'\\' => self.pos += 2,
                0x00..=0x1f => return None,
                _ => self.pos += 1,
            }
        }
        let content = &self.bytes[start..self.pos];
        self.pos += 1;
        Some(content)
    }

    fn number(&mut self) -> Option<f32> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') = self.bytes.get(self.pos) {
            self.pos += 1;
        }
        let text = &self.bytes[start..self.pos];
        // Em JSON, um número começa por dígito ou sinal de menos.
        if !matches!(text.first(), Some(b'0'..=b'9' | b'-')) {
            return None;
        }
        core::str::from_utf8(text).ok()?.parse().ok()
    }

    fn literal(&mut self, word: &[u8]) -> Option<()> {
        self.skip_whitespace();
        let end = self.pos + word.len();
        if self.bytes.get(self.pos..end)? != word {
            return None;
        }
        self.pos = end;
        Some(())
    }

    /// Consome `byte` se ele for o próximo caractere significativo.
    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.pos += 1;
        }
        found
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        self.eat(byte).then_some(())
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_whitespace();
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.bytes.get(self.pos) {
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MALFORMED: Result<ExternalReading, ExternalFailure> =
        Err(ExternalFailure::MalformedResponse);

    fn ok(body: &str) -> Result<ExternalReading, ExternalFailure> {
        interpret(200, body.as_bytes())
    }

    #[test]
    fn reads_current_values_and_ignores_same_keys_in_other_objects() {
        let body = r#"{"current_units":{"temperature_2m":"°C","relative_humidity_2m":"%"},
                       "current":{"time":"2026-09-23T14:45","temperature_2m":-3.5,
                                  "relative_humidity_2m":51},
                       "extra":{"temperature_2m":99}}"#;

        assert_eq!(ok(body), Ok(ExternalReading::new(-3.5, 51.0)));
    }

    #[test]
    fn member_order_and_whitespace_do_not_matter() {
        let body = "\r\n{ \"current\" :\t{ \"relative_humidity_2m\" : 7e1 ,\n\
                    \"temperature_2m\" : 0 } , \"latitude\" : 2.8 }\n";

        assert_eq!(ok(body), Ok(ExternalReading::new(0.0, 70.0)));
    }

    #[test]
    fn unrelated_members_of_any_json_type_are_skipped() {
        let body = r#"{"a":[1,[2,{"b":null}],"x\"y\\"],"t":true,"f":false,"n":null,
                       "current":{"temperature_2m":25,"relative_humidity_2m":60,"u":[]},
                       "o":{}}"#;

        assert_eq!(ok(body), Ok(ExternalReading::new(25.0, 60.0)));
    }

    #[test]
    fn missing_or_non_numeric_values_make_the_response_malformed() {
        for body in [
            r#"{"current":{"temperature_2m":25}}"#,
            r#"{"current":{"relative_humidity_2m":60}}"#,
            r#"{"current":{"temperature_2m":null,"relative_humidity_2m":60}}"#,
            r#"{"current":{"temperature_2m":"25","relative_humidity_2m":60}}"#,
            r#"{"current":{"temperature_2m":+25,"relative_humidity_2m":60}}"#,
            r#"{"current":{"temperature_2m":.5,"relative_humidity_2m":60}}"#,
            r#"{"current":[25,60]}"#,
            r#"{"latitude":2.8}"#,
            r#"{}"#,
        ] {
            assert_eq!(ok(body), MALFORMED, "{body}");
        }
    }

    #[test]
    fn invalid_or_incomplete_json_makes_the_response_malformed() {
        for body in [
            "",
            "   ",
            "[]",
            r#"{"current":{"temperature_2m":25,"relative_humidity_2m":60}"#,
            r#"{"current":{"temperature_2m":25,"relative_humidity_2m":60}}}"#,
            r#"{"current":{"temperature_2m":25,"relative_humidity_2m":60}} {}"#,
            r#"{"current":{"temperature_2m":25 "relative_humidity_2m":60}}"#,
            r#"{"current":{"temperature_2m":25,"relative_humidity_2m":60},}"#,
            r#"{"x":"unterminated}"#,
            "{\"x\":\"line\nbreak\",\"current\":{\"temperature_2m\":1,\"relative_humidity_2m\":2}}",
            r#"{"x":tru,"current":{"temperature_2m":1,"relative_humidity_2m":2}}"#,
        ] {
            assert_eq!(ok(body), MALFORMED, "{body}");
        }
    }

    /// Resposta válida cujo membro `a` tem `arrays` listas aninhadas.
    fn nested(arrays: usize, buf: &mut [u8; 256]) -> &[u8] {
        let head = br#"{"a":"#;
        let tail = br#","current":{"temperature_2m":1,"relative_humidity_2m":2}}"#;
        let mut len = head.len();
        buf[..len].copy_from_slice(head);
        buf[len..len + arrays].fill(b'[');
        len += arrays;
        buf[len..len + arrays].fill(b']');
        len += arrays;
        buf[len..len + tail.len()].copy_from_slice(tail);
        &buf[..len + tail.len()]
    }

    #[test]
    fn nesting_is_accepted_up_to_the_limit_and_rejected_beyond_it() {
        let mut buf = [0; 256];
        // O objeto raiz ocupa um nível.
        let deepest = usize::from(MAX_DEPTH) - 1;

        assert_eq!(
            interpret(200, nested(deepest, &mut buf)),
            Ok(ExternalReading::new(1.0, 2.0))
        );
        assert_eq!(interpret(200, nested(deepest + 1, &mut buf)), MALFORMED);
    }

    #[test]
    fn non_200_status_is_a_service_error_even_with_a_valid_body() {
        let body = br#"{"current":{"temperature_2m":25,"relative_humidity_2m":60}}"#;

        for status in [201, 400, 429, 500, 503] {
            assert_eq!(
                interpret(status, body),
                Err(ExternalFailure::ServiceError),
                "{status}"
            );
        }
    }

    #[test]
    fn values_are_not_range_checked_here() {
        let body = br#"{"current":{"temperature_2m":1e999,"relative_humidity_2m":150}}"#;
        let reading = interpret(200, body).expect("resposta completa");

        assert!(reading.temperature.is_infinite());
        assert_eq!(reading.humidity, 150.0);
    }
}
