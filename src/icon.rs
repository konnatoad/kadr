pub fn egui_icon() -> egui::IconData {
    let size = 64u32;
    let rgba = icon_rgba(size);
    egui::IconData { rgba, width: size, height: size }
}

pub fn icon_rgba(size: u32) -> Vec<u8> {
    let fi   = size as f32;
    let half = fi / 2.0;
    let mut out = vec![0u8; (size * size * 4) as usize];

    for py in 0..size {
        for px in 0..size {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;
            let cx = fx - half;
            let cy = fy - half;

            let bg_d = rrect_sdf(cx, cy, half - 1.5, half - 1.5, half * 0.28);
            let bg_a = smoothstep(-0.7, 0.7, bg_d);
            if bg_a < 0.005 { continue; }

            let la = ka_alpha(fx, fy, fi);
            // Dark bg (12,12,18) → accent-blue letters (99,155,255)
            let r = lerp(12.0,  99.0, la) as u8;
            let g = lerp(12.0, 155.0, la) as u8;
            let b = lerp(18.0, 255.0, la) as u8;
            let a = (bg_a * 255.0) as u8;

            let idx = ((py * size + px) * 4) as usize;
            out[idx]     = r;
            out[idx + 1] = g;
            out[idx + 2] = b;
            out[idx + 3] = a;
        }
    }
    out
}

// ── "ka" letterforms ──────────────────────────────────────────────────────────
//
// Coordinates are expressed as fractions of `size` so they scale cleanly to
// every ICO resolution.  Key constraint: K's rightmost stroke extent must not
// overlap with a's leftmost stroke extent — verified analytically below.
//
// At 64 px (sw ≈ 4.0):
//   K right arm tip  = 0.380 × 64  = 24.3  → visible edge ≈ 24.3 + 4.0 + 0.8 = 29.1
//   a bowl left edge = bcx - (br + sw + 0.8) = 46.1 - 15.1 = 31.0  → 2 px gap  ✓
// At 32 px (sw = 2.0 clamped):
//   K right arm tip  = 12.2  → visible edge ≈ 15.0
//   a bowl left edge = 23.1 - 8.1 = 15.0                            → flush   ✓
// At 16 px (sw = 2.0 clamped): minor blend, acceptable at that size.

fn ka_alpha(px: f32, py: f32, size: f32) -> f32 {
    let sw = (size * 0.062).max(2.0);   // stroke half-width
    let t  = size * 0.145;              // top y
    let b  = size * 0.855;              // bottom y
    let m  = size * 0.500;              // mid y (K branch point)

    // ── K ────────────────────────────────────────────────────────────────────
    let ksx = size * 0.160;             // stem centre x
    let kax = size * 0.380;             // arm tips x

    let d_k = seg(px, py, ksx, t, ksx, b)          // vertical stem
        .min(seg(px, py, ksx, m, kax, t))           // upper diagonal
        .min(seg(px, py, ksx, m, kax, b));          // lower diagonal

    // ── a ────────────────────────────────────────────────────────────────────
    // Circle bowl (open on the right) + full-height stem tangent to the bowl.
    let bcx = size * 0.720;             // bowl centre x
    let bcy = size * 0.500;             // bowl centre y
    let br  = size * 0.163;             // bowl radius
    let sx  = bcx + br;                 // stem x (tangent at 3 o'clock)

    let dc  = ((px - bcx).powi(2) + (py - bcy).powi(2)).sqrt();
    let ang = (py - bcy).atan2(px - bcx);
    // Narrow opening where the stem meets the bowl (< ~16°)
    let d_bowl = if ang.abs() < 0.28 { f32::MAX } else { (dc - br).abs() };

    // Stem runs full icon height for clear legibility at 16 px
    let d_a = d_bowl.min(seg(px, py, sx, t, sx, b));

    smoothstep(sw - 0.8, sw + 0.8, d_k.min(d_a))
}

// ── Math helpers ──────────────────────────────────────────────────────────────

fn seg(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-6 {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }
    let t = (((px - x1) * dx + (py - y1) * dy) / len2).clamp(0.0, 1.0);
    ((px - x1 - t * dx).powi(2) + (py - y1 - t * dy).powi(2)).sqrt()
}

fn rrect_sdf(cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = cx.abs() - hw + r;
    let qy = cy.abs() - hh + r;
    qx.max(0.0).hypot(qy.max(0.0)) + qx.min(0.0).max(qy.min(0.0)) - r
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge1) / (edge0 - edge1)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
