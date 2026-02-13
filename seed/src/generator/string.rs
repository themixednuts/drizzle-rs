use super::{Generator, RngCore, SqlValue};
use crate::datasets::{domains, locations, names};
use rand::Rng;

/// Generates random text strings of a given length range.
pub struct TextGen {
    pub min_len: usize,
    pub max_len: usize,
}

impl Generator for TextGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let len = rng.random_range(self.min_len..=self.max_len);
        let s: String = (0..len)
            .map(|_| {
                let idx = rng.random_range(0u8..26);
                (b'a' + idx) as char
            })
            .collect();
        SqlValue::Text(s)
    }
    fn name(&self) -> &'static str {
        "Text"
    }
}

/// Picks a random first name from the dataset.
pub struct FirstNameGen;

impl Generator for FirstNameGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let idx = rng.random_range(0..names::FIRST_NAMES.len());
        SqlValue::Text(names::FIRST_NAMES[idx].to_string())
    }
    fn name(&self) -> &'static str {
        "FirstName"
    }
}

/// Picks a random last name from the dataset.
pub struct LastNameGen;

impl Generator for LastNameGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let idx = rng.random_range(0..names::LAST_NAMES.len());
        SqlValue::Text(names::LAST_NAMES[idx].to_string())
    }
    fn name(&self) -> &'static str {
        "LastName"
    }
}

/// Generates a random full name (first + last).
pub struct FullNameGen;

impl Generator for FullNameGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let first = names::FIRST_NAMES[rng.random_range(0..names::FIRST_NAMES.len())];
        let last = names::LAST_NAMES[rng.random_range(0..names::LAST_NAMES.len())];
        SqlValue::Text(format!("{first} {last}"))
    }
    fn name(&self) -> &'static str {
        "FullName"
    }
}

/// Generates a random email address using first/last name and domains.
pub struct EmailGen;

impl Generator for EmailGen {
    fn generate(&self, rng: &mut dyn RngCore, index: usize) -> SqlValue {
        let first =
            names::FIRST_NAMES[rng.random_range(0..names::FIRST_NAMES.len())].to_lowercase();
        let last = names::LAST_NAMES[rng.random_range(0..names::LAST_NAMES.len())].to_lowercase();
        let domain = domains::EMAIL_DOMAINS[rng.random_range(0..domains::EMAIL_DOMAINS.len())];
        // Add index suffix for uniqueness
        SqlValue::Text(format!("{first}.{last}{index}@{domain}"))
    }
    fn name(&self) -> &'static str {
        "Email"
    }
}

/// Generates a random US-style phone number.
pub struct PhoneGen;

impl Generator for PhoneGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let area: u16 = rng.random_range(200..999);
        let exchange: u16 = rng.random_range(200..999);
        let subscriber: u16 = rng.random_range(1000..9999);
        SqlValue::Text(format!("({area}) {exchange}-{subscriber}"))
    }
    fn name(&self) -> &'static str {
        "Phone"
    }
}

/// Picks a random city.
pub struct CityGen;

impl Generator for CityGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let idx = rng.random_range(0..locations::CITIES.len());
        SqlValue::Text(locations::CITIES[idx].to_string())
    }
    fn name(&self) -> &'static str {
        "City"
    }
}

/// Picks a random country.
pub struct CountryGen;

impl Generator for CountryGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let idx = rng.random_range(0..locations::COUNTRIES.len());
        SqlValue::Text(locations::COUNTRIES[idx].to_string())
    }
    fn name(&self) -> &'static str {
        "Country"
    }
}

/// Generates a random street address.
pub struct AddressGen;

impl Generator for AddressGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let num: u16 = rng.random_range(1..9999);
        let first = names::FIRST_NAMES[rng.random_range(0..names::FIRST_NAMES.len())];
        let suffix =
            locations::STREET_SUFFIXES[rng.random_range(0..locations::STREET_SUFFIXES.len())];
        SqlValue::Text(format!("{num} {first} {suffix}"))
    }
    fn name(&self) -> &'static str {
        "Address"
    }
}

/// Picks a random job title.
pub struct JobTitleGen;

impl Generator for JobTitleGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let idx = rng.random_range(0..names::JOB_TITLES.len());
        SqlValue::Text(names::JOB_TITLES[idx].to_string())
    }
    fn name(&self) -> &'static str {
        "JobTitle"
    }
}

/// Generates a random company name.
pub struct CompanyGen;

