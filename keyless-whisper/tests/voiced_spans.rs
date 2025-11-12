use keyless_whisper::inference::voiced::{VoicedCfg, build_voiced_spans};

#[test]
fn single_voiced_region_with_padding() {
    let mut pcm = vec![0.0f32; 48_000];
    for sample in pcm.iter_mut().take(20_000).skip(5_000) {
        *sample = 0.4;
    }
    let cfg = VoicedCfg {
        hop_ms: 25,
        open_db: -32.0,
        close_db: -38.0,
        min_speech_ms: 200,
        max_silence_ms: 700,
        pad_ms: 400,
    };

    let spans = build_voiced_spans(&pcm, 16_000, &cfg);
    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert!(span.start <= 5_000);
    assert!(span.end >= 20_000);
}

#[test]
fn splits_disjoint_regions() {
    let mut pcm = vec![0.0f32; 64_000];
    for sample in pcm.iter_mut().take(10_000).skip(2_000) {
        *sample = 0.5;
    }
    for sample in pcm.iter_mut().take(40_000).skip(30_000) {
        *sample = 0.6;
    }

    let spans = build_voiced_spans(&pcm, 16_000, &VoicedCfg::default());
    assert_eq!(spans.len(), 2);
    assert!(spans[0].end <= spans[1].start);
}
