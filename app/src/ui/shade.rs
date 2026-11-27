//! Shared paint helpers for CSS gradients and shadows.

/// `linear-gradient(135deg, from, to)` clipped to a rounded rect.
///
/// The stop position is linear in `(x + y)`, so a triangle fan with the stop
/// colour sampled per vertex is exact (no cell seams). The fan's border gets a
/// one-physical-pixel feather, matching epaint's own anti-aliasing; the old
/// implementation filled a square mesh and overpainted the corner tips, which
/// left hard, un-antialiased steps around icon badges and gradient buttons.
pub fn grad135(
    ui: &egui::Ui,
    rect: egui::Rect,
    radius: u8,
    c_from: egui::Color32,
    c_to: egui::Color32,
) {
    let span = (rect.width() + rect.height()).max(1.0);
    let painter = ui.painter();
    // Interpolate in *unmultiplied* space: `Color32` is premultiplied, so
    // lerping its raw channels darkens translucent stops.
    let (a, b) = (c_from.to_srgba_unmultiplied(), c_to.to_srgba_unmultiplied());
    let at = |p: egui::Pos2| -> egui::Color32 {
        let t = (((p.x - rect.min.x) + (p.y - rect.min.y)) / span).clamp(0.0, 1.0);
        let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
        egui::Color32::from_rgba_unmultiplied(
            l(a[0], b[0]),
            l(a[1], b[1]),
            l(a[2], b[2]),
            l(a[3], b[3]),
        )
    };

    let r = (radius as f32)
        .min(rect.width() * 0.5)
        .min(rect.height() * 0.5);
    let mut pts: Vec<egui::Pos2> = Vec::with_capacity(40);
    if r <= 0.0 {
        pts.extend([
            rect.left_top(),
            rect.right_top(),
            rect.right_bottom(),
            rect.left_bottom(),
        ]);
    } else {
        // Clockwise arcs: NW, NE, SE, SW. Endpoints land on the straight edges,
        // so the fan spans the edges without extra vertices.
        const N: usize = 8;
        let corners = [
            (rect.min.x + r, rect.min.y + r, std::f32::consts::PI),
            (rect.max.x - r, rect.min.y + r, std::f32::consts::PI * 1.5),
            (rect.max.x - r, rect.max.y - r, 0.0),
            (rect.min.x + r, rect.max.y - r, std::f32::consts::FRAC_PI_2),
        ];
        for (ox, oy, a0) in corners {
            for i in 0..=N {
                let ang = a0 + std::f32::consts::FRAC_PI_2 * (i as f32 / N as f32);
                let p = egui::pos2(ox + r * ang.cos(), oy + r * ang.sin());
                if pts
                    .last()
                    .is_none_or(|q: &egui::Pos2| q.distance(p) > 0.01)
                {
                    pts.push(p);
                }
            }
        }
        // Degenerate narrow rect: the last point can wrap onto the first.
        if pts.len() > 1 && pts[0].distance(*pts.last().unwrap()) <= 0.01 {
            pts.pop();
        }
    }

    let n = pts.len();
    let center = rect.center();
    // One physical pixel of feather, centred on the outline (epaint's metric).
    let half = 0.5 / ui.ctx().pixels_per_point();
    let mut mesh = egui::Mesh::default();
    for p in &pts {
        let outward = (*p - center).normalized();
        let inner = *p - outward * half;
        let outer = *p + outward * half;
        mesh.colored_vertex(inner, at(inner));
        mesh.colored_vertex(outer, egui::Color32::TRANSPARENT);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ii, ij) = (2 * i as u32, 2 * j as u32);
        // Feather: inner_i -> inner_j -> outer_j -> outer_i.
        mesh.add_triangle(ii, ij, ij + 1);
        mesh.add_triangle(ii, ij + 1, ii + 1);
    }
    let c = (2 * n) as u32;
    mesh.colored_vertex(center, at(center));
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.add_triangle(c, 2 * i as u32, 2 * j as u32);
    }
    painter.add(mesh);
}

