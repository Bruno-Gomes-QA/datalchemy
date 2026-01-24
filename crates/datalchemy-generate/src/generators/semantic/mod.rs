use crate::assets::assets_loader;
use crate::errors::GenerationError;
use crate::generators::{GeneratedValue, Generator, GeneratorContext, GeneratorRegistry};
use rand::Rng;
use serde_json::Value;

pub fn register(registry: &mut GeneratorRegistry) {
    registry.register_generator(Box::new(NameGenerator));
    registry.register_generator(Box::new(EmailSafeGenerator));
    registry.register_generator(Box::new(PhoneBrGenerator));
    registry.register_generator(Box::new(CpfGenerator));
    registry.register_generator(Box::new(CnpjGenerator));
    registry.register_generator(Box::new(RgGenerator));
    registry.register_generator(Box::new(CepGenerator));
    registry.register_generator(Box::new(UfGenerator));
    registry.register_generator(Box::new(CityGenerator));
    registry.register_generator(Box::new(AddressGenerator));
    registry.register_generator(Box::new(MoneyBrlGenerator));
    registry.register_generator(Box::new(IpGenerator));
    registry.register_generator(Box::new(UrlGenerator));
}

struct NameGenerator;

impl Generator for NameGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.name"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.name"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let loader = assets_loader();
        let mut first_names = loader
            .load_lines("pt_BR/names.txt")
            .unwrap_or_else(|_| Vec::new());
        let mut last_names = loader
            .load_lines("pt_BR/surnames.txt")
            .unwrap_or_else(|_| Vec::new());

        if first_names.is_empty() {
            first_names = DEFAULT_FIRST_NAMES.iter().map(|s| s.to_string()).collect();
        }
        if last_names.is_empty() {
            last_names = DEFAULT_LAST_NAMES.iter().map(|s| s.to_string()).collect();
        }

        let first = pick(&first_names, rng).unwrap_or("Pessoa");
        let last = pick(&last_names, rng).unwrap_or("Teste");
        Ok(GeneratedValue::Text(format!("{first} {last}")))
    }
}

struct EmailSafeGenerator;

impl Generator for EmailSafeGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.email.safe"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.email"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let loader = assets_loader();
        let names = loader
            .load_lines("pt_BR/names.txt")
            .unwrap_or_else(|_| Vec::new());
        let base = if !names.is_empty() {
            let name = pick(&names, rng).unwrap_or("usuario");
            slugify(name)
        } else {
            format!("user{}", rng.gen_range(1..=9999))
        };
        Ok(GeneratedValue::Text(format!("{base}@example.com")))
    }
}

struct PhoneBrGenerator;

impl Generator for PhoneBrGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.phone"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.phone"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let ddd = DDD_CODES[rng.gen_range(0..DDD_CODES.len())];
        let prefix = rng.gen_range(90000..=99999);
        let suffix = rng.gen_range(0..=9999);
        Ok(GeneratedValue::Text(format!(
            "+55{ddd}{prefix:05}{suffix:04}"
        )))
    }
}

struct CpfGenerator;

impl Generator for CpfGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.cpf"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.cpf"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let mut digits = [0_u8; 11];
        for digit in digits.iter_mut().take(9) {
            *digit = rng.gen_range(0..=9);
        }
        let d1 = cpf_check_digit(&digits[..9]);
        let d2 = cpf_check_digit(&[&digits[..9], &[d1]].concat());
        digits[9] = d1;
        digits[10] = d2;
        let value = digits.iter().map(|d| char::from(b'0' + *d)).collect();
        Ok(GeneratedValue::Text(value))
    }
}

struct CnpjGenerator;

impl Generator for CnpjGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.cnpj"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.cnpj"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let mut digits = [0_u8; 14];
        for digit in digits.iter_mut().take(12) {
            *digit = rng.gen_range(0..=9);
        }
        let d1 = cnpj_check_digit(&digits[..12]);
        let d2 = cnpj_check_digit(&[&digits[..12], &[d1]].concat());
        digits[12] = d1;
        digits[13] = d2;
        let value = digits.iter().map(|d| char::from(b'0' + *d)).collect();
        Ok(GeneratedValue::Text(value))
    }
}

struct RgGenerator;

impl Generator for RgGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.rg"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.rg"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let value = format!("{:09}", rng.gen_range(0..=999_999_999));
        Ok(GeneratedValue::Text(value))
    }
}

struct CepGenerator;

impl Generator for CepGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.cep"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.location"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let value = format!("{:08}", rng.gen_range(0..=99_999_999));
        Ok(GeneratedValue::Text(value))
    }
}

struct UfGenerator;

impl Generator for UfGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.uf"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.location"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let value = STATES[rng.gen_range(0..STATES.len())];
        Ok(GeneratedValue::Text(value.to_string()))
    }
}

struct CityGenerator;