impl Generator for CompanyGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let last = names::LAST_NAMES[rng.random_range(0..names::LAST_NAMES.len())];
        let suffix =
            domains::COMPANY_SUFFIXES[rng.random_range(0..domains::COMPANY_SUFFIXES.len())];
        SqlValue::Text(format!("{last} {suffix}"))
    }
    fn name(&self) -> &'static str {
        "Company"
    }
}

/// Generates lorem ipsum text with a given word count.
pub struct LoremGen {
    pub words: usize,
}

impl Generator for LoremGen {
    fn generate(&self, rng: &mut dyn RngCore, _index: usize) -> SqlValue {
        let text: Vec<&str> = (0..self.words)
            .map(|_| {
                let idx = rng.random_range(0..domains::LOREM_WORDS.len());
                domains::LOREM_WORDS[idx]
            })
            .collect();
        SqlValue::Text(text.join(" "))
    }
    fn name(&self) -> &'static str {
        "Lorem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn email_is_deterministic() {
        let g = EmailGen;
        let mut rng1 = StdRng::seed_from_u64(42);
        let mut rng2 = StdRng::seed_from_u64(42);
        assert_eq!(g.generate(&mut rng1, 0), g.generate(&mut rng2, 0));
    }

    #[test]
    fn email_contains_at_and_dot() {
        let g = EmailGen;
        let mut rng = StdRng::seed_from_u64(42);
        for i in 0..20 {
            match g.generate(&mut rng, i) {
                SqlValue::Text(s) => {
                    assert!(s.contains('@'), "email missing @: {}", s);
                    assert!(s.contains('.'), "email missing dot: {}", s);
                    // Index is appended for uniqueness
                    assert!(
                        s.contains(&i.to_string()),
                        "email missing index suffix: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn text_respects_length() {
        let g = TextGen {
            min_len: 5,
            max_len: 10,
        };
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..50 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(
                        (5..=10).contains(&s.len()),
                        "length out of range: {}",
                        s.len()
                    );
                    assert!(
                        s.chars().all(|c| c.is_ascii_lowercase()),
                        "non-lowercase: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn first_name_from_dataset() {
        let g = FirstNameGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(!s.is_empty());
                    assert!(
                        names::FIRST_NAMES.contains(&s.as_str()),
                        "name not in dataset: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn last_name_from_dataset() {
        let g = LastNameGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(!s.is_empty());
                    assert!(
                        names::LAST_NAMES.contains(&s.as_str()),
                        "name not in dataset: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn full_name_has_space() {
        let g = FullNameGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    let parts: Vec<&str> = s.split(' ').collect();
                    assert_eq!(parts.len(), 2, "full name should be 'first last': {}", s);
                    assert!(!parts[0].is_empty());
                    assert!(!parts[1].is_empty());
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn phone_format() {
        let g = PhoneGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    // Format: (NNN) NNN-NNNN
                    assert!(s.starts_with('('), "phone should start with '(': {}", s);
                    assert!(s.contains(") "), "phone missing ') ': {}", s);
                    assert!(s.contains('-'), "phone missing '-': {}", s);
                    assert_eq!(s.len(), 14, "phone wrong length: {}", s);
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn city_from_dataset() {
        let g = CityGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(
                        locations::CITIES.contains(&s.as_str()),
                        "city not in dataset: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn country_from_dataset() {
        let g = CountryGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(
                        locations::COUNTRIES.contains(&s.as_str()),
                        "country not in dataset: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn address_has_number_and_street() {
        let g = AddressGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    let parts: Vec<&str> = s.splitn(2, ' ').collect();
                    assert_eq!(parts.len(), 2, "address should have number + street: {}", s);
                    // First part should be a number
                    assert!(
                        parts[0].parse::<u16>().is_ok(),
                        "address number invalid: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn job_title_from_dataset() {
        let g = JobTitleGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(
                        names::JOB_TITLES.contains(&s.as_str()),
                        "job title not in dataset: {}",
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn company_has_suffix() {
        let g = CompanyGen;
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    assert!(s.contains(' '), "company should have name + suffix: {}", s);
                    let suffix = s.rsplit(' ').next().unwrap();
                    assert!(
                        domains::COMPANY_SUFFIXES.contains(&suffix),
                        "company suffix not in dataset: {} (from {})",
                        suffix,
                        s
                    );
                }
                _ => panic!("expected Text"),
            }
        }
    }

    #[test]
    fn lorem_has_correct_word_count() {
        let g = LoremGen { words: 7 };
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            match g.generate(&mut rng, 0) {
                SqlValue::Text(s) => {
                    let word_count = s.split(' ').count();
                    assert_eq!(word_count, 7, "expected 7 words, got {}: {}", word_count, s);
                }
                _ => panic!("expected Text"),
            }
        }
    }
}
