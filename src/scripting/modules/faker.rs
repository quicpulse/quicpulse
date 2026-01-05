//! Faker module for generating test data
//!
//! Provides functions to generate realistic fake data for testing APIs.

use rune::alloc::String as RuneString;
use rune::{ContextError, Module};
use fake::faker::name::en::*;
use fake::faker::internet::en::*;
use fake::faker::address::en::*;
use fake::faker::phone_number::en::*;
use fake::faker::company::en::*;
use fake::faker::lorem::en::*;
use fake::faker::creditcard::en::*;
use fake::faker::filesystem::en::*;
use fake::faker::boolean::en::*;
use fake::Fake;

/// Create the faker module
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("faker")?;

    // Name generators
    module.function("name", name).build()?;
    module.function("first_name", first_name).build()?;
    module.function("last_name", last_name).build()?;
    module.function("name_with_title", name_with_title).build()?;
    module.function("title", title).build()?;
    module.function("suffix", suffix).build()?;

    // Internet generators
    module.function("email", email).build()?;
    module.function("safe_email", safe_email).build()?;
    module.function("free_email", free_email).build()?;
    module.function("username", username).build()?;
    module.function("password", password).build()?;
    module.function("password_range", password_range).build()?;
    module.function("domain", domain).build()?;
    module.function("ipv4", ipv4).build()?;
    module.function("ipv6", ipv6).build()?;
    module.function("mac_address", mac_address).build()?;
    module.function("user_agent", user_agent).build()?;

    // Address generators
    module.function("city", city).build()?;
    module.function("street_name", street_name).build()?;
    module.function("street_address", street_address).build()?;
    module.function("zip_code", zip_code).build()?;
    module.function("state", state).build()?;
    module.function("state_abbr", state_abbr).build()?;
    module.function("country", country).build()?;
    module.function("country_code", country_code).build()?;
    module.function("latitude", latitude).build()?;
    module.function("longitude", longitude).build()?;

    // Phone generators
    module.function("phone_number", phone_number).build()?;
    module.function("cell_number", cell_number).build()?;

    // Company generators
    module.function("company_name", company_name).build()?;
    module.function("company_suffix", company_suffix).build()?;
    module.function("industry", industry).build()?;
    module.function("profession", profession).build()?;
    module.function("buzzword", buzzword).build()?;
    module.function("catch_phrase", catch_phrase).build()?;

    // Lorem ipsum generators
    module.function("word", word).build()?;
    module.function("words", words).build()?;
    module.function("sentence", sentence).build()?;
    module.function("sentences", sentences).build()?;
    module.function("paragraph", paragraph).build()?;
    module.function("paragraphs", paragraphs).build()?;

    // Credit card generators
    module.function("credit_card_number", credit_card_number).build()?;

    // Filesystem generators
    module.function("file_name", file_name).build()?;
    module.function("file_path", file_path).build()?;
    module.function("file_extension", file_extension).build()?;
    module.function("mime_type", dir_path).build()?;

    // Boolean generator
    module.function("bool", bool_val).build()?;
    module.function("bool_ratio", bool_ratio).build()?;

    // Number generators (using rand)
    module.function("number", random_number).build()?;
    module.function("number_range", random_number_range).build()?;
    module.function("float", random_float).build()?;
    module.function("float_range", random_float_range).build()?;

    Ok(module)
}

// Name generators
fn name() -> RuneString {
    let n: String = Name().fake();
    RuneString::try_from(n).unwrap_or_default()
}

fn first_name() -> RuneString {
    let n: String = FirstName().fake();
    RuneString::try_from(n).unwrap_or_default()
}

fn last_name() -> RuneString {
    let n: String = LastName().fake();
    RuneString::try_from(n).unwrap_or_default()
}

fn name_with_title() -> RuneString {
    let n: String = NameWithTitle().fake();
    RuneString::try_from(n).unwrap_or_default()
}

fn title() -> RuneString {
    let n: String = Title().fake();
    RuneString::try_from(n).unwrap_or_default()
}

fn suffix() -> RuneString {
    let n: String = Suffix().fake();
    RuneString::try_from(n).unwrap_or_default()
}

// Internet generators
fn email() -> RuneString {
    let e: String = FreeEmail().fake();
    RuneString::try_from(e).unwrap_or_default()
}

fn safe_email() -> RuneString {
    let e: String = SafeEmail().fake();
    RuneString::try_from(e).unwrap_or_default()
}

fn free_email() -> RuneString {
    let e: String = FreeEmail().fake();
    RuneString::try_from(e).unwrap_or_default()
}

fn username() -> RuneString {
    let u: String = Username().fake();
    RuneString::try_from(u).unwrap_or_default()
}

fn password() -> RuneString {
    let p: String = Password(8..20).fake();
    RuneString::try_from(p).unwrap_or_default()
}

fn password_range(min: i64, max: i64) -> RuneString {
    let min = min.max(1) as usize;
    let max = max.max(min as i64 + 1) as usize;
    let p: String = Password(min..max).fake();
    RuneString::try_from(p).unwrap_or_default()
}

fn domain() -> RuneString {
    let d: String = DomainSuffix().fake();
    RuneString::try_from(d).unwrap_or_default()
}

fn ipv4() -> RuneString {
    let ip: String = IPv4().fake();
    RuneString::try_from(ip).unwrap_or_default()
}

fn ipv6() -> RuneString {
    let ip: String = IPv6().fake();
    RuneString::try_from(ip).unwrap_or_default()
}

