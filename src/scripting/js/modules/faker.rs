//! Faker module for JavaScript
//!
//! Provides fake data generation utilities.

use rquickjs::{Ctx, Object, Function};
use rand::Rng;
use crate::errors::QuicpulseError;

pub fn register(ctx: &Ctx<'_>) -> Result<(), QuicpulseError> {
    let globals = ctx.globals();
    let faker = Object::new(ctx.clone())
        .map_err(|e| QuicpulseError::Script(format!("Failed to create faker object: {}", e)))?;

    faker.set("name", Function::new(ctx.clone(), fake_name)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("first_name", Function::new(ctx.clone(), fake_first_name)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("last_name", Function::new(ctx.clone(), fake_last_name)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("email", Function::new(ctx.clone(), fake_email)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("username", Function::new(ctx.clone(), fake_username)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("phone", Function::new(ctx.clone(), fake_phone)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("address", Function::new(ctx.clone(), fake_address)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("city", Function::new(ctx.clone(), fake_city)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("country", Function::new(ctx.clone(), fake_country)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("company", Function::new(ctx.clone(), fake_company)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("sentence", Function::new(ctx.clone(), fake_sentence)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("paragraph", Function::new(ctx.clone(), fake_paragraph)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("word", Function::new(ctx.clone(), fake_word)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("integer", Function::new(ctx.clone(), fake_integer)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("float", Function::new(ctx.clone(), fake_float)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("boolean", Function::new(ctx.clone(), fake_boolean)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("uuid", Function::new(ctx.clone(), fake_uuid)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("ipv4", Function::new(ctx.clone(), fake_ipv4)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("ipv6", Function::new(ctx.clone(), fake_ipv6)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("url", Function::new(ctx.clone(), fake_url)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;
    faker.set("user_agent", Function::new(ctx.clone(), fake_user_agent)?)
        .map_err(|e| QuicpulseError::Script(e.to_string()))?;

    globals.set("faker", faker)
        .map_err(|e| QuicpulseError::Script(format!("Failed to set faker global: {}", e)))?;

    Ok(())
}

const FIRST_NAMES: &[&str] = &[
    "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph",
    "Mary", "Patricia", "Jennifer", "Linda", "Elizabeth", "Barbara", "Susan", "Jessica",
    "Thomas", "Charles", "Christopher", "Daniel", "Matthew", "Anthony", "Mark", "Donald",
    "Sarah", "Karen", "Nancy", "Lisa", "Betty", "Margaret", "Sandra", "Ashley",
];

const LAST_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
    "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas",
    "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez", "Thompson", "White",
];

const DOMAINS: &[&str] = &[
    "gmail.com", "yahoo.com", "hotmail.com", "outlook.com", "example.com", "test.com",
];

const CITIES: &[&str] = &[
    "New York", "Los Angeles", "Chicago", "Houston", "Phoenix", "Philadelphia",
    "San Antonio", "San Diego", "Dallas", "San Jose", "Austin", "Jacksonville",
    "London", "Paris", "Tokyo", "Sydney", "Berlin", "Madrid", "Rome", "Toronto",
];

const COUNTRIES: &[&str] = &[
    "United States", "United Kingdom", "Canada", "Australia", "Germany", "France",
    "Japan", "Italy", "Spain", "Brazil", "Mexico", "India", "China", "Russia",
];

const COMPANIES: &[&str] = &[
    "Acme Corp", "Globex", "Initech", "Umbrella Corp", "Stark Industries",
    "Wayne Enterprises", "Oscorp", "LexCorp", "Cyberdyne Systems", "Weyland-Yutani",
];

const WORDS: &[&str] = &[
    "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
    "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
    "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis", "nostrud",
];

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
];

fn pick_random<T: Copy>(items: &[T]) -> T {
    let mut rng = rand::rng();
    items[rng.random_range(0..items.len())]
}

fn fake_first_name() -> String {
    pick_random(FIRST_NAMES).to_string()
}

fn fake_last_name() -> String {
    pick_random(LAST_NAMES).to_string()
}

fn fake_name() -> String {
    format!("{} {}", fake_first_name(), fake_last_name())
}

fn fake_email() -> String {
    let first = fake_first_name().to_lowercase();
    let last = fake_last_name().to_lowercase();
    let domain = pick_random(DOMAINS);
    let mut rng = rand::rng();
    let num: u32 = rng.random_range(1..1000);
    format!("{}.{}{}", first, last, num).replace(' ', "") + "@" + domain
}

fn fake_username() -> String {
    let first = fake_first_name().to_lowercase();
    let mut rng = rand::rng();
    let num: u32 = rng.random_range(1..10000);
    format!("{}{}", first, num)
}

fn fake_phone() -> String {
    let mut rng = rand::rng();
    format!(
        "+1-{}-{}-{}",
        rng.random_range(200..999),
        rng.random_range(200..999),
        rng.random_range(1000..9999)
    )
}

fn fake_address() -> String {
    let mut rng = rand::rng();
    let num: u32 = rng.random_range(1..9999);
    let streets = ["Main St", "Oak Ave", "Maple Dr", "Cedar Ln", "Pine Rd", "Elm St"];
    format!("{} {}", num, pick_random(&streets))
}

fn fake_city() -> String {
    pick_random(CITIES).to_string()
}

fn fake_country() -> String {
    pick_random(COUNTRIES).to_string()
}

fn fake_company() -> String {
    pick_random(COMPANIES).to_string()
}

fn fake_word() -> String {
    pick_random(WORDS).to_string()
}

fn fake_sentence() -> String {
    let mut rng = rand::rng();
    let word_count = rng.random_range(5..12);
    let words: Vec<String> = (0..word_count).map(|_| fake_word()).collect();
    let mut sentence = words.join(" ");
    if let Some(first) = sentence.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    sentence + "."
}

fn fake_paragraph() -> String {
    let mut rng = rand::rng();
    let sentence_count = rng.random_range(3..6);
    let sentences: Vec<String> = (0..sentence_count).map(|_| fake_sentence()).collect();
    sentences.join(" ")
}

fn fake_integer(min: Option<i64>, max: Option<i64>) -> i64 {
    let mut rng = rand::rng();
    let min_val = min.unwrap_or(0);
    let max_val = max.unwrap_or(100);
    rng.random_range(min_val..=max_val)
}

fn fake_float(min: Option<f64>, max: Option<f64>) -> f64 {
    let mut rng = rand::rng();
    let min_val = min.unwrap_or(0.0);
    let max_val = max.unwrap_or(1.0);
    rng.random_range(min_val..max_val)
}

fn fake_boolean() -> bool {
    let mut rng = rand::rng();
    rng.random_bool(0.5)
}

fn fake_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn fake_ipv4() -> String {
    let mut rng = rand::rng();
    format!(
        "{}.{}.{}.{}",
        rng.random_range(1..255),
        rng.random_range(0..255),
        rng.random_range(0..255),
        rng.random_range(1..255)
    )
}

fn fake_ipv6() -> String {
    let mut rng = rand::rng();
    let parts: Vec<String> = (0..8)
        .map(|_| format!("{:04x}", rng.random_range(0..0xFFFFu32)))
        .collect();
    parts.join(":")
}

fn fake_url() -> String {
    let domain = pick_random(DOMAINS);
    let path = fake_word();
    format!("https://{}/{}", domain, path)
}

fn fake_user_agent() -> String {
    pick_random(USER_AGENTS).to_string()
}