impl Generator for CityGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.city"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.location"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let loader = assets_loader();
        let mut cities = loader
            .load_json_strings("pt_BR/cities.json")
            .unwrap_or_else(|_| Vec::new());
        if cities.is_empty() {
            cities = DEFAULT_CITIES.iter().map(|s| s.to_string()).collect();
        }
        let value = pick(&cities, rng).unwrap_or("Cidade");
        Ok(GeneratedValue::Text(value.to_string()))
    }
}

struct AddressGenerator;

impl Generator for AddressGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.address"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.address"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let loader = assets_loader();
        let mut streets = loader
            .load_lines("pt_BR/streets.txt")
            .unwrap_or_else(|_| Vec::new());
        if streets.is_empty() {
            streets = DEFAULT_STREETS.iter().map(|s| s.to_string()).collect();
        }
        let street = pick(&streets, rng).unwrap_or("Rua Central");
        let number = rng.gen_range(1..=9999);
        Ok(GeneratedValue::Text(format!("{street}, {number}")))
    }
}

struct MoneyBrlGenerator;

impl Generator for MoneyBrlGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.money.brl"
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let min = params
            .and_then(|params| params.get("min"))
            .and_then(|value| value.as_f64())
            .unwrap_or(10.0);
        let max = params
            .and_then(|params| params.get("max"))
            .and_then(|value| value.as_f64())
            .unwrap_or(10000.0);
        if min > max {
            return Err(GenerationError::InvalidPlan(
                "semantic.br.money.brl min must be <= max".to_string(),
            ));
        }
        let value = rng.gen_range(min..=max);
        let rounded = (value * 100.0).round() / 100.0;
        Ok(GeneratedValue::Float(rounded))
    }
}

struct IpGenerator;

impl Generator for IpGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.ip"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.network"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let mut octet = || rng.gen_range(1..=254);
        Ok(GeneratedValue::Text(format!(
            "{}.{}.{}.{}",
            octet(),
            octet(),
            octet(),
            octet()
        )))
    }
}

struct UrlGenerator;

impl Generator for UrlGenerator {
    fn id(&self) -> &'static str {
        "semantic.br.url"
    }

    fn pii_tags(&self) -> &'static [&'static str] {
        &["pii.network"]
    }

    fn generate(
        &self,
        _ctx: &mut GeneratorContext<'_>,
        _params: Option<&Value>,
        rng: &mut dyn rand::RngCore,
    ) -> Result<GeneratedValue, GenerationError> {
        let slug = format!("pagina-{}", rng.gen_range(1..=9999));
        Ok(GeneratedValue::Text(format!("https://example.com/{slug}")))
    }
}

fn pick<'a>(values: &'a [String], rng: &mut dyn rand::RngCore) -> Option<&'a str> {
    if values.is_empty() {
        return None;
    }
    let idx = rng.gen_range(0..values.len());
    values.get(idx).map(String::as_str)
}

fn cpf_check_digit(digits: &[u8]) -> u8 {
    let mut sum = 0_u32;
    let mut weight = digits.len() as u32 + 1;
    for digit in digits {
        sum += (*digit as u32) * weight;
        weight = weight.saturating_sub(1);
    }
    let remainder = sum % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

fn cnpj_check_digit(digits: &[u8]) -> u8 {
    let weights = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let offset = weights.len().saturating_sub(digits.len());
    let mut sum = 0_u32;
    for (idx, digit) in digits.iter().enumerate() {
        sum += (*digit as u32) * weights[idx + offset];
    }
    let remainder = sum % 11;
    if remainder < 2 {
        0
    } else {
        (11 - remainder) as u8
    }
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

const DEFAULT_FIRST_NAMES: &[&str] = &[
    "Ana", "Bruno", "Carlos", "Daniela", "Eduardo", "Fernanda", "Gustavo", "Helena",
];

const DEFAULT_LAST_NAMES: &[&str] = &[
    "Silva", "Santos", "Oliveira", "Souza", "Lima", "Costa", "Ribeiro", "Almeida",
];

const DEFAULT_CITIES: &[&str] = &[
    "Sao Paulo",
    "Rio de Janeiro",
    "Belo Horizonte",
    "Porto Alegre",
    "Curitiba",
    "Salvador",
    "Fortaleza",
    "Recife",
];

const DEFAULT_STREETS: &[&str] = &[
    "Rua das Flores",
    "Avenida Central",
    "Rua do Comercio",
    "Avenida Paulista",
    "Rua da Praia",
];

const STATES: &[&str] = &[
    "AC", "AL", "AP", "AM", "BA", "CE", "DF", "ES", "GO", "MA", "MT", "MS", "MG", "PA", "PB", "PR",
    "PE", "PI", "RJ", "RN", "RS", "RO", "RR", "SC", "SP", "SE", "TO",
];

const DDD_CODES: &[&str] = &["11", "21", "31", "41", "51", "61", "71", "81", "91"];
