// Cremeplay Romanization Engine
// Pure-Rust phonetic romanization for foreign lyrics (Korean Hangul, Japanese Kana, Cyrillic, Greek).
// Follows strict English exemption rule: non-foreign lines must never be modified.

use crate::lrclib::LyricsPayload;

include!("hanzi_table.rs");

fn pinyin_lookup(c: char) -> Option<&'static str> {
    HANZI_TABLE
        .binary_search_by_key(&c, |&(ch, _)| ch)
        .ok()
        .map(|idx| HANZI_TABLE[idx].1)
}

/// Detects if the string contains non-Latin scripts (Korean, Japanese Kana, Cyrillic, Greek, Hanzi).
pub fn has_foreign_script(text: &str) -> bool {
    text.chars().any(|c| match c as u32 {
        // Korean Hangul Syllables and Jamo
        0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F => true,
        // Japanese Hiragana & Katakana
        0x3040..=0x30FF => true,
        // Cyrillic
        0x0400..=0x04FF => true,
        // Greek
        0x0370..=0x03FF => true,
        // CJK Unified Ideographs
        0x4E00..=0x9FFF => true,
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// Hangul Revised Romanization
// ---------------------------------------------------------------------------

const HANGUL_INITIALS: [&str; 19] = [
    "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t", "p", "h",
];

const HANGUL_MEDIALS: [&str; 21] = [
    "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo", "we", "wi", "yu", "eu", "ui", "i",
];

const HANGUL_FINALS: [&str; 28] = [
    "", "k", "k", "k", "n", "n", "n", "t", "l", "k", "m", "p", "l", "l", "p", "l", "m", "p", "p", "t", "t", "ng", "t", "t", "k", "t", "p", "h",
];

fn romanize_hangul_char(c: char) -> Option<String> {
    let cp = c as u32;
    if (0xAC00..=0xD7AF).contains(&cp) {
        let s_idx = cp - 0xAC00;
        let l_idx = (s_idx / 588) as usize;
        let v_idx = ((s_idx % 588) / 28) as usize;
        let t_idx = (s_idx % 28) as usize;

        let mut out = String::new();
        out.push_str(HANGUL_INITIALS.get(l_idx).unwrap_or(&""));
        out.push_str(HANGUL_MEDIALS.get(v_idx).unwrap_or(&""));
        out.push_str(HANGUL_FINALS.get(t_idx).unwrap_or(&""));
        Some(out)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Japanese Kana Romanization (Hepburn)
// ---------------------------------------------------------------------------

fn kana_lookup(c: char) -> Option<&'static str> {
    match c {
        // Hiragana
        'あ' => Some("a"), 'い' => Some("i"), 'う' => Some("u"), 'え' => Some("e"), 'お' => Some("o"),
        'か' => Some("ka"), 'き' => Some("ki"), 'く' => Some("ku"), 'け' => Some("ke"), 'こ' => Some("ko"),
        'さ' => Some("sa"), 'し' => Some("shi"), 'す' => Some("su"), 'せ' => Some("se"), 'そ' => Some("so"),
        'た' => Some("ta"), 'ち' => Some("chi"), 'つ' => Some("tsu"), 'て' => Some("te"), 'と' => Some("to"),
        'な' => Some("na"), 'に' => Some("ni"), 'ぬ' => Some("nu"), 'ね' => Some("ne"), 'の' => Some("no"),
        'は' => Some("ha"), 'ひ' => Some("hi"), 'ふ' => Some("fu"), 'へ' => Some("he"), 'ほ' => Some("ho"),
        'ま' => Some("ma"), 'み' => Some("mi"), 'む' => Some("mu"), 'め' => Some("me"), 'も' => Some("mo"),
        'や' => Some("ya"), 'ゆ' => Some("yu"), 'よ' => Some("yo"),
        'ら' => Some("ra"), 'り' => Some("ri"), 'る' => Some("ru"), 'れ' => Some("re"), 'ろ' => Some("ro"),
        'わ' => Some("wa"), 'ゐ' => Some("wi"), 'ゑ' => Some("we"), 'を' => Some("wo"), 'ん' => Some("n"),
        'が' => Some("ga"), 'ぎ' => Some("gi"), 'ぐ' => Some("gu"), 'げ' => Some("ge"), 'ご' => Some("go"),
        'ざ' => Some("za"), 'じ' => Some("ji"), 'ず' => Some("zu"), 'ぜ' => Some("ze"), 'ぞ' => Some("zo"),
        'だ' => Some("da"), 'ぢ' => Some("ji"), 'づ' => Some("zu"), 'で' => Some("de"), 'ど' => Some("do"),
        'ば' => Some("ba"), 'び' => Some("bi"), 'ぶ' => Some("bu"), 'べ' => Some("be"), 'ぼ' => Some("bo"),
        'ぱ' => Some("pa"), 'ぴ' => Some("pi"), 'ぷ' => Some("pu"), 'ぺ' => Some("pe"), 'ぽ' => Some("po"),

        // Katakana
        'ア' => Some("a"), 'イ' => Some("i"), 'ウ' => Some("u"), 'エ' => Some("e"), 'オ' => Some("o"),
        'カ' => Some("ka"), 'キ' => Some("ki"), 'ク' => Some("ku"), 'ケ' => Some("ke"), 'コ' => Some("ko"),
        'サ' => Some("sa"), 'シ' => Some("shi"), 'ス' => Some("su"), 'セ' => Some("se"), 'ソ' => Some("so"),
        'タ' => Some("ta"), 'チ' => Some("chi"), 'ツ' => Some("tsu"), 'テ' => Some("te"), 'ト' => Some("to"),
        'ナ' => Some("na"), 'ニ' => Some("ni"), 'ヌ' => Some("nu"), 'ネ' => Some("ne"), 'ノ' => Some("no"),
        'ハ' => Some("ha"), 'ヒ' => Some("hi"), 'フ' => Some("fu"), 'ヘ' => Some("he"), 'ホ' => Some("ho"),
        'マ' => Some("ma"), 'ミ' => Some("mi"), 'ム' => Some("mu"), 'メ' => Some("me"), 'モ' => Some("mo"),
        'ヤ' => Some("ya"), 'ユ' => Some("yu"), 'ヨ' => Some("yo"),
        'ラ' => Some("ra"), 'リ' => Some("ri"), 'ル' => Some("ru"), 'レ' => Some("re"), 'ロ' => Some("ro"),
        'ワ' => Some("wa"), 'ヰ' => Some("wi"), 'ヱ' => Some("we"), 'ヲ' => Some("wo"), 'ン' => Some("n"),
        'ガ' => Some("ga"), 'ギ' => Some("gi"), 'グ' => Some("gu"), 'ゲ' => Some("ge"), 'ゴ' => Some("go"),
        'ザ' => Some("za"), 'ジ' => Some("ji"), 'ズ' => Some("zu"), 'ゼ' => Some("ze"), 'ゾ' => Some("zo"),
        'ダ' => Some("da"), 'ヂ' => Some("ji"), 'ヅ' => Some("zu"), 'デ' => Some("de"), 'ド' => Some("do"),
        'バ' => Some("ba"), 'ビ' => Some("bi"), 'ブ' => Some("bu"), 'ベ' => Some("be"), 'ボ' => Some("bo"),
        'パ' => Some("pa"), 'ピ' => Some("pi"), 'プ' => Some("pu"), 'ペ' => Some("pe"), 'ポ' => Some("po"),
        'ヴ' => Some("vu"),
        _ => None,
    }
}

fn kana_digraph(first: char, second: char) -> Option<&'static str> {
    match (first, second) {
        ('き', 'ゃ') | ('キ', 'ャ') => Some("kya"),
        ('き', 'ゅ') | ('キ', 'ュ') => Some("kyu"),
        ('き', 'ょ') | ('キ', 'ョ') => Some("kyo"),
        ('し', 'ゃ') | ('シ', 'ャ') => Some("sha"),
        ('し', 'ゅ') | ('シ', 'ュ') => Some("shu"),
        ('し', 'ょ') | ('シ', 'ョ') => Some("sho"),
        ('ち', 'ゃ') | ('チ', 'ャ') => Some("cha"),
        ('ち', 'ゅ') | ('チ', 'ュ') => Some("chu"),
        ('ち', 'ょ') | ('チ', 'ョ') => Some("cho"),
        ('に', 'ゃ') | ('ニ', 'ャ') => Some("nya"),
        ('に', 'ゅ') | ('ニ', 'ュ') => Some("nyu"),
        ('に', 'ょ') | ('ニ', 'ョ') => Some("nyo"),
        ('ひ', 'ゃ') | ('ヒ', 'ャ') => Some("hya"),
        ('ひ', 'ゅ') | ('ヒ', 'ュ') => Some("hyu"),
        ('ひ', 'ょ') | ('ヒ', 'ョ') => Some("hyo"),
        ('み', 'ゃ') | ('ミ', 'ャ') => Some("mya"),
        ('み', 'ゅ') | ('ミ', 'ュ') => Some("myu"),
        ('み', 'ょ') | ('ミ', 'ョ') => Some("myo"),
        ('り', 'ゃ') | ('リ', 'ャ') => Some("rya"),
        ('り', 'ゅ') | ('リ', 'ュ') => Some("ryu"),
        ('り', 'ょ') | ('リ', 'ョ') => Some("ryo"),
        ('ぎ', 'ゃ') | ('ギ', 'ャ') => Some("gya"),
        ('ぎ', 'ゅ') | ('ギ', 'ュ') => Some("gyu"),
        ('ぎ', 'ょ') | ('ギ', 'ョ') => Some("gyo"),
        ('じ', 'ゃ') | ('ジ', 'ャ') => Some("ja"),
        ('じ', 'ゅ') | ('ジ', 'ュ') => Some("ju"),
        ('じ', 'ょ') | ('ジ', 'ョ') => Some("jo"),
        ('び', 'ゃ') | ('ビ', 'ャ') => Some("bya"),
        ('び', 'ゅ') | ('ビ', 'ュ') => Some("byu"),
        ('び', 'ょ') | ('ビ', 'ョ') => Some("byo"),
        ('ぴ', 'ゃ') | ('ピ', 'ャ') => Some("pya"),
        ('ぴ', 'ゅ') | ('ピ', 'ュ') => Some("pyu"),
        ('ぴ', 'ょ') | ('ピ', 'ョ') => Some("pyo"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Cyrillic Romanization
// ---------------------------------------------------------------------------

fn cyrillic_lookup(c: char) -> Option<&'static str> {
    match c {
        'А' => Some("A"), 'а' => Some("a"),
        'Б' => Some("B"), 'б' => Some("b"),
        'В' => Some("V"), 'в' => Some("v"),
        'Г' => Some("G"), 'г' => Some("g"),
        'Д' => Some("D"), 'д' => Some("d"),
        'Е' => Some("E"), 'е' => Some("e"),
        'Ё' => Some("Yo"), 'ё' => Some("yo"),
        'Ж' => Some("Zh"), 'ж' => Some("zh"),
        'З' => Some("Z"), 'з' => Some("z"),
        'И' => Some("I"), 'и' => Some("i"),
        'Й' => Some("Y"), 'й' => Some("y"),
        'К' => Some("K"), 'к' => Some("k"),
        'Л' => Some("L"), 'л' => Some("l"),
        'М' => Some("M"), 'м' => Some("m"),
        'Н' => Some("N"), 'н' => Some("n"),
        'О' => Some("O"), 'о' => Some("o"),
        'П' => Some("P"), 'п' => Some("p"),
        'Р' => Some("R"), 'р' => Some("r"),
        'С' => Some("S"), 'с' => Some("s"),
        'Т' => Some("T"), 'т' => Some("t"),
        'У' => Some("U"), 'у' => Some("u"),
        'Ф' => Some("F"), 'ф' => Some("f"),
        'Х' => Some("Kh"), 'х' => Some("kh"),
        'Ц' => Some("Ts"), 'ц' => Some("ts"),
        'Ч' => Some("Ch"), 'ч' => Some("ch"),
        'Ш' => Some("Sh"), 'ш' => Some("sh"),
        'Щ' => Some("Shch"), 'щ' => Some("shch"),
        'Ъ' | 'ъ' => Some(""),
        'Ы' => Some("Y"), 'ы' => Some("y"),
        'Ь' | 'ь' => Some(""),
        'Э' => Some("E"), 'э' => Some("e"),
        'Ю' => Some("Yu"), 'ю' => Some("yu"),
        'Я' => Some("Ya"), 'я' => Some("ya"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Greek Romanization
// ---------------------------------------------------------------------------

fn greek_lookup(c: char) -> Option<&'static str> {
    match c {
        'Α' | 'Ά' => Some("A"), 'α' | 'ά' => Some("a"),
        'Β' => Some("V"), 'β' => Some("v"),
        'Γ' => Some("G"), 'γ' => Some("g"),
        'Δ' => Some("D"), 'δ' => Some("d"),
        'Ε' | 'Έ' => Some("E"), 'ε' | 'έ' => Some("e"),
        'Ζ' => Some("Z"), 'ζ' => Some("z"),
        'Η' | 'Ή' => Some("I"), 'η' | 'ή' => Some("i"),
        'Θ' => Some("Th"), 'θ' => Some("th"),
        'Ι' | 'Ί' => Some("I"), 'ι' | 'ί' | 'ϊ' | 'ΐ' => Some("i"),
        'Κ' => Some("K"), 'κ' => Some("k"),
        'Λ' => Some("L"), 'λ' => Some("l"),
        'Μ' => Some("M"), 'μ' => Some("m"),
        'Ν' => Some("N"), 'ν' => Some("n"),
        'Ξ' => Some("X"), 'ξ' => Some("x"),
        'Ο' | 'Ό' => Some("O"), 'ο' | 'ό' => Some("o"),
        'Π' => Some("P"), 'π' => Some("p"),
        'Ρ' => Some("R"), 'ρ' => Some("r"),
        'Σ' => Some("S"), 'σ' | 'ς' => Some("s"),
        'Τ' => Some("T"), 'τ' => Some("t"),
        'Υ' | 'Ύ' => Some("Y"), 'υ' | 'ύ' | 'ϋ' | 'ΰ' => Some("y"),
        'Φ' => Some("F"), 'φ' => Some("f"),
        'Χ' => Some("Ch"), 'χ' => Some("ch"),
        'Ψ' => Some("Ps"), 'ψ' => Some("ps"),
        'Ω' | 'Ώ' => Some("O"), 'ω' | 'ώ' => Some("o"),
        _ => None,
    }
}

/// Converts a single line of lyrics into romanized phonetic Latin text.
/// If the line does not contain foreign characters (e.g. pure English), returns None.
pub fn romanize_line(line: &str) -> Option<String> {
    if !has_foreign_script(line) {
        return None;
    }

    let mut result = String::with_capacity(line.len() * 2);
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut modified = false;

    while i < len {
        let c = chars[i];

        // 1. Check Japanese small tsu (sokuon: っ or ッ)
        if (c == 'っ' || c == 'ッ') && i + 1 < len
            && let Some(next_rom) = kana_lookup(chars[i + 1])
                && let Some(first_char) = next_rom.chars().next() {
                    result.push(first_char);
                    modified = true;
                    i += 1;
                    continue;
                }

        // 2. Check Japanese digraphs (kya, sho, etc.)
        if i + 1 < len
            && let Some(digraph) = kana_digraph(c, chars[i + 1]) {
                result.push_str(digraph);
                modified = true;
                i += 2;
                continue;
            }

        // 3. Korean Hangul
        if let Some(hangul) = romanize_hangul_char(c) {
            result.push_str(&hangul);
            modified = true;
            i += 1;
            continue;
        }

        // 4. Japanese Kana single character
        if let Some(kana) = kana_lookup(c) {
            result.push_str(kana);
            modified = true;
            i += 1;
            continue;
        }

        // 5. Cyrillic
        if let Some(cyr) = cyrillic_lookup(c) {
            result.push_str(cyr);
            modified = true;
            i += 1;
            continue;
        }

        // 6. Greek
        if let Some(grk) = greek_lookup(c) {
            result.push_str(grk);
            modified = true;
            i += 1;
            continue;
        }

        // 7. Chinese Hanzi (Pinyin)
        if let Some(pinyin) = pinyin_lookup(c) {
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
            }
            result.push_str(pinyin);
            if i + 1 < len && chars[i + 1] != ' ' {
                result.push(' ');
            }
            modified = true;
            i += 1;
            continue;
        }

        // Japanese prolonged sound mark: ー
        if c == 'ー'
            && let Some(prev) = result.chars().last()
                && matches!(prev, 'a' | 'i' | 'u' | 'e' | 'o') {
                    result.push(prev);
                    modified = true;
                    i += 1;
                    continue;
                }

        // Fallthrough: keep character as-is
        result.push(c);
        i += 1;
    }

    if modified && result != line {
        Some(result.trim().to_string())
    } else {
        None
    }
}

/// Applies romanization to all synced lines in a `LyricsPayload` in place.
pub fn romanize_payload(payload: &mut LyricsPayload) {
    for line in &mut payload.lines {
        if line.romanized.is_none() {
            line.romanized = romanize_line(&line.text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english_exemption() {
        assert_eq!(romanize_line("Hello world, this is English!"), None);
        assert_eq!(romanize_line("I want to break free"), None);
        assert_eq!(romanize_line("12345!@#$%^&*()"), None);
    }

    #[test]
    fn test_korean_hangul_romanization() {
        assert_eq!(romanize_line("안녕하세요"), Some("annyeonghaseyo".to_string()));
        assert_eq!(romanize_line("사랑해"), Some("saranghae".to_string()));
    }

    #[test]
    fn test_japanese_kana_romanization() {
        assert_eq!(romanize_line("さくら"), Some("sakura".to_string()));
        assert_eq!(romanize_line("ありがとう"), Some("arigatou".to_string()));
        assert_eq!(romanize_line("ちょっと"), Some("chotto".to_string()));
        assert_eq!(romanize_line("ラーメン"), Some("raamen".to_string()));
    }

    #[test]
    fn test_cyrillic_romanization() {
        assert_eq!(romanize_line("Привет мир"), Some("Privet mir".to_string()));
        assert_eq!(romanize_line("Хорошо"), Some("Khorosho".to_string()));
    }

    #[test]
    fn test_greek_romanization() {
        assert_eq!(romanize_line("Καλημέρα"), Some("Kalimera".to_string()));
    }

    #[test]
    fn test_chinese_pinyin_romanization() {
        assert_eq!(romanize_line("我爱你"), Some("wo ai ni".to_string()));
        assert_eq!(romanize_line("明天会更好"), Some("ming tian hui geng hao".to_string()));
        assert_eq!(romanize_line("我爱你 baby"), Some("wo ai ni baby".to_string()));
    }
}
