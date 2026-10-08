#![cfg(feature = "native")]
use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{ExportOptions, pptx::render_pptx, render_pdf, render_png, render_svg};

#[test]
fn opentype_math_outlines_export_in_every_vector_and_bitmap_backend() {
    for bytes in [
        include_bytes!("../../../tests/fonts/math/latinmodern-math.otf").as_slice(),
        include_bytes!("../../../tests/fonts/math/STIX2Math.otf").as_slice(),
        include_bytes!("../../../tests/fonts/math/XITSMath-Regular.otf").as_slice(),
    ] {
        let mut host = Host::default();
        host.files.insert("/math.otf".into(), bytes.to_vec());
        let source = r##"page=canvas(size=(100mm,60mm),background="#ffffff")
page.add(formula(r"\left(\frac{-b+\sqrt{b^2-4ac}}{2a}\right)",math_font="/math.otf",font_size=18pt,style=display),offset=(5mm,5mm))"##;
        let scene = compile_source(source, "/main.lay", host).unwrap();
        assert!(scene.fonts.is_empty());
        let svg = render_svg(&scene).unwrap();
        assert!(svg.contains("<path") && !svg.contains("<text") && !svg.contains("@font-face"));
        assert!(render_pdf(&scene).unwrap().starts_with(b"%PDF-"));
        let png = render_png(&scene, 120.).unwrap();
        let info = png::Decoder::new(std::io::Cursor::new(png))
            .read_info()
            .unwrap();
        assert_eq!((info.info().width, info.info().height), (472, 283));
        assert!(
            render_pptx(&scene, &ExportOptions::default())
                .unwrap()
                .0
                .starts_with(b"PK")
        );
    }
}

#[test]
fn chemical_physical_and_proof_constructions_export_without_text_or_font_substitution() {
    for bytes in [
        include_bytes!("../../../tests/fonts/math/latinmodern-math.otf").as_slice(),
        include_bytes!("../../../tests/fonts/math/STIX2Math.otf").as_slice(),
        include_bytes!("../../../tests/fonts/math/XITSMath-Regular.otf").as_slice(),
    ] {
        for math in [
            r"\ce{Zn^2+ <=>[+ 2OH-][+ 2H+] Zn(OH)2}",
            r"\color{#2468ac}{\ce{A\bond{~}B\bond{~-}C\bond{~--}D\bond{~=}E\bond{-~-}F}}+X_{\ce{A\bond{-~-}B}}",
            r"\Braket{\psi|\hat{H}|\psi}=\frac{1}{2}m\dot{x}^2",
            r"\begin{prooftree}\AxiomC{A}\AxiomC{B}\RightLabel{cut}\dashedLine\BinaryInfC{C}\rootAtTop\UnaryInfC{D}\end{prooftree}",
            r"\boxed{E=mc^2}+\cancel{\frac{a}{b}}+\begin{array}{|c|c|}\hline a&b\\\hdashline c&d\end{array}",
        ] {
            let mut host = Host::default();
            host.files.insert("/math.otf".into(), bytes.to_vec());
            let source = format!(
                "page=canvas(size=(220mm,70mm),background=\"#ffffff\")\npage.add(formula(r\"{math}\",math_font=\"/math.otf\",font_size=18pt,style=display),offset=(5mm,5mm))"
            );
            let scene = compile_source(&source, "/domain.lay", host).unwrap();
            assert!(scene.fonts.is_empty());
            let svg = render_svg(&scene).unwrap();
            assert!(svg.contains("<path") && !svg.contains("<text") && !svg.contains("@font-face"));
            assert!(render_pdf(&scene).unwrap().starts_with(b"%PDF-"));
            let data = render_png(&scene, 120.).unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(data))
                .read_info()
                .unwrap();
            let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut pixels).unwrap();
            assert!(
                pixels[..info.buffer_size()]
                    .iter()
                    .filter(|&&v| v < 128)
                    .count()
                    > 300,
                "empty export for {math}"
            );
            let (pptx, warnings) = render_pptx(&scene, &ExportOptions::default()).unwrap();
            assert!(pptx.starts_with(b"PK") && warnings.is_empty());
        }
    }
}
