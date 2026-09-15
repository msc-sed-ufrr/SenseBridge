#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherReading {
    pub temperature: f32,
    pub humidity: f32,
}

impl WeatherReading {
    pub fn new(temperature: f32, humidity: f32) -> Self {
        Self {
            temperature,
            humidity,
        }
    }
}