use dotenv::dotenv;
use serde::Deserialize;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub headless: bool,
    pub username: String,
    pub password: String,
    pub have_booking: bool,
    pub element_timeout: u64,
    pub retries: u64,
    pub scrape_refresh_time_min: u64,
    pub proxy_path: String,
    pub parallel_browsers: usize,
    pub scraping_enabled: bool,
    #[serde(default)]
    pub webhook_url: Option<String>,
}

impl Settings {
    pub fn from_yaml<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();

        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut settings: Settings = serde_yaml::from_str(&contents)?;

        settings.username = parse_env_var(&settings.username)?;
        settings.password = parse_env_var(&settings.password)?;
        settings.proxy_path = parse_env_var(&settings.proxy_path)?;
        
        if let Some(ref webhook_url) = settings.webhook_url {
            settings.webhook_url = Some(parse_env_var(webhook_url)?);
        }

        Ok(settings)
    }

    pub fn read_proxies(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let proxy_contents = std::fs::read_to_string(&self.proxy_path)
            .map_err(|e| format!("Failed to read proxy file '{}': {}", self.proxy_path, e))?;
        let proxies: Vec<String> = proxy_contents
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        Ok(proxies)
    }

    pub fn get_proxies_from_index(&self, start_index: usize, count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let all_proxies = self.read_proxies()?;
        if all_proxies.is_empty() {
            return Ok(vec![]);
        }
        
        let len = all_proxies.len();
        let mut result = Vec::with_capacity(count.min(len));
        
        for i in 0..count.min(len) {
            let idx = (start_index + i) % len;
            result.push(all_proxies[idx].clone());
        }
        
        Ok(result)
    }
}

fn parse_env_var(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    if value.starts_with("${") && value.ends_with("}") {
        let env_name = &value[2..value.len() - 1];
        match env::var(env_name) {
            Ok(val) => Ok(val),
            Err(_) => Err(format!("Environment variable '{}' not found", env_name).into()),
        }
    } else {
        Ok(value.to_string())
    }
}
