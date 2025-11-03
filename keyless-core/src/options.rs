//! Centralized selectable options
//!
//! Curated lists of Whisper model identifiers and language codes for UI selection.
//! These constants ensure that input handling and settings hydration agree on the
//! same indices. A future model manager can replace these.
//!
//! These are application constants shared across frontends (TUI, future Tauri, etc.).

/// Curated list of popular language codes for TUI display.
///
/// Whisper supports 99 languages total. This is a curated subset of the most
/// commonly used languages for UI simplicity. Users can manually specify any
/// of the 99 codes via CLI/ENV.
pub const LANG_CODES: &[&str] = &[
    "en", "es", "fr", "de", "it", "pt", // Western Europe
    "zh", "ja", "ko", // East Asia
    "hi", "ar", "ru", "tr", // High‑usage global
    "nl", "pl", "sv", "no", "da", "fi", // Nordics/NE
];

/// Returns the full language name for a given language code.
/// Falls back to the code itself if not found.
pub fn lang_name(code: &str) -> &str {
    match code {
        "en" => "English",
        "fr" => "French",
        "es" => "Spanish",
        "de" => "German",
        "it" => "Italian",
        "pt" => "Portuguese",
        "nl" => "Dutch",
        "sv" => "Swedish",
        "da" => "Danish",
        "no" => "Norwegian",
        "fi" => "Finnish",
        "pl" => "Polish",
        "cs" => "Czech",
        "ru" => "Russian",
        "uk" => "Ukrainian",
        "tr" => "Turkish",
        "ar" => "Arabic",
        "fa" => "Persian",
        "hi" => "Hindi",
        "bn" => "Bengali",
        "ur" => "Urdu",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "vi" => "Vietnamese",
        "id" => "Indonesian",
        "th" => "Thai",
        "ms" => "Malay",
        "tl" => "Tagalog",
        "el" => "Greek",
        "ro" => "Romanian",
        "hu" => "Hungarian",
        "sk" => "Slovak",
        "sl" => "Slovenian",
        "bg" => "Bulgarian",
        "sr" => "Serbian",
        "hr" => "Croatian",
        "lt" => "Lithuanian",
        "lv" => "Latvian",
        "et" => "Estonian",
        "is" => "Icelandic",
        "ga" => "Irish",
        "af" => "Afrikaans",
        "sw" => "Swahili",
        "am" => "Amharic",
        "ta" => "Tamil",
        "te" => "Telugu",
        "kn" => "Kannada",
        "ml" => "Malayalam",
        "mr" => "Marathi",
        "gu" => "Gujarati",
        "pa" => "Punjabi",
        "ne" => "Nepali",
        "si" => "Sinhala",
        "km" => "Khmer",
        "lo" => "Lao",
        "my" => "Burmese",
        _ => code,
    }
}

/// Returns true if the model identifier indicates an English-only Whisper model.
/// We conservatively match the known ".en" suffix used by official model IDs.
pub fn is_english_only_model(model_id: &str) -> bool {
    model_id.ends_with(".en")
}
