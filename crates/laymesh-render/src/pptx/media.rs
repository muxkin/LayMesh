//! Native PowerPoint media objects and their playback timing. No external links.
use super::*;
use laymesh_core::media::Cue;

impl Writer<'_> {
    fn media_relationship(&mut self, asset: &str, video: bool) -> Result<usize> {
        let resource = self
            .list
            .resources
            .get(asset)
            .ok_or_else(|| error("Missing PPTX media resource"))?;
        let extension = match resource.mime.as_str() {
            "video/mp4" => "mp4",
            "audio/mp4" => "m4a",
            "audio/mpeg" => "mp3",
            "audio/wav" => "wav",
            _ => {
                return Err(error(format!(
                    "Unsupported PPTX media MIME: {}",
                    resource.mime
                )));
            }
        };
        let kind = if video { "video" } else { "audio" };
        if let Some(i) = self
            .media
            .iter()
            .position(|m| m.extension == extension && m.kind == kind && m.data == *resource.data)
        {
            return Ok(i + 2);
        }
        self.media.push(Media {
            data: resource.data.as_ref().clone(),
            extension,
            kind,
        });
        Ok(self.media.len() + 1)
    }
    pub(super) fn video(
        &mut self,
        n: &DrawNode,
        t: Affine,
        opacity: f64,
        name: &str,
    ) -> Result<String> {
        let asset_id = n.hint["mediaId"]
            .as_str()
            .ok_or_else(|| error("Missing video asset ID"))?;
        let asset = self
            .list
            .media
            .assets
            .get(asset_id)
            .ok_or_else(|| error("Missing video metadata"))?;
        let cue = self
            .list
            .media
            .cues
            .iter()
            .find(|c| c.target.as_deref() == Some(name))
            .cloned()
            .unwrap_or_else(|| Cue {
                asset: asset_id.into(),
                scope: "page".into(),
                target: Some(name.into()),
                step: None,
                trigger: "click".into(),
                delay: 0.,
                trim_start: 0.,
                trim_end: asset.duration,
                looped: false,
                volume: 1.,
                muted: false,
                export_start: None,
                file: asset.src.clone(),
                loc: Loc::default(),
            });
        // Image placement is already resolved by the shared display list. A cover
        // fit or explicit crop becomes a native source rectangle, while contain
        // uses the smaller, centered video rectangle.
        fn first_image(n: &DrawNode, t: Affine) -> Option<(&str, Rect)> {
            match &n.kind {
                DrawKind::Image { resource, rect, .. } => {
                    Some((resource, t.transform_rect_bbox(*rect)))
                }
                DrawKind::Group { children, .. } => children
                    .iter()
                    .find_map(|c| first_image(c, t * c.transform)),
                _ => None,
            }
        }
        let (poster, full) =
            first_image(n, Affine::IDENTITY).ok_or_else(|| error("Missing video poster"))?;
        let mut data = self.image_data(poster)?;
        let b = &n.hint["mediaBounds"];
        let aperture = Rect::new(
            b[0].as_f64().unwrap_or(full.x0),
            b[1].as_f64().unwrap_or(full.y0),
            b[2].as_f64().unwrap_or(full.x1),
            b[3].as_f64().unwrap_or(full.y1),
        );
        let local = full.intersect(aperture);
        let mut crop = [
            (local.x0 - full.x0) / full.width(),
            (local.y0 - full.y0) / full.height(),
            (full.x1 - local.x1) / full.width(),
            (full.y1 - local.y1) / full.height(),
        ];
        if n.hint["mediaPosterCropped"] == true {
            let c = &n.hint["mediaCrop"];
            let x = c["offset"][0].as_f64().unwrap_or(0.);
            let y = c["offset"][1].as_f64().unwrap_or(0.);
            let w = c["size"][0].as_f64().unwrap_or(1.);
            let h = c["size"][1].as_f64().unwrap_or(1.);
            crop = [
                x + crop[0] * w,
                y + crop[1] * h,
                1. - x - w + crop[2] * w,
                1. - y - h + crop[3] * h,
            ];
            // Portable previews carry cropped poster pixels. Pad them back to the
            // full source extent so the same srcRect applies to poster and video.
            let im = image::load_from_memory(&data.bytes)
                .map_err(|e| error(e.to_string()))?
                .to_rgba8();
            let width = n.hint["mediaPosterSize"][0]
                .as_u64()
                .unwrap_or(im.width() as u64) as u32;
            let height = n.hint["mediaPosterSize"][1]
                .as_u64()
                .unwrap_or(im.height() as u64) as u32;
            if width as u64 * height as u64 > 100_000_000 {
                return Err(error("Video poster exceeds 100,000,000 pixels"));
            }
            let mut full = image::RgbaImage::new(width, height);
            image::imageops::overlay(
                &mut full,
                &im,
                (x * width as f64).round() as i64,
                (y * height as f64).round() as i64,
            );
            let mut bytes = Cursor::new(Vec::new());
            full.write_to(&mut bytes, image::ImageFormat::Png)
                .map_err(|e| error(e.to_string()))?;
            data.bytes = bytes.into_inner();
            data.extension = "png";
        }
        let src_rect = format!(
            "<a:srcRect l=\"{}\" t=\"{}\" r=\"{}\" b=\"{}\"/>",
            pct(crop[0]),
            pct(crop[1]),
            pct(crop[2]),
            pct(crop[3])
        );
        let (_, transform, _) = geometry::frame(t, local).unwrap_or_else(|| {
            let rect = t.transform_rect_bbox(local);
            (rect, xfrm(rect, 0.), Affine::IDENTITY)
        });
        fn unsupported_decoration(n: &DrawNode) -> bool {
            match &n.kind {
                DrawKind::Image { .. } => false,
                DrawKind::Group { children, mask, .. } => {
                    mask.is_some() || children.iter().any(unsupported_decoration)
                }
                _ => true,
            }
        }
        if !n.effects.is_empty()
            || geometry::frame(t, local).is_none()
            || opacity != 1.
            || unsupported_decoration(n)
        {
            self.warnings.push(Diagnostic::new("W_PPTX_MEDIA",format!("Video {name}: unsupported effects/opacity/shear/decorations omitted; native playback retained"),&cue.file,cue.loc));
        }
        let id = self.id();
        self.bounds.push(t.transform_rect_bbox(local));
        let poster_rid = self.image_relationship(data.bytes, data.extension);
        let media_rid = self.media_relationship(asset_id, true)?;
        let trim = self.trim(&cue)?;
        self.playback.push((id, cue, true));
        Ok(format!(
            "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{}\"><a:hlinkClick r:id=\"\" action=\"ppaction://media\"/></p:cNvPr><p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr><p:nvPr><a:videoFile r:link=\"rId{media_rid}\"/><p:extLst><p:ext uri=\"{{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}}\"><p14:media r:embed=\"media{media_rid}\">{trim}</p14:media></p:ext></p:extLst></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed=\"rId{poster_rid}\"/>{src_rect}<a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr>{transform}<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom><a:ln><a:noFill/></a:ln></p:spPr></p:pic>",
            escape(name)
        ))
    }
    fn trim(&self, c: &Cue) -> Result<String> {
        let a = self
            .list
            .media
            .assets
            .get(&c.asset)
            .ok_or_else(|| error("Missing media metadata"))?;
        // MS-PPTX CT_MediaTrim.end is removed duration, not the endpoint.
        Ok(format!(
            "<p14:trim st=\"{}\" end=\"{}\"/>",
            ms(c.trim_start),
            ms((a.duration - c.trim_end).max(0.))
        ))
    }
    pub(super) fn audio(&mut self) -> Result<String> {
        let mut out = String::new();
        for c in self
            .list
            .media
            .cues
            .iter()
            .filter(|c| c.target.is_none())
            .cloned()
            .collect::<Vec<_>>()
        {
            let id = self.id();
            let rid = self.media_relationship(&c.asset, false)?;
            let trim = self.trim(&c)?;
            // Kept outside the slide; audio remains selectable in PowerPoint's selection pane.
            let mut pixel = Cursor::new(Vec::new());
            image::RgbaImage::new(1, 1)
                .write_to(&mut pixel, image::ImageFormat::Png)
                .map_err(|e| error(e.to_string()))?;
            let pixel = pixel.into_inner();
            let poster = self.image_relationship(pixel, "png");
            out += &format!(
                "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"{}\"><a:hlinkClick r:id=\"\" action=\"ppaction://media\"/></p:cNvPr><p:cNvPicPr/><p:nvPr><a:audioFile r:link=\"rId{rid}\"/><p:extLst><p:ext uri=\"{{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}}\"><p14:media r:embed=\"media{rid}\">{trim}</p14:media></p:ext></p:extLst></p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed=\"rId{poster}\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x=\"-360000\" y=\"-360000\"/><a:ext cx=\"36000\" cy=\"36000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr></p:pic>",
                escape(&format!("{} audio {}", c.scope, c.asset))
            );
            self.playback.push((id, c, false));
        }
        Ok(out)
    }
    pub(super) fn timing(&self) -> String {
        if self.playback.is_empty() {
            return String::new();
        }
        let mut next = 2u32;
        let mut track = |id: u32, c: &Cue, video: bool| {
            next += 1;
            let tn = next;
            let tag = if video { "video" } else { "audio" };
            let condition = if c.trigger == "click" {
                format!(
                    "<p:cond evt=\"onClick\" delay=\"{}\"><p:tgtEl><p:spTgt spid=\"{id}\"/></p:tgtEl></p:cond>",
                    ms(c.delay)
                )
            } else {
                format!("<p:cond delay=\"{}\"/>", ms(c.delay))
            };
            let repeat = if c.looped {
                " repeatCount=\"indefinite\""
            } else {
                ""
            };
            format!(
                "<p:{tag}><p:cMediaNode vol=\"{}\" mute=\"{}\" numSld=\"{}\" showWhenStopped=\"{}\"><p:cTn id=\"{tn}\" dur=\"{}\" fill=\"hold\"{repeat}><p:stCondLst>{condition}</p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid=\"{id}\"/></p:tgtEl></p:cMediaNode></p:{tag}>",
                (c.volume * 100000.).round() as u32,
                u8::from(c.muted),
                1,
                u8::from(video),
                ms(c.trim_end - c.trim_start)
            )
        };
        let mut children = String::new();
        for (id, c, v) in self
            .playback
            .iter()
            .filter(|(_, c, _)| c.step.is_none() || c.trigger == "click")
        {
            children += &track(*id, c, *v);
        }
        let mut steps = String::new();
        if let Some(a) = &self.list.animation {
            for (i, step) in a.normal.steps.iter().enumerate() {
                let mut body = String::new();
                for (id, c, v) in self
                    .playback
                    .iter()
                    .filter(|(_, c, _)| c.step == Some(i) && c.trigger != "click")
                {
                    body += &track(*id, c, *v);
                }
                // Reserve stable, non-overlapping IDs for the media-only step sequence.
                let id = 1_000_000 + i;
                let trigger = if step.trigger == "click" {
                    "<p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>"
                } else {
                    "<p:cond delay=\"0\"/>"
                };
                steps += &format!(
                    "<p:par><p:cTn id=\"{id}\" dur=\"{}\" fill=\"hold\"><p:stCondLst>{trigger}</p:stCondLst>{}</p:cTn></p:par>",
                    ms(step.duration + step.hold),
                    if body.is_empty() {
                        String::new()
                    } else {
                        format!("<p:childTnLst>{body}</p:childTnLst>")
                    }
                );
            }
        }
        if !steps.is_empty() {
            children += &format!(
                "<p:seq concurrent=\"1\" nextAc=\"seek\"><p:cTn id=\"2\" dur=\"indefinite\" nodeType=\"mainSeq\"><p:childTnLst>{steps}</p:childTnLst></p:cTn><p:prevCondLst><p:cond evt=\"onPrev\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst><p:nextCondLst><p:cond evt=\"onNext\" delay=\"0\"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq>"
            );
        }
        format!(
            "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst>{children}</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"
        )
    }
}
fn ms(seconds: f64) -> u64 {
    (seconds * 1000.).round().max(0.) as u64
}

fn pct(v: f64) -> u32 {
    (v.clamp(0., 1.) * 100000.).round() as u32
}
