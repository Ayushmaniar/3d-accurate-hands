//! Fingerspelling and the ASL sign clips: valid clips that `check` passes.

use handkit_core::check;
use handkit_core::clip::Clip;
use handkit_core::spell::{SpellOptions, spell};

fn problems(clip: &Clip) -> (usize, String) {
    let times: Vec<f64> = (0..clip.frames()).map(|i| i as f64 / clip.fps).collect();
    let frames: Vec<_> = times.iter().map(|t| (*t, clip.sample(*t).unwrap())).collect();
    let r = check::analyze(clip, &frames, clip.aspect()).unwrap();
    (r.problems, r.text())
}

#[test]
fn spells_a_word_cleanly() {
    let v = spell("HELLO", &SpellOptions::default()).unwrap();
    let clip = Clip::from_value(&v).unwrap();
    // rest, H, E (with the thumb's waypoint), L, a bounce and the second L, O, rest
    assert!(v["keys"].as_array().unwrap().len() >= 8);
    let (n, text) = problems(&clip);
    assert_eq!(n, 0, "{text}");
}

#[test]
fn every_letter_spells() {
    let v = spell("ABCDEFGHIJKLM NOPQRSTUVWXYZ", &SpellOptions { rate: 3.0, ..Default::default() }).unwrap();
    let clip = Clip::from_value(&v).unwrap();
    let (_, text) = problems(&clip);
    assert!(text.contains("penetrations: none"), "{text}");
}

#[test]
fn rejects_what_it_cannot_spell() {
    assert!(spell("R2D2", &SpellOptions::default()).is_err());
    assert!(spell("  ", &SpellOptions::default()).is_err());
    assert!(spell("HI", &SpellOptions { rate: 40.0, ..Default::default() }).is_err());
}

#[test]
fn sign_clips_check_clean() {
    for name in ["asl_hello", "asl_thank_you", "asl_i_love_you", "asl_yes", "asl_no", "asl_please"] {
        let path = format!("{}/../../clips/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let clip = Clip::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let (n, text) = problems(&clip);
        assert_eq!(n, 0, "{name}:\n{text}");
    }
}
