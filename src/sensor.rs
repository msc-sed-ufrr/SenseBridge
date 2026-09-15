#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorReading {
    pub temperature: f32,
    pub humidity: f32,
}

impl SensorReading {
    pub fn new(temperature: f32, humidity: f32) -> Self {
        Self {
            temperature,
            humidity,
        }
    }
}