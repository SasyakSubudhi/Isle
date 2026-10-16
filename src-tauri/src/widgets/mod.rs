//! Small built-in widgets: weather (opt-in, uses the network) and a habit
//! tracker (local only).
use crate::core::{now, Core};
use serde::{Deserialize, Serialize};
use tauri::State;

// ---------- Habits ----------

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Habit {
    pub id: String,
    pub name: String,
    /// Local dates the habit was done, as YYYY-MM-DD.
    pub done: Vec<String>,
}

const MAX_HABITS: usize = 12;
const MAX_DAYS: usize = 400;

fn is_date(text: &str) -> bool {
    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok() && text.len() == 10
}

pub fn validate_habits(habits: &[Habit]) -> Result<(), String> {
    if habits.len() > MAX_HABITS {
        return Err(format!("Keep it to {MAX_HABITS} habits"));
    }
    for habit in habits {
        let name = habit.name.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err("Habit names must be 1–40 characters".into());
        }
        if habit.id.is_empty() || habit.id.len() > 64 {
            return Err("Invalid habit".into());
        }
        if habit.done.len() > MAX_DAYS || !habit.done.iter().all(|d| is_date(d)) {
            return Err("Invalid habit history".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn habits_list(core: State<'_, Core>) -> Result<Vec<Habit>, String> {
    Ok(core
        .kv_get::<Vec<Habit>>("widgets:habits")?
        .unwrap_or_default())
}

#[tauri::command]
pub fn habits_save(core: State<'_, Core>, habits: Vec<Habit>) -> Result<Vec<Habit>, String> {
    validate_habits(&habits)?;
    core.kv_set("widgets:habits", &habits)?;
    Ok(habits)
}

// ---------- Weather ----------

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Weather {
    pub place: String,
    pub temperature: f64,
    pub high: Option<f64>,
    pub low: Option<f64>,
    /// WMO weather interpretation code, mapped to words in the UI.
    pub code: i64,
    pub is_day: bool,
    pub fahrenheit: bool,
    pub fetched_at: i64,
    /// What the user typed, so a changed place invalidates the cache.
    pub query: String,
}

const CACHE_MS: i64 = 30 * 60 * 1000;

/// "12.97, 77.59" is used as coordinates directly, skipping the place lookup.
pub fn parse_coordinates(text: &str) -> Option<(f64, f64)> {
    let (lat, lon) = text.split_once(',')?;
    let (lat, lon) = (
        lat.trim().parse::<f64>().ok()?,
        lon.trim().parse::<f64>().ok()?,
    );
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}

async fn get_json(
    client: &reqwest::Client,
    url: reqwest::Url,
) -> Result<serde_json::Value, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "Weather is unreachable. Check your connection.".to_string())?;
    if !response.status().is_success() {
        return Err(format!("Weather service answered {}", response.status()));
    }
    response.json().await.map_err(|e| e.to_string())
}

async fn fetch(query: &str, fahrenheit: bool) -> Result<Weather, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let (lat, lon, place) = if let Some((lat, lon)) = parse_coordinates(query) {
        (lat, lon, format!("{lat:.2}, {lon:.2}"))
    } else {
        let mut url = reqwest::Url::parse("https://geocoding-api.open-meteo.com/v1/search")
            .map_err(|e| e.to_string())?;
        url.query_pairs_mut()
            .append_pair("name", query)
            .append_pair("count", "1")
            .append_pair("format", "json");
        let found = get_json(&client, url).await?;
        let first = found
            .get("results")
            .and_then(|r| r.get(0))
            .ok_or_else(|| format!("Couldn't find “{query}”. Try a nearby city."))?;
        let lat = first
            .get("latitude")
            .and_then(|v| v.as_f64())
            .ok_or("Bad place")?;
        let lon = first
            .get("longitude")
            .and_then(|v| v.as_f64())
            .ok_or("Bad place")?;
        let name = first.get("name").and_then(|v| v.as_str()).unwrap_or(query);
        (lat, lon, name.to_string())
    };
    let mut url =
        reqwest::Url::parse("https://api.open-meteo.com/v1/forecast").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("latitude", &format!("{lat:.3}"))
        .append_pair("longitude", &format!("{lon:.3}"))
        .append_pair("current", "temperature_2m,weather_code,is_day")
        .append_pair("daily", "temperature_2m_max,temperature_2m_min")
        .append_pair("forecast_days", "1")
        .append_pair("timezone", "auto")
        .append_pair(
            "temperature_unit",
            if fahrenheit { "fahrenheit" } else { "celsius" },
        );
    let body = get_json(&client, url).await?;
    let current = body.get("current").ok_or("Weather data missing")?;
    let number = |v: Option<&serde_json::Value>| v.and_then(|v| v.as_f64());
    let first = |key: &str| {
        number(
            body.get("daily")
                .and_then(|d| d.get(key))
                .and_then(|a| a.get(0)),
        )
    };
    Ok(Weather {
        place,
        temperature: number(current.get("temperature_2m")).ok_or("Weather data missing")?,
        code: current
            .get("weather_code")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        is_day: current.get("is_day").and_then(|v| v.as_i64()) != Some(0),
        high: first("temperature_2m_max"),
        low: first("temperature_2m_min"),
        fahrenheit,
        fetched_at: now(),
        query: query.to_string(),
    })
}

/// Weather for the place in Settings. Off unless the user turned it on; then
/// it sends that place name (or coordinates) to open-meteo.com, at most once
/// every 30 minutes unless `refresh` is set.
#[tauri::command]
pub async fn weather_get(core: State<'_, Core>, refresh: bool) -> Result<Weather, String> {
    let core = core.inner().clone();
    let settings = core.settings()?;
    if !settings.weather_enabled {
        return Err("Weather is off. Turn it on in Settings.".into());
    }
    let query = settings.weather_place.trim().to_string();
    if query.is_empty() {
        return Err("Add your city in Settings to see the weather.".into());
    }
    if !refresh {
        if let Some(cached) = core.kv_get::<Weather>("widgets:weather")? {
            if cached.query == query
                && cached.fahrenheit == settings.weather_fahrenheit
                && now() - cached.fetched_at < CACHE_MS
            {
                return Ok(cached);
            }
        }
    }
    let weather = fetch(&query, settings.weather_fahrenheit).await?;
    core.kv_set("widgets:weather", &weather)?;
    Ok(weather)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_are_recognised_only_when_valid() {
        assert_eq!(parse_coordinates("12.97, 77.59"), Some((12.97, 77.59)));
        assert_eq!(parse_coordinates("Bengaluru"), None);
        assert_eq!(parse_coordinates("100, 0"), None);
    }

    #[test]
    fn habits_are_bounded() {
        let ok = Habit {
            id: "a".into(),
            name: "Read".into(),
            done: vec!["2026-10-16".into()],
        };
        assert!(validate_habits(std::slice::from_ref(&ok)).is_ok());
        assert!(validate_habits(&[Habit {
            done: vec!["16/10/2026".into()],
            ..ok.clone()
        }])
        .is_err());
        assert!(validate_habits(&[Habit {
            name: " ".into(),
            ..ok.clone()
        }])
        .is_err());
        assert!(validate_habits(&vec![ok; MAX_HABITS + 1]).is_err());
    }
}
