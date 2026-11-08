use rune::color::hex;
use rune::Color;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub is_dark: bool,
    pub surface: Color,
    pub panel: Color,
    pub band: Color,
    pub brand: Color,
    pub brand_text: Color,
    pub heading: Color,
    pub body: Color,
    pub muted: Color,
    pub border: Color,
    pub hover: Color,
}

fn c(s: &str) -> Color {
    hex(s).expect("valid hex color")
}

fn c_alpha(s: &str, a: f32) -> Color {
    let mut col = hex(s).expect("valid hex color");
    col.a = a;
    col
}

impl Theme {
    pub fn get(id: &str) -> Self {
        all_themes()
            .into_iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| all_themes()[0])
    }
}

pub fn all_themes() -> [Theme; 10] {
    [
        // 0: Classic (Default Cream & Coral - Iconic Cremeplay Look)
        Theme {
            id: "classic",
            name: "Classic Cream",
            is_dark: false,
            surface: c("#F7F7F5"),
            panel: c("#FBFAF9"),
            band: c("#F1F2EA"),
            brand: c("#E4544B"),
            brand_text: c("#C9443A"),
            heading: c("#1C1917"),
            body: c("#57534E"),
            muted: c("#79716B"),
            border: c("#E7E6E5"),
            hover: c_alpha("#1C1917", 0.06),
        },
        // 1: Adaptive Dark (Modern Dark Mode)
        Theme {
            id: "adaptive",
            name: "Adaptive Dark",
            is_dark: true,
            surface: c("#14161B"),
            panel: c("#1E2128"),
            band: c("#292D36"),
            brand: c("#E4544B"),
            brand_text: c("#FF7B73"),
            heading: c("#FFFFFF"),
            body: c("#D4D8E0"),
            muted: c("#959CA8"),
            border: c_alpha("#FFFFFF", 0.15),
            hover: c_alpha("#FFFFFF", 0.10),
        },
        // 2: Dark Olive
        Theme {
            id: "dark",
            name: "Dark Olive",
            is_dark: true,
            surface: c("#1F2319"),
            panel: c("#2B3022"),
            band: c("#46321F"),
            brand: c("#F2EAA7"),
            brand_text: c("#F2EAA7"),
            heading: c("#F2EAA7"),
            body: c("#CFC9A0"),
            muted: c("#A29B72"),
            border: c_alpha("#F2EAA7", 0.14),
            hover: c_alpha("#F2EAA7", 0.10),
        },
        // 3: European Vintage (Cream & Wine)
        Theme {
            id: "european-vintage",
            name: "European Vintage",
            is_dark: false,
            surface: c("#EFE9D3"),
            panel: c("#F7F3E6"),
            band: c("#E5DFC7"),
            brand: c("#602238"),
            brand_text: c("#602238"),
            heading: c("#602238"),
            body: c("#4A1A2B"),
            muted: c("#87626F"),
            border: c("#DED6BD"),
            hover: c_alpha("#602238", 0.09),
        },
        // 4: Chinese Vintage (Vermilion & Rice Paper)
        Theme {
            id: "chinese-vintage",
            name: "Chinese Vintage",
            is_dark: false,
            surface: c("#F2E4D0"),
            panel: c("#FAF2E6"),
            band: c("#EBC799"),
            brand: c("#BD433E"),
            brand_text: c("#A63632"),
            heading: c("#7A2824"),
            body: c("#57332A"),
            muted: c("#9C7262"),
            border: c("#DEBC90"),
            hover: c_alpha("#BD433E", 0.09),
        },
        // 5: Matcha Forest
        Theme {
            id: "matcha-forest",
            name: "Matcha Forest",
            is_dark: false,
            surface: c("#F4EDE0"),
            panel: c("#FAF5EC"),
            band: c("#E6DFCF"),
            brand: c("#54927D"),
            brand_text: c("#3D7361"),
            heading: c("#453832"),
            body: c("#4F443E"),
            muted: c("#7E7269"),
            border: c("#DCD2BF"),
            hover: c_alpha("#54927D", 0.10),
        },
        // 6: Spring Blossom
        Theme {
            id: "spring-blossom",
            name: "Spring Blossom",
            is_dark: false,
            surface: c("#FFF5F7"),
            panel: c("#FFFFFF"),
            band: c("#FFE1E7"),
            brand: c("#FC97B7"),
            brand_text: c("#DE648B"),
            heading: c("#542836"),
            body: c("#754353"),
            muted: c("#AA7585"),
            border: c("#FCD0DC"),
            hover: c_alpha("#FC97B7", 0.10),
        },
        // 7: Blueberry Berry
        Theme {
            id: "blueberry-berry",
            name: "Blueberry Berry",
            is_dark: true,
            surface: c("#241A20"),
            panel: c("#2F222B"),
            band: c("#412B42"),
            brand: c("#86A88E"),
            brand_text: c("#9EC2A6"),
            heading: c("#E2D4E0"),
            body: c("#BDB0BC"),
            muted: c("#8C7E8B"),
            border: c_alpha("#86A88E", 0.25),
            hover: c_alpha("#86A88E", 0.10),
        },
        // 8: Sapphire Heritage
        Theme {
            id: "sapphire-heritage",
            name: "Sapphire Heritage",
            is_dark: true,
            surface: c("#1A1F28"),
            panel: c("#232A36"),
            band: c("#2D3645"),
            brand: c("#4B5DC3"),
            brand_text: c("#6879DE"),
            heading: c("#FFFFFF"),
            body: c("#B0B9C9"),
            muted: c("#7D8799"),
            border: c_alpha("#4B5DC3", 0.30),
            hover: c_alpha("#4B5DC3", 0.12),
        },
        // 9: Crimson Amber
        Theme {
            id: "crimson-amber",
            name: "Crimson Amber",
            is_dark: false,
            surface: c("#FFFDF5"),
            panel: c("#FAF6EA"),
            band: c("#F2EBD7"),
            brand: c("#B61524"),
            brand_text: c("#940E1B"),
            heading: c("#3D070D"),
            body: c("#59171F"),
            muted: c("#8A4E55"),
            border: c("#E5D9BF"),
            hover: c_alpha("#B61524", 0.08),
        },
    ]
}

pub mod typo {
    use rune::TextStyle;

    pub const HERO: TextStyle = TextStyle::new(26.0, 700.0);
    pub const TITLE: TextStyle = TextStyle::new(16.0, 600.0);
    pub const SUBTITLE: TextStyle = TextStyle::new(14.0, 500.0);
    pub const BODY: TextStyle = TextStyle::new(14.0, 400.0);
    pub const LABEL: TextStyle = TextStyle::new(13.0, 500.0);
    pub const CAPTION: TextStyle = TextStyle::new(11.0, 400.0);
    pub const MONO_TIMER: TextStyle = TextStyle::new(12.0, 400.0);
}
