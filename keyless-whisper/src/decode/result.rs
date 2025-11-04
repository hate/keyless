/// Result of decoding a single audio segment with quality metrics.
#[derive(Debug, Clone)]
pub struct DecodingResult {
    /// Decoded text.
    pub text: String,
    /// Average log probability across all tokens (quality indicator).
    pub avg_logprob: f64,
    /// Probability that the segment contains no speech (0.0-1.0).
    pub no_speech_prob: f64,
    /// Temperature used for sampling (0.0 = greedy).
    pub temperature: f64,
    /// Text compression ratio (higher = more repetitive/hallucinated).
    pub compression_ratio: f64,
}
