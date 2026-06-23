fn main() {
    #[cfg(windows)]
    {
        let ver = env!("CARGO_PKG_VERSION");
        // Convert "2.0.0" -> "2,0,0,0"
        let ver_csv = ver.split('.').chain(std::iter::repeat("0")).take(4).collect::<Vec<_>>().join(",");

        let ico_path = std::path::Path::new("static/icon/app.ico");
        let svg_path = std::path::Path::new("static/svg/logo_monitor.svg");

        if !ico_path.exists() && svg_path.exists() {
            if let Some(bytes) = render_svg_to_ico(svg_path) {
                let _ = std::fs::write(ico_path, &bytes);
            }
        }

        let has_ico = ico_path.exists();
        let ico_line = if has_ico { r#"app ICON "static/icon/app.ico""# } else { "" };
        let rc = format!(r#"{}
1 VERSIONINFO
FILEVERSION {ver}
PRODUCTVERSION {ver}
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "Key & Mouse Click Monitor"
      VALUE "ProductName", "keymouse-monitor"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
            ico_line,
            ver = ver_csv
        );
        let rc_dir = std::path::Path::new("static/icon");
        let _ = std::fs::create_dir_all(rc_dir);
        let rc_path = rc_dir.join("app.rc");
        let _ = std::fs::write(&rc_path, rc);
        embed_resource::compile(&rc_path, embed_resource::NONE);
    }
}

#[cfg(windows)]
fn render_svg_to_ico(svg_path: &std::path::Path) -> Option<Vec<u8>> {
    let svg_data = std::fs::read(svg_path).ok()?;
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(&svg_data, &opt).ok()?;
    let svg_size = tree.size();
    let svg_w: f32 = svg_size.width();
    let svg_h: f32 = svg_size.height();

    let sizes: [u32; 4] = [16, 32, 48, 256];
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for &target in &sizes {
        let scale = target as f32 / svg_w.max(svg_h);
        let w = (svg_w * scale).round() as u32;
        let h = (svg_h * scale).round() as u32;
        let w = w.max(1);
        let h = h.max(1);

        let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)?;
        let ts = resvg::tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, ts, &mut pixmap.as_mut());
        let straight = un_premultiply(pixmap.data());
        let icon_img = ico::IconImage::from_rgba_data(w, h, straight);
        let entry = ico::IconDirEntry::encode_as_png(&icon_img).ok()?;
        icon_dir.add_entry(entry);
    }

    let mut buf = std::io::Cursor::new(Vec::new());
    icon_dir.write(&mut buf).ok()?;
    Some(buf.into_inner())
}

#[cfg(windows)]
#[allow(clippy::manual_checked_ops)]
fn un_premultiply(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    for px in out.chunks_exact_mut(4) {
        let a = px[3] as u16;
        if a == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else {
            px[0] = ((px[0] as u16 * 255).min(a * 255) / a) as u8;
            px[1] = ((px[1] as u16 * 255).min(a * 255) / a) as u8;
            px[2] = ((px[2] as u16 * 255).min(a * 255) / a) as u8;
        }
    }
    out
}
