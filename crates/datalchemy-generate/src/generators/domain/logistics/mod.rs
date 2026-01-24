use rand::Rng;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(TrackingCodeGenerator));
    registry.register_generator(Box::new(ShipmentStatusGenerator));
    registry.register_generator(Box::new(CarrierGenerator));
    registry.register_generator(Box::new(DimensionsGenerator));
}

struct TrackingCodeGenerator;

impl Generator for TrackingCodeGenerator {
    fn id(&self) -> &'static str {
        "domain.logistics.tracking_code"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let mut digits = String::new();
        for _ in 0..10 {
            let value = rng.gen_range(0..=9);
            digits.push_str(&value.to_string());
        }
        Ok(GeneratedValue::Text(format!("BR{digits}")))
    }
}

struct ShipmentStatusGenerator;

impl Generator for ShipmentStatusGenerator {
    fn id(&self) -> &'static str {
        "domain.logistics.shipment_status"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["criado", "em_transito", "entregue", "cancelado"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct CarrierGenerator;

impl Generator for CarrierGenerator {
    fn id(&self) -> &'static str {
        "domain.logistics.carrier"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["correios", "jadlog", "total_express", "azul_cargo"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct DimensionsGenerator;

impl Generator for DimensionsGenerator {
    fn id(&self) -> &'static str {
        "domain.logistics.dimensions_cm"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let length = rng.gen_range(10..=100);
        let width = rng.gen_range(10..=100);
        let height = rng.gen_range(5..=80);
        Ok(GeneratedValue::Text(format!("{length}x{width}x{height}")))
    }
}

fn pick(values: &[&str], rng: &mut dyn rand::RngCore) -> String {
    let idx = rng.gen_range(0..values.len());
    values[idx].to_string()
}
