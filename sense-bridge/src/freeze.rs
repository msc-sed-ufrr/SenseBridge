//! Suspeita de leitura local congelada.
//!
//! A validação de faixa aceita um sensor travado em um valor plausível. O [`FreezeDetector`]
//! acompanha os registros e marca como suspeita a leitura válida que se repete, idêntica, por
//! muitos ciclos seguidos.
//!
//! A suspeita é só um indicador. A leitura continua válida e o estado de operação não muda,
//! porque a resolução de 1 °C e 1 %UR do DHT11 faz repetições legítimas serem comuns: nas
//! capturas do projeto, a mesma temperatura e a mesma umidade se repetiram por até 64 ciclos
//! de 5 s (cerca de 5 min 20 s) com o sensor funcionando. Por isso, o limiar deve ficar bem
//! acima da maior sequência legítima esperada no ambiente de uso.

use core::num::NonZeroU32;

use crate::reading::{LocalFailure, LocalReading};

/// Conta os ciclos seguidos com a mesma leitura local válida e sinaliza a suspeita de
/// congelamento quando a contagem atinge o limiar.
///
/// Uma falha da fonte local encerra a sequência: depois dela, a contagem recomeça.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreezeDetector {
    threshold: NonZeroU32,
    last: Option<LocalReading>,
    run: u32,
}

impl FreezeDetector {
    /// Detector que marca a suspeita a partir de `threshold` ciclos seguidos com a mesma
    /// leitura, contando o primeiro.
    pub const fn new(threshold: NonZeroU32) -> Self {
        Self {
            threshold,
            last: None,
            run: 0,
        }
    }

    /// Limiar, em ciclos.
    pub const fn threshold(&self) -> NonZeroU32 {
        self.threshold
    }

    /// Registra o resultado local de um ciclo, como [`Record::local`](crate::Record::local), e
    /// devolve a contagem atualizada (ver [`FreezeDetector::run`]).
    pub fn observe(&mut self, local: &Result<LocalReading, LocalFailure>) -> u32 {
        match local {
            Ok(reading) if self.last == Some(*reading) => {
                self.run = self.run.saturating_add(1);
            }
            Ok(reading) => {
                self.last = Some(*reading);
                self.run = 1;
            }
            Err(_) => {
                self.last = None;
                self.run = 0;
            }
        }
        self.run
    }

    /// Ciclos seguidos, até o último observado, com a mesma leitura válida: 1 quando ela mudou
    /// ou é a primeira, 0 quando a fonte esteve indisponível.
    pub const fn run(&self) -> u32 {
        self.run
    }

    /// Se a última leitura observada é suspeita de congelamento: repetida em pelo menos
    /// [`FreezeDetector::threshold`] ciclos seguidos.
    pub const fn is_suspect(&self) -> bool {
        self.run >= self.threshold.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const READING: Result<LocalReading, LocalFailure> = Ok(LocalReading::new(24.0, 37.0));

    fn detector(threshold: u32) -> FreezeDetector {
        FreezeDetector::new(NonZeroU32::new(threshold).unwrap())
    }

    #[test]
    fn starts_without_suspicion() {
        let detector = detector(1);

        assert_eq!(detector.run(), 0);
        assert!(!detector.is_suspect());
    }

    #[test]
    fn repeated_reading_becomes_suspect_at_the_threshold() {
        let mut detector = detector(3);

        assert_eq!(detector.observe(&READING), 1);
        assert_eq!(detector.observe(&READING), 2);
        assert!(!detector.is_suspect());
        assert_eq!(detector.observe(&READING), 3);
        assert!(detector.is_suspect());
        assert_eq!(detector.observe(&READING), 4);
        assert!(detector.is_suspect());
    }

    #[test]
    fn any_change_restarts_the_count() {
        let mut detector = detector(2);
        detector.observe(&READING);
        detector.observe(&READING);

        assert_eq!(detector.observe(&Ok(LocalReading::new(24.0, 38.0))), 1);
        assert!(!detector.is_suspect());
        assert_eq!(detector.observe(&Ok(LocalReading::new(25.0, 38.0))), 1);
    }

    #[test]
    fn a_local_failure_ends_the_sequence() {
        let mut detector = detector(2);
        detector.observe(&READING);
        detector.observe(&READING);

        assert_eq!(detector.observe(&Err(LocalFailure::NoResponse)), 0);
        assert!(!detector.is_suspect());
        assert_eq!(detector.observe(&READING), 1);
    }

    #[test]
    fn the_count_saturates() {
        let mut detector = FreezeDetector {
            run: u32::MAX,
            ..detector(1)
        };
        detector.last = Some(LocalReading::new(24.0, 37.0));

        assert_eq!(detector.observe(&READING), u32::MAX);
        assert!(detector.is_suspect());
    }
}
