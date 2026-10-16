use super::{now, Core};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DictionaryResult {
    pub word: String,
    pub entries: serde_json::Value,
    pub cached: bool,
    pub saved: bool,
}
impl Core {
    pub async fn dictionary_lookup(
        &self,
        word: String,
        refresh: bool,
    ) -> Result<DictionaryResult, String> {
        let word = word.trim().to_lowercase();
        if word.is_empty()
            || word.len() > 64
            || !word
                .chars()
                .all(|c| c.is_ascii_alphabetic() || ['-', '\'', ' '].contains(&c))
        {
            return Err("Enter an English word or short phrase (up to 64 characters)".into());
        }
        let cached: Option<(String, bool)> = {
            self.0
                .db
                .lock()
                .map_err(|_| "Database unavailable")?
                .query_row(
                    "SELECT entries,saved FROM dictionary WHERE word=?1",
                    [&word],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?
        };
        if !refresh {
            if let Some((entries, saved)) = &cached {
                return Ok(DictionaryResult {
                    word,
                    entries: serde_json::from_str(entries).map_err(|e| e.to_string())?,
                    cached: true,
                    saved: *saved,
                });
            }
        }
        let mut url = reqwest::Url::parse("https://api.dictionaryapi.dev/api/v2/entries/en/")
            .map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "Invalid dictionary URL")?
            .pop_if_empty()
            .push(&word);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .map_err(|e| e.to_string())?;
        let response = client.get(url).send().await.map_err(|_| {
            "Could not reach the dictionary. Previously looked-up words work offline."
        })?;
        if response.status() == 404 {
            return Err(format!(
                "No definition found for “{word}”. Check the spelling or try another word."
            ));
        }
        let entries: serde_json::Value = response
            .error_for_status()
            .map_err(|e| format!("Dictionary service unavailable: {e}"))?
            .json()
            .await
            .map_err(|_| "Dictionary returned an unreadable response")?;
        if !entries.is_array() {
            return Err("Dictionary returned an unexpected response".into());
        }
        let saved = cached.is_some_and(|(_, saved)| saved);
        self.0.db.lock().map_err(|_|"Database unavailable")?.execute("INSERT INTO dictionary(word,entries,saved,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(word) DO UPDATE SET entries=excluded.entries,updated_at=excluded.updated_at",params![word,entries.to_string(),saved,now()]).map_err(|e|e.to_string())?;
        Ok(DictionaryResult {
            word,
            entries,
            cached: false,
            saved,
        })
    }
    pub fn dictionary_save(&self, word: &str, saved: bool) -> Result<(), String> {
        self.0
            .db
            .lock()
            .map_err(|_| "Database unavailable")?
            .execute(
                "UPDATE dictionary SET saved=?1 WHERE word=?2",
                params![saved, word],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn vocabulary(&self) -> Result<Vec<String>, String> {
        let db = self.0.db.lock().map_err(|_| "Database unavailable")?;
        let mut q = db
            .prepare("SELECT word FROM dictionary WHERE saved=1 ORDER BY word")
            .map_err(|e| e.to_string())?;
        let values = q
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(values)
    }
}
