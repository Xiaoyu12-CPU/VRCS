//! Bounded Korean lookup guesses, not a morphological analyzer.
//!
//! Candidates must be checked against dictionary headwords. Several analyses can
//! be valid (e.g. 들어요 -> 들다 / 듣다); callers must retain those alternatives.

pub(crate) const MAX_CANDIDATES: usize = 24;

fn is_hangul(character: char) -> bool {
    matches!(character, '\u{AC00}'..='\u{D7A3}')
}

/// Accept one precomposed Hangul word, optionally enclosed in chat punctuation.
/// Spaces, decomposed jamo, mixed scripts and long selections are not guessed.
pub(crate) fn normalize_query(query: &str) -> Option<&str> {
    let query = query.trim_matches(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '.' | ','
                    | '!'
                    | '?'
                    | '~'
                    | '…'
                    | '。'
                    | '，'
                    | '！'
                    | '？'
                    | '～'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '\''
                    | '"'
            )
    });
    let length = query.chars().count();
    (length > 0 && length <= 64 && query.chars().all(is_hangul)).then_some(query)
}

struct Candidates<'a> {
    query: &'a str,
    values: Vec<String>,
}

impl Candidates<'_> {
    fn push(&mut self, candidate: String) {
        if !candidate.is_empty()
            && candidate != self.query
            && self.values.len() < MAX_CANDIDATES
            && !self.values.contains(&candidate)
        {
            self.values.push(candidate);
        }
    }

    fn noun(&mut self, stem: &str) {
        if stem.is_empty() {
            return;
        }
        self.push(stem.to_owned());
        if let Some(singular) = stem.strip_suffix('들') {
            if !singular.is_empty() {
                self.push(singular.to_owned());
            }
        }
    }

    fn verb(&mut self, stem: &str) {
        if stem.is_empty() {
            return;
        }
        // Only explicit common honorific stems are reduced. In particular,
        // 마시다 must not turn into the valid but unrelated headword 마다.
        if let Some(root) = stem.strip_suffix("하시") {
            self.push(format!("{root}하다"));
        } else if let Some(base) = match stem {
            "가시" => Some("가다"),
            "오시" => Some("오다"),
            "보시" => Some("보다"),
            "주시" => Some("주다"),
            "아시" => Some("알다"),
            _ => None,
        } {
            self.push(base.to_owned());
        }
        if let Some(root) = stem.strip_suffix("으시") {
            if !root.is_empty() {
                self.push(format!("{root}다"));
            }
        }
        self.push(format!("{stem}다"));
    }
}

fn last_syllable(value: &str) -> Option<(&str, u32, u32, u32)> {
    let (index, character) = value.char_indices().next_back()?;
    if !is_hangul(character) {
        return None;
    }
    let syllable = character as u32 - 0xAC00;
    Some((
        &value[..index],
        syllable / 588,
        (syllable % 588) / 28,
        syllable % 28,
    ))
}

fn with_last(root: &str, initial: u32, vowel: u32, final_: u32) -> String {
    let character = char::from_u32(0xAC00 + initial * 588 + vowel * 28 + final_)
        .expect("Hangul components are taken from a valid syllable");
    format!("{root}{character}")
}