fn mac_address() -> RuneString {
    let mac: String = MACAddress().fake();
    RuneString::try_from(mac).unwrap_or_default()
}

fn user_agent() -> RuneString {
    let ua: String = UserAgent().fake();
    RuneString::try_from(ua).unwrap_or_default()
}

// Address generators
fn city() -> RuneString {
    let c: String = CityName().fake();
    RuneString::try_from(c).unwrap_or_default()
}

fn street_name() -> RuneString {
    let s: String = StreetName().fake();
    RuneString::try_from(s).unwrap_or_default()
}

fn street_address() -> RuneString {
    let s: String = StreetName().fake();
    let n: u32 = (1..9999u32).fake();
    let addr = format!("{} {}", n, s);
    RuneString::try_from(addr).unwrap_or_default()
}

fn zip_code() -> RuneString {
    let z: String = ZipCode().fake();
    RuneString::try_from(z).unwrap_or_default()
}

fn state() -> RuneString {
    let s: String = StateName().fake();
    RuneString::try_from(s).unwrap_or_default()
}

fn state_abbr() -> RuneString {
    let s: String = StateAbbr().fake();
    RuneString::try_from(s).unwrap_or_default()
}

fn country() -> RuneString {
    let c: String = CountryName().fake();
    RuneString::try_from(c).unwrap_or_default()
}

fn country_code() -> RuneString {
    let c: String = CountryCode().fake();
    RuneString::try_from(c).unwrap_or_default()
}

fn latitude() -> f64 {
    Latitude().fake()
}

fn longitude() -> f64 {
    Longitude().fake()
}

// Phone generators
fn phone_number() -> RuneString {
    let p: String = PhoneNumber().fake();
    RuneString::try_from(p).unwrap_or_default()
}

fn cell_number() -> RuneString {
    let p: String = CellNumber().fake();
    RuneString::try_from(p).unwrap_or_default()
}

// Company generators
fn company_name() -> RuneString {
    let c: String = CompanyName().fake();
    RuneString::try_from(c).unwrap_or_default()
}

fn company_suffix() -> RuneString {
    let c: String = CompanySuffix().fake();
    RuneString::try_from(c).unwrap_or_default()
}

fn industry() -> RuneString {
    let i: String = Industry().fake();
    RuneString::try_from(i).unwrap_or_default()
}

fn profession() -> RuneString {
    let p: String = Profession().fake();
    RuneString::try_from(p).unwrap_or_default()
}

fn buzzword() -> RuneString {
    let b: String = Buzzword().fake();
    RuneString::try_from(b).unwrap_or_default()
}

fn catch_phrase() -> RuneString {
    let c: String = CatchPhrase().fake();
    RuneString::try_from(c).unwrap_or_default()
}

// Lorem ipsum generators
fn word() -> RuneString {
    let w: String = Word().fake();
    RuneString::try_from(w).unwrap_or_default()
}

fn words() -> RuneString {
    let w: Vec<String> = Words(3..8).fake();
    RuneString::try_from(w.join(" ")).unwrap_or_default()
}

fn sentence() -> RuneString {
    let s: String = Sentence(5..12).fake();
    RuneString::try_from(s).unwrap_or_default()
}

fn sentences() -> RuneString {
    let s: Vec<String> = Sentences(2..5).fake();
    RuneString::try_from(s.join(" ")).unwrap_or_default()
}

fn paragraph() -> RuneString {
    let p: String = Paragraph(3..7).fake();
    RuneString::try_from(p).unwrap_or_default()
}

fn paragraphs() -> RuneString {
    let p: Vec<String> = Paragraphs(2..4).fake();
    RuneString::try_from(p.join("\n\n")).unwrap_or_default()
}

// Credit card generators
fn credit_card_number() -> RuneString {
    let c: String = CreditCardNumber().fake();
    RuneString::try_from(c).unwrap_or_default()
}

// Filesystem generators
fn file_name() -> RuneString {
    let f: String = FileName().fake();
    RuneString::try_from(f).unwrap_or_default()
}

fn file_path() -> RuneString {
    let f: String = FilePath().fake();
    RuneString::try_from(f).unwrap_or_default()
}

fn file_extension() -> RuneString {
    let f: String = FileExtension().fake();
    RuneString::try_from(f).unwrap_or_default()
}

fn dir_path() -> RuneString {
    let d: String = DirPath().fake();
    RuneString::try_from(d).unwrap_or_default()
}

// Boolean generators
fn bool_val() -> bool {
    Boolean(50).fake()
}

fn bool_ratio(ratio: i64) -> bool {
    let ratio = ratio.clamp(0, 100) as u8;
    Boolean(ratio).fake()
}

// Number generators
fn random_number() -> i64 {
    rand::random::<i32>() as i64
}

fn random_number_range(min: i64, max: i64) -> i64 {
    use rand::Rng;
    let mut rng = rand::rng();
    rng.random_range(min..=max)
}

fn random_float() -> f64 {
    rand::random::<f64>()
}

fn random_float_range(min: f64, max: f64) -> f64 {
    use rand::Rng;
    let mut rng = rand::rng();
    rng.random_range(min..=max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_generation() {
        let n = name();
        assert!(!n.is_empty());
    }

    #[test]
    fn test_email_generation() {
        let e = email();
        assert!(e.contains('@'));
    }

    #[test]
    fn test_number_range() {
        for _ in 0..100 {
            let n = random_number_range(1, 10);
            assert!(n >= 1 && n <= 10);
        }
    }
}
