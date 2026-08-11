//! Feature-gated Simplified Chinese fallback data for filesystem-free WASM.

use std::sync::OnceLock;

use typst::text::Font;

pub(crate) const CJK_LAST_RESORT_FAMILY: &str = "Noto Sans CJK SC";

const CJK_FONT_BYTES: &[u8] = include_bytes!("../fonts/NotoSansCJKsc-GB2312.otf");

static CJK_FONTS: OnceLock<Vec<Font>> = OnceLock::new();

/// Parsed bundled faces, cached because the 3.3 MiB subset is immutable.
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

    #[test]
    fn bundled_face_is_indexed_as_simplified_chinese_coverage() {
        let context =
            crate::render::font_context::resolve_font_search_context_from_fonts(cjk_fonts());
        assert!(context.covers_script(
            CJK_LAST_RESORT_FAMILY,
            crate::render::font_subst::TextScript::Chinese
        ));
    }

    #[test]
    fn bundled_face_stays_within_the_documented_size_envelope() {
        assert!(CJK_FONT_BYTES.len() >= 2_000_000);
        assert!(CJK_FONT_BYTES.len() <= 4_000_000);
    }
}