fn restore_stem(stem: &str, vowel_ending: bool, output: &mut Candidates<'_>) {
    if stem.is_empty() {
        return;
    }
    if let Some(base) = stem.strip_suffix("겠") {
        if !base.is_empty() {
            restore_stem(base, false, output);
            return;
        }
    }
    // Past suffixes are removed before generating dictionary forms; do not
    // present 먹었다 / 갔다 as lemmas ahead of 먹다 / 가다.
    for suffix in ["었", "았", "였"] {
        if let Some(base) = stem.strip_suffix(suffix) {
            if !base.is_empty() {
                restore_stem(base, true, output);
                return;
            }
        }
    }
    let Some((root, initial, vowel, final_)) = last_syllable(stem) else {
        return;
    };
    // The final ㅆ of 있다 is lexical, not a contracted past tense.
    // Restrict this guess to the vowels used by common past contractions.
    if final_ == 20 && matches!(vowel, 0 | 1 | 4 | 6 | 9 | 10 | 14) {
        restore_stem(&with_last(root, initial, vowel, 0), true, output);
        return;
    }

    output.verb(stem);
    if final_ == 0 {
        let original_vowel = match vowel {
            9 => Some(8),   // 봐 -> 보, 와 -> 오
            14 => Some(13), // 줘 -> 주
            6 => Some(20),  // 마셔 -> 마시
            10 => Some(11), // 돼 -> 되
            _ => None,
        };
        if let Some(vowel) = original_vowel {
            output.verb(&with_last(root, initial, vowel, 0));
        }
        if let Some(base) = stem.strip_suffix('해') {
            output.verb(&format!("{base}하"));
        }
    }

    // A small reviewed list avoids applying irregular transformations to
    // every syllable. Dictionary verification still cannot resolve homonyms.
    if vowel_ending {
        if let Some(base) = match stem {
            "들" => Some("듣"),
            "걸" => Some("걷"),
            "물" => Some("묻"),
            "지" => Some("짓"),
            "나" => Some("낫"),
            "부" => Some("붓"),
            "도와" => Some("돕"),
            "더워" => Some("덥"),
            "추워" => Some("춥"),
            "고마워" => Some("고맙"),
            "어려워" => Some("어렵"),
            "쉬워" => Some("쉽"),
            "무서워" => Some("무섭"),
            "귀여워" => Some("귀엽"),
            "아름다워" => Some("아름답"),
            "가까워" => Some("가깝"),
            "즐거워" => Some("즐겁"),
            "몰라" => Some("모르"),
            "불러" => Some("부르"),
            "달라" => Some("다르"),
            "빨라" => Some("빠르"),
            "말라" => Some("마르"),
            "골라" => Some("고르"),
            "흘러" => Some("흐르"),
            "올라" => Some("오르"),
            "예뻐" => Some("예쁘"),
            "바빠" => Some("바쁘"),
            "아파" => Some("아프"),
            "슬퍼" => Some("슬프"),
            "기뻐" => Some("기쁘"),
            "나빠" => Some("나쁘"),
            "써" => Some("쓰"),
            "커" => Some("크"),
            "꺼" => Some("끄"),
            "고파" => Some("고프"),
            _ => None,
        } {
            output.verb(base);
        }
    }
}

