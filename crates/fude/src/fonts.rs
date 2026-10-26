//! A system font behind egui's own, for the Japanese. The candidates are
//! every uchmk app's (`ito_common::fonts::JAPANESE`): the first that reads
//! is used. Behind it, the system's emoji face, for the emoji egui's own
//! small set lacks. The band is set in a Mincho face (`ito_common::fonts::
//! MINCHO`, which carries the vertical forms of the punctuation) in a family
//! of its own, with the rest behind it.

/// The band's family, once the faces are in.
const MINCHO: &str = "mincho";

/// The family the band is set in: the Mincho one, or egui's own until the
/// faces have been read (asking for a family that is not there panics).
pub fn mincho(ctx: &egui::Context) -> egui::FontFamily {
    let family = egui::FontFamily::Name(MINCHO.into());
    if ctx.fonts(|f| f.families().contains(&family)) {
        family
    } else {
        egui::FontFamily::Proportional
    }
}

/// The system emoji faces, in order. Their outlines draw in one colour, as
/// egui's own emoji do. macOS has only a bitmap face, which egui cannot read.
#[cfg(windows)]
const EMOJI: &[&str] = &[r"C:\Windows\Fonts\seguiemj.ttf"];
#[cfg(target_os = "macos")]
const EMOJI: &[&str] = &[];
#[cfg(not(any(windows, target_os = "macos")))]
const EMOJI: &[&str] = &["/usr/share/fonts/truetype/noto/NotoEmoji-Regular.ttf", "/usr/share/fonts/noto/NotoEmoji-Regular.ttf"];

/// Read the candidates on a thread (the files are tens of megabytes) and put
/// them behind the monospace and proportional faces. Until they are in, a
/// Japanese character draws as a box for a frame or two.
pub fn load_in_background(ctx: egui::Context) {
    let _ = std::thread::Builder::new().name("fonts".into()).spawn(move || {
        let found: Vec<_> = [ito_common::fonts::JAPANESE, EMOJI].into_iter().filter_map(ito_common::fonts::read_first).collect();
        let mincho = ito_common::fonts::read_first(ito_common::fonts::MINCHO);
        let mut fonts = egui::FontDefinitions::default();
        for (path, bytes) in found {
            let name = format!("system:{}", path.display());
            fonts.font_data.insert(name.clone(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push(name.clone());
            }
        }
        let mut band = Vec::new();
        if let Some((path, bytes)) = mincho {
            let name = format!("system:{}", path.display());
            fonts.font_data.insert(name.clone(), egui::FontData::from_owned(bytes).into());
            band.push(name);
        }
        band.extend(fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default());
        fonts.families.insert(egui::FontFamily::Name(MINCHO.into()), band);
        ctx.set_fonts(fonts);
        ctx.request_repaint();
    });
}
