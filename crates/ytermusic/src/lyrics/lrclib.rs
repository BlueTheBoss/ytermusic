use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct LyricLine {
    pub timestamp: f64,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct LrcLibResponse {
    #[serde(default)]
    synced_lyrics: Option<String>,
    #[serde(default)]
    plain_lyrics: Option<String>,
}

pub async fn fetch_lyrics(
    artist: &str,
    title: &str,
    album: &str,
    duration: f64,
) -> Result<Vec<LyricLine>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}&album_name={}&duration={}",
        urlencoding::encode(artist),
        urlencoding::encode(title),
        urlencoding::encode(album),
        duration as u64,
    );

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<LrcLibResponse>().await {
                Ok(data) => {
                    if let Some(synced) = data.synced_lyrics {
                        return Ok(parse_lrc(&synced));
                    }
                    if let Some(plain) = data.plain_lyrics {
                        return Ok(parse_plain_lyrics(&plain));
                    }
                    Err("No lyrics found".to_string())
                }
                Err(e) => Err(format!("Failed to parse lyrics: {e}")),
            }
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            Err("Lyrics not found".to_string())
        }
        Ok(resp) => Err(format!("LrcLib error: {}", resp.status())),
        Err(e) => Err(format!("LrcLib request failed: {e}")),
    }
}

fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for line in lrc.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(parsed) = parse_lrc_line(line) {
            lines.push(parsed);
        }
    }
    lines.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

fn parse_lrc_line(line: &str) -> Option<LyricLine> {
    let line = line.trim_start_matches('[');
    let (time_str, text) = line.split_once(']')?;
    let timestamp = parse_lrc_timestamp(time_str)?;
    Some(LyricLine {
        timestamp,
        text: text.trim().to_string(),
    })
}

fn parse_lrc_timestamp(ts: &str) -> Option<f64> {
    let (minutes, rest) = ts.split_once(':')?;
    let (seconds, _milliseconds) = rest.split_once('.').unwrap_or((rest, "0"));
    let minutes: f64 = minutes.parse().ok()?;
    let seconds: f64 = seconds.parse().ok()?;
    Some(minutes * 60.0 + seconds)
}

fn parse_plain_lyrics(text: &str) -> Vec<LyricLine> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, line)| LyricLine {
            timestamp: i as f64 * 5.0,
            text: line.trim().to_string(),
        })
        .collect()
}
