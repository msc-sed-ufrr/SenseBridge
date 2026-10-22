use crate::reading::{ExternalFailure, ExternalReading, InvalidReason, LocalFailure, LocalReading};

/// Intervalo fechado `[min, max]` de valores aceitos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f32,
    pub max: f32,
}

impl Range {
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn check(&self, value: f32) -> Result<(), InvalidReason> {
        if !value.is_finite() {
            Err(InvalidReason::NotANumber)
        } else if value < self.min || value > self.max {
            Err(InvalidReason::OutOfRange)
        } else {
            Ok(())
        }
    }
}

/// Faixas aceitas de temperatura (°C) e umidade relativa (%).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub temperature: Range,
    pub humidity: Range,
}

impl Limits {
    /// Faixa de medição do DHT11 segundo o manual do fabricante (PRD, D3).
    pub const DHT11: Self = Self {
        temperature: Range::new(0.0, 50.0),
        humidity: Range::new(20.0, 90.0),
    };

    /// Limites físicos plausíveis para dados meteorológicos (PRD, D4).
    pub const PLAUSIBLE: Self = Self {
        temperature: Range::new(-90.0, 60.0),
        humidity: Range::new(0.0, 100.0),
    };

    pub fn check(&self, temperature: f32, humidity: f32) -> Result<(), InvalidReason> {
        self.temperature.check(temperature)?;
        self.humidity.check(humidity)
    }
}

pub fn validate_local(
    reading: LocalReading,
    limits: &Limits,
) -> Result<LocalReading, LocalFailure> {
    limits
        .check(reading.temperature, reading.humidity)
        .map(|()| reading)
        .map_err(LocalFailure::Invalid)
}

pub fn validate_external(
    reading: ExternalReading,
    limits: &Limits,
) -> Result<ExternalReading, ExternalFailure> {
    limits
        .check(reading.temperature, reading.humidity)
        .map(|()| reading)
        .map_err(ExternalFailure::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_on_range_boundaries_are_accepted() {
        let limits = Limits::DHT11;

        assert_eq!(limits.check(0.0, 20.0), Ok(()));
        assert_eq!(limits.check(50.0, 90.0), Ok(()));
    }

    #[test]
    fn values_just_outside_range_are_rejected() {
        let limits = Limits::DHT11;

        assert_eq!(limits.check(-0.1, 50.0), Err(InvalidReason::OutOfRange));
        assert_eq!(limits.check(50.1, 50.0), Err(InvalidReason::OutOfRange));
        assert_eq!(limits.check(25.0, 19.9), Err(InvalidReason::OutOfRange));
        assert_eq!(limits.check(25.0, 90.1), Err(InvalidReason::OutOfRange));
    }

    #[test]
    fn non_finite_values_are_rejected() {
        let limits = Limits::PLAUSIBLE;

        assert_eq!(limits.check(f32::NAN, 50.0), Err(InvalidReason::NotANumber));
        assert_eq!(
            limits.check(25.0, f32::INFINITY),
            Err(InvalidReason::NotANumber)
        );
        assert_eq!(
            limits.check(f32::NEG_INFINITY, 50.0),
            Err(InvalidReason::NotANumber)
        );
    }

    #[test]
    fn invalid_readings_carry_the_reason_for_their_source() {
        assert_eq!(
            validate_local(LocalReading::new(25.0, 95.0), &Limits::DHT11),
            Err(LocalFailure::Invalid(InvalidReason::OutOfRange))
        );
        assert_eq!(
            validate_external(ExternalReading::new(f32::NAN, 70.0), &Limits::PLAUSIBLE),
            Err(ExternalFailure::Invalid(InvalidReason::NotANumber))
        );
    }

    #[test]
    fn valid_readings_are_returned_unchanged() {
        let reading = LocalReading::new(25.0, 60.0);

        assert_eq!(validate_local(reading, &Limits::DHT11), Ok(reading));
    }
}
