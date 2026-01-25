use rand::Rng;
use serde_json::Value;

use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(TransactionTypeGenerator));
    registry.register_generator(Box::new(PaymentMethodGenerator));
    registry.register_generator(Box::new(InvoiceStatusGenerator));
    registry.register_generator(Box::new(InstallmentsGenerator));
}

struct TransactionTypeGenerator;

impl Generator for TransactionTypeGenerator {
    fn id(&self) -> &'static str {
        "domain.finance.transaction_type"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["debito", "credito", "pix"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct PaymentMethodGenerator;

impl Generator for PaymentMethodGenerator {
    fn id(&self) -> &'static str {
        "domain.finance.payment_method"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = [
            "cartao_credito",
            "cartao_debito",
            "pix",
            "boleto",
            "transferencia",
        ];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct InvoiceStatusGenerator;

impl Generator for InvoiceStatusGenerator {
    fn id(&self) -> &'static str {
        "domain.finance.invoice_status"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let values = ["aberta", "paga", "cancelada"];
        Ok(GeneratedValue::Text(pick(&values, rng)))
    }
}

struct InstallmentsGenerator;

impl Generator for InstallmentsGenerator {
    fn id(&self) -> &'static str {
        "domain.finance.installments"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = get_i64(params, "min").unwrap_or(1);
        let max = get_i64(params, "max").unwrap_or(12);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "domain.finance.installments min must be <= max".to_string(),
            ));
        }
        let value = rng.gen_range(min..=max);
        Ok(GeneratedValue::Int(value))
    }
}

fn pick(values: &[&str], rng: &mut dyn rand::RngCore) -> String {
    let idx = rng.gen_range(0..values.len());
    values[idx].to_string()
}

fn get_i64(params: Option<&Value>, key: &str) -> Option<i64> {
    params
        .and_then(|params| params.get(key))
        .and_then(|value| value.as_i64())
}
