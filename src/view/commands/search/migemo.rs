use rustmigemo::migemo::{
    compact_dictionary::CompactDictionary, query::query_a_word, regex_generator::RegexOperator,
    romaji_processor::RomajiProcessor, ternary_regex_generator::TernaryRegexGenerator,
};
use std::sync::OnceLock;

pub(super) fn pattern(text: &str) -> String {
    static DICTIONARY: OnceLock<CompactDictionary> = OnceLock::new();
    static ROMAJI: OnceLock<RomajiProcessor> = OnceLock::new();
    let dictionary = DICTIONARY.get_or_init(|| {
        CompactDictionary::new(
            &include_bytes!("../../../../assets/migemo/migemo-compact-dict").to_vec(),
        )
    });
    let romaji = ROMAJI.get_or_init(RomajiProcessor::new);
    let reading = romaji.romaji_to_hiragana_predictively(&text.encode_utf16().collect::<Vec<_>>());
    let mut generator = TernaryRegexGenerator::new();
    // The BSD dictionary strips okurigana: ひら -> 開, not ひらく -> 開く.
    // Reattach the remaining kana for each exact stem, including incomplete romaji.
    for suffix in reading.suffixes {
        let mut kana = reading.prefix.clone();
        kana.extend(suffix);
        for split in 1..kana.len() {
            let stem = kana[..split].to_vec();
            for mut word in dictionary.search(&stem) {
                word.extend_from_slice(&kana[split..]);
                generator.add(&String::from_utf16_lossy(&word).chars().collect::<Vec<_>>());
            }
        }
    }
    let base = query_a_word(text, dictionary, &RegexOperator::Default);
    let okurigana = generator.generate(&RegexOperator::Default);
    if okurigana.is_empty() {
        base
    } else {
        format!("(?:{base}|{okurigana})")
    }
}