/// Approximate `box-shadow: 0 <oy> <blur> rgba(rgb, alpha)` with expanding
/// translucent rings (cheap, and close enough for the 6-16px CSS blurs the
/// cards and buttons use).
pub fn soft_shadow(
    ui: &egui::Ui,
    rect: egui::Rect,
    radius: u8,
    color: egui::Color32,
    offset_y: f32,
    blur: f32,
    alpha: u8,
) {
    ui.painter()
        .extend(soft_shadow_shapes(rect, radius, color, offset_y, blur, alpha));
}

/// How many rings a shadow of this blur needs. One ring per pixel of blur,
/// clamped so a 2px drop stays cheap and a wide one does not stack forever.
fn shadow_steps(blur: f32) -> usize {
    blur.round().clamp(2.0, 16.0) as usize
}

/// Source-over alphas, outer ring first. Ring `i` is chosen from the coverage
/// already painted, so rounding a byte does not accumulate: the next ring
/// aims at `t * peak` from wherever the stack actually is.
fn shadow_ring_alphas(n: usize, peak: f32) -> Vec<u8> {
    let mut covered = 0.0f32;
    let mut out = Vec::with_capacity(n);
    for i in 1..=n {
        let want = (i as f32 / n as f32) * peak;
        let a = if covered >= 0.999 {
            0.0
        } else {
            ((want - covered) / (1.0 - covered)).clamp(0.0, 1.0)
        };
        let byte = (a * 255.0).round().clamp(0.0, 255.0) as u8;
        covered += (1.0 - covered) * (byte as f32 / 255.0);
        out.push(byte);
    }
    out
}

/// Same shadow as a shape list, so a caller can insert it *below* a fill that
/// is painted later (`Painter::set` on a reserved shape index).
pub fn soft_shadow_shapes(
    rect: egui::Rect,
    radius: u8,
    color: egui::Color32,
    offset_y: f32,
    blur: f32,
    alpha: u8,
) -> Vec<egui::Shape> {
    // CSS `box-shadow` fades the caster's colour out over `blur`. epaint's
    // `RectShape::blur_width` only feathers ~1px, so the penumbra is stacked
    // rings. Outer rings are painted first; `shadow_ring_alphas` picks each
    // byte from the coverage already painted so the stack ramps up to `alpha`.
    let n = shadow_steps(blur);
    let mut out = Vec::with_capacity(n);
    let peak = alpha as f32 / 255.0;
    let alphas = shadow_ring_alphas(n, peak);
    for (i, a) in alphas.into_iter().enumerate() {
        if a == 0 {
            continue;
        }
        let t = (i + 1) as f32 / n as f32; // 0 = outer edge, 1 = caster edge
        let grow = blur * (1.0 - t);
        let r = egui::Rect::from_min_max(
            rect.min + egui::vec2(-grow, offset_y - grow),
            rect.max + egui::vec2(grow, offset_y + grow),
        );
        out.push(egui::Shape::rect_filled(
            r,
            egui::CornerRadius::same((radius as f32 + grow).round() as u8),
            egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn composite(n: usize, peak: f32) -> Vec<f32> {
        let mut c = 0.0;
        let mut out = Vec::new();
        for a in shadow_ring_alphas(n, peak) {
            c += (1.0 - c) * (a as f32 / 255.0);
            out.push(c);
        }
        out
    }

    #[test]
    fn shadow_ramp_tracks_peak_below_opaque() {
        let peak = 40.0 / 255.0;
        let n = 16;
        let got = composite(n, peak);
        for i in 1..=n {
            let want = (i as f32 / n as f32) * peak;
            assert!(
                (got[i - 1] - want).abs() < 2.0 / 255.0,
                "ring {i}: got {} want {want}",
                got[i - 1]
            );
        }
    }

    #[test]
    fn shadow_steps_follow_blur() {
        assert_eq!(shadow_steps(0.2), 2);
        assert_eq!(shadow_steps(2.0), 2);
        assert_eq!(shadow_steps(16.0), 16);
        assert_eq!(shadow_steps(40.0), 16);
    }
}