pub(crate) fn candidates(query: &str) -> Vec<String> {
    let Some(query) = normalize_query(query) else {
        return Vec::new();
    };
    let mut output = Candidates {
        query,
        values: Vec::new(),
    };

    // The prohibitive auxiliary 말다 loses ㄹ in these common imperatives.
    // Parsing them as ordinary 마 + 세요/십시오 would incorrectly suggest
    // 마다 or 마시다 ("drink"). Keep this exception deliberately word-specific.
    if matches!(query, "마세요" | "마십시오") {
        output.verb("말");
        return output.values;
    }

    for suffix in [
        "이었습니다",
        "이었어요",
        "였습니다",
        "였어요",
        "이었어",
        "였어",
        "입니까",
        "입니다",
        "이에요",
        "예요",
        "이라서",
        "이라면",
        "이고요",
        "이지만",
        "이고",
    ] {
        if let Some(stem) = query.strip_suffix(suffix) {
            if !stem.is_empty() {
                output.noun(stem);
                break;
            }
        }
    }

    // Match the longest applicable ending. The short attributive endings 는 / ㄴ
    // are deliberately omitted: 나는 is often 나 + 는, not a form of 나다.
    for (suffix, vowel_ending, honorific) in [
        ("으십시오", false, false),
        ("으세요", false, false),
        ("습니다", false, false),
        ("습니까", false, false),
        ("으려고", false, false),
        ("으니까", false, false),
        ("으면서", false, false),
        ("으러", false, false),
        ("으면", false, false),
        ("겠어요", false, false),
        ("겠어", false, false),
        ("어요", true, false),
        ("아요", true, false),
        ("여요", true, false),
        ("어서", true, false),
        ("아서", true, false),
        ("어도", true, false),
        ("아도", true, false),
        ("지만", false, false),
        ("는데", false, false),
        ("려고", false, false),
        ("면서", false, false),
        ("세요", false, true),
        ("십시오", false, true),
        ("니다", false, false),
        ("니까", false, false),
        ("고요", false, false),
        ("고", false, false),
        ("면", false, false),
        ("서", true, false),
        ("어", true, false),
        ("아", true, false),
        ("요", true, false),
    ] {
        if let Some(stem) = query.strip_suffix(suffix) {
            if stem.is_empty() {
                continue;
            }
            if matches!(suffix, "니다" | "니까") {
                if let Some((root, initial, vowel, 17)) = last_syllable(stem) {
                    let base = with_last(root, initial, vowel, 0);
                    restore_stem(&base, false, &mut output);
                    // ㄹ is dropped before ㅂ: 삽니다 can be 사다 or 살다.
                    output.verb(&with_last(root, initial, vowel, 8));
                    break;
                }
                if suffix == "니다" {
                    break;
                }
            }
            if honorific && matches!(stem, "계" | "주무" | "드") {
                output.verb(&format!("{stem}시"));
                break;
            }
            restore_stem(stem, vowel_ending, &mut output);
            if honorific {
                output.verb(&format!("{stem}시"));
            }
            break;
        }
    }

    for suffix in [
        "으로부터",
        "에서부터",
        "에게서는",
        "한테서는",
        "에게서",
        "한테서",
        "으로는",
        "에서는",
        "까지는",
        "부터는",
        "에게는",
        "한테는",
        "이라도",
        "이라고",
        "에는",
        "에도",
        "에만",
        "에서",
        "으로",
        "에게",
        "한테",
        "처럼",
        "보다",
        "까지",
        "부터",
        "마저",
        "조차",
        "밖에",
        "하고",
        "이랑",
        "랑",
        "이나",
        "라도",
        "의",
        "이",
        "가",
        "을",
        "를",
        "은",
        "는",
        "에",
        "도",
        "만",
        "와",
        "과",
        "로",
    ] {
        if let Some(stem) = query.strip_suffix(suffix) {
            if !stem.is_empty() {
                output.noun(stem);
                // Prefer 사과 for 사과는 before considering 사 + 과 + 는.
                // Still permit stacked particles such as 친구 + 와 + 는.
                if matches!(suffix, "는" | "도" | "만") {
                    for inner in [
                        "에서", "으로", "에게", "한테", "까지", "부터", "에", "와", "과",
                    ] {
                        if let Some(noun) = stem.strip_suffix(inner) {
                            output.noun(noun);
                            break;
                        }
                    }
                }
                break;
            }
        }
    }
    if let Some(stem) = query.strip_suffix('들') {
        output.noun(stem);
    }
    output.values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_one_bounded_precomposed_hangul_word() {
        assert_eq!(normalize_query(" “먹었어요!” "), Some("먹었어요"));
        for query in [
            "",
            "...",
            "안 먹어요",
            "hello",
            "韓国語",
            "먹어요abc",
            "먹어요",
            "ㅋㅋㅋ",
        ] {
            assert!(normalize_query(query).is_none(), "{query}");
            assert!(candidates(query).is_empty(), "{query}");
        }
        assert!(normalize_query(&"가".repeat(65)).is_none());
    }

    #[test]
    fn restores_common_endings_and_contractions() {
        for (query, expected) in [
            ("먹어요", "먹다"),
            ("좋아요", "좋다"),
            ("먹었어요", "먹다"),
            ("있어요", "있다"),
            ("없어요", "없다"),
            ("있습니다", "있다"),
            ("좋았어요", "좋다"),
            ("먹었습니다", "먹다"),
            ("갑니다", "가다"),
            ("합니까", "하다"),
            ("해요", "하다"),
            ("공부했어요", "공부하다"),
            ("봤어요", "보다"),
            ("왔어요", "오다"),
            ("줘요", "주다"),
            ("됐어요", "되다"),
            ("마셔요", "마시다"),
            ("마셨어요", "마시다"),
            ("먹으세요", "먹다"),
            ("가세요", "가다"),
            ("계세요", "계시다"),
            ("주무세요", "주무시다"),
            ("하셨어요", "하다"),
            ("먹으셨어요", "먹다"),
            ("먹고", "먹다"),
            ("먹지만", "먹다"),
            ("먹으면", "먹다"),
            ("먹어서", "먹다"),
            ("해서", "하다"),
            ("가서", "가다"),
            ("먹겠어요", "먹다"),
            ("먹겠습니다", "먹다"),
        ] {
            let found = candidates(query);
            assert!(
                found.iter().any(|entry| entry == expected),
                "{query}: {found:?}"
            );
            assert!(found.len() <= MAX_CANDIDATES);
        }
        assert!(!candidates("먹었어요").contains(&"먹".to_owned()));
        assert!(!candidates("마셨어요").contains(&"마다".to_owned()));
        assert!(!candidates("나는").contains(&"나다".to_owned()));
        assert!(!candidates("있어요").contains(&"이다".to_owned()));
        assert!(!candidates("계세요").contains(&"계다".to_owned()));
        assert!(!candidates("주무세요").contains(&"주무다".to_owned()));
        assert_eq!(candidates("마세요"), ["말다"]);
        assert_eq!(candidates("마십시오"), ["말다"]);
    }

    #[test]
    fn restores_particles_and_copulas_without_losing_short_nouns() {
        for (query, expected) in [
            ("사과는", "사과"),
            ("책을", "책"),
            ("학교에서", "학교"),
            ("한국어로", "한국어"),
            ("집으로", "집"),
            ("친구에게는", "친구"),
            ("나는", "나"),
            ("사람들이", "사람"),
            ("친구들의", "친구"),
            ("학생입니다", "학생"),
            ("학생이에요", "학생"),
            ("학생이었어요", "학생"),
            ("친구예요", "친구"),
            ("친구였어요", "친구"),
        ] {
            let found = candidates(query);
            assert!(
                found.iter().any(|entry| entry == expected),
                "{query}: {found:?}"
            );
        }
    }

    #[test]
    fn keeps_ambiguous_and_reviewed_irregular_analyses() {
        for (query, expected) in [
            ("들어요", &["들다", "듣다"][..]),
            ("걸어요", &["걸다", "걷다"]),
            ("물어요", &["물다", "묻다"]),
            ("삽니다", &["사다", "살다"]),
            ("도와요", &["돕다"]),
            ("고마워요", &["고맙다"]),
            ("몰라요", &["모르다"]),
            ("불러요", &["부르다"]),
            ("달라요", &["다르다"]),
            ("예뻐요", &["예쁘다"]),
            ("바빠요", &["바쁘다"]),
            ("아파요", &["아프다"]),
        ] {
            let found = candidates(query);
            for expected in expected {
                assert!(
                    found.iter().any(|entry| entry == expected),
                    "{query}: {found:?}"
                );
            }
        }
        assert!(!candidates("들고").contains(&"듣다".to_owned()));
    }

    #[test]
    fn hangul_syllable_transformations_remain_valid_bounded_and_unique() {
        for codepoint in 0xAC00..=0xD7A3 {
            let character = char::from_u32(codepoint).unwrap();
            for ending in ["어요", "니다", "었어요"] {
                let query = format!("{character}{ending}");
                let found = candidates(&query);
                assert!(found.len() <= MAX_CANDIDATES);
                for (index, candidate) in found.iter().enumerate() {
                    assert!(candidate.chars().all(is_hangul));
                    assert_ne!(candidate, &query);
                    assert!(!found[..index].contains(candidate));
                }
            }
        }
    }
}
