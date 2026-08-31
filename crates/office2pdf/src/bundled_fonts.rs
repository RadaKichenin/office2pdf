//! Reproducible fallback faces shipped with the converter.

use std::sync::OnceLock;

use typst::text::Font;

pub(crate) const NOTO_SERIF_FAMILY: &str = "Noto Serif";

const NOTO_SERIF_REGULAR_BYTES: &[u8] = include_bytes!("../fonts/NotoSerif-Regular.ttf");
const NOTO_SERIF_BOLD_BYTES: &[u8] = include_bytes!("../fonts/NotoSerif-Bold.ttf");

static NOTO_SERIF_FONTS: OnceLock<Vec<Font>> = OnceLock::new();

/// Noto Serif 2.015 Regular and Bold, parsed once for deterministic Office
/// poster fallbacks on native and WASM builds (issue #1458).
pub(crate) fn noto_serif_fonts() -> &'static [Font] {
    NOTO_SERIF_FONTS.get_or_init(|| {
        let fonts = crate::render::pdf::load_fonts_from_bytes([
            NOTO_SERIF_REGULAR_BYTES,
            NOTO_SERIF_BOLD_BYTES,
        ]);
        assert_eq!(
            fonts.len(),
            2,
            "the bundled Noto Serif assets must contain two usable faces"
        );
        fonts
    })
}

#[cfg(all(feature = "wasm-cjk-font", any(target_arch = "wasm32", test)))]
pub(crate) const CJK_LAST_RESORT_FAMILY: &str = "Noto Sans CJK SC";

#[cfg(all(feature = "wasm-cjk-font", any(target_arch = "wasm32", test)))]
const CJK_FONT_BYTES: &[u8] = include_bytes!("../fonts/NotoSansCJKsc-GB2312.otf");

#[cfg(all(feature = "wasm-cjk-font", any(target_arch = "wasm32", test)))]
static CJK_FONTS: OnceLock<Vec<Font>> = OnceLock::new();

/// Parsed bundled faces, cached because the 3.3 MiB subset is immutable.
#[cfg(all(feature = "wasm-cjk-font", any(target_arch = "wasm32", test)))]
pub(crate) fn cjk_fonts() -> &'static [Font] {
    CJK_FONTS.get_or_init(|| {
        let fonts = crate::render::pdf::load_fonts_from_bytes([CJK_FONT_BYTES]);
        assert!(
            !fonts.is_empty(),
            "the bundled CJK asset must contain a usable font face"
        );
        fonts
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_noto_serif_has_regular_and_bold_faces() {
        let fonts = noto_serif_fonts();
        assert_eq!(fonts.len(), 2);
        assert!(
            fonts
                .iter()
                .all(|font| font.info().family == NOTO_SERIF_FAMILY)
        );

        let mut weights: Vec<u16> = fonts
            .iter()
            .map(|font| font.info().variant.weight.to_number())
            .collect();
        weights.sort_unstable();
        assert_eq!(weights, vec![400, 700]);

        for character in ['A', 'É', 'Ω', 'Ж'] {
            assert!(
                fonts
                    .iter()
                    .all(|font| font.info().coverage.contains(character as u32)),
                "both bundled Noto Serif faces should cover {character:?}"
            );
        }
    }

    #[test]
    fn bundled_noto_serif_assets_stay_within_the_documented_size_envelope() {
        for bytes in [NOTO_SERIF_REGULAR_BYTES, NOTO_SERIF_BOLD_BYTES] {
            assert!(bytes.len() >= 450_000);
            assert!(bytes.len() <= 500_000);
        }
    }

    #[cfg(feature = "wasm-cjk-font")]
    #[test]
    fn bundled_face_has_declared_family_and_gb2312_samples() {
        let fonts = cjk_fonts();
        assert_eq!(fonts.len(), 1);
        let info = fonts[0].info();
        assert_eq!(info.family, CJK_LAST_RESORT_FAMILY);

        for character in [
            'A', '中', '文', '测', '试', '国', '龟', '啊', '凹', '座', '￥', '。',
        ] {
            assert!(
                info.coverage.contains(character as u32),
                "bundled face should cover {character:?}"
            );
        }
        for character in ['龍', '가'] {
            assert!(
                !info.coverage.contains(character as u32),
                "coverage must not be described as pan-CJK: {character:?} is absent"
            );
        }
    }

    #[cfg(feature = "wasm-cjk-font")]
    #[test]
    fn bundled_face_is_indexed_as_simplified_chinese_coverage() {
        let context =
            crate::render::font_context::resolve_font_search_context_from_fonts(cjk_fonts());
        assert!(context.covers_script(
            CJK_LAST_RESORT_FAMILY,
            crate::render::font_subst::TextScript::Chinese
        ));
    }

    #[cfg(feature = "wasm-cjk-font")]
    #[test]
    fn bundled_face_stays_within_the_documented_size_envelope() {
        assert!(CJK_FONT_BYTES.len() >= 2_000_000);
        assert!(CJK_FONT_BYTES.len() <= 4_000_000);
    }
}
