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

            // Rounded-rect background
            let bg_d = rrect_sdf(cx, cy, half - 1.5, half - 1.5, half * 0.26);
            let bg_a = smoothstep(0.8, -0.8, bg_d);
            if bg_a < 0.005 { continue; }

            let ha = heart_alpha(fx, fy, fi);

            // Kadr accent blue (99, 155, 255) on dark bg (10, 10, 14)
            let r = lerp(10.0,  99.0, ha) as u8;
            let g = lerp(10.0, 155.0, ha) as u8;
            let b = lerp(14.0, 255.0, ha) as u8;
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

// ── Heart ─────────────────────────────────────────────────────────────────────

/// Fills the algebraic heart curve  (x²+y²−1)³ − x²y³ = 0
/// with a slightly wavy edge for a hand-drawn feel.
fn heart_alpha(px: f32, py: f32, size: f32) -> f32 {
    let cx    = size * 0.50;
    let cy    = size * 0.53; // shifted down slightly so the tip has room
    let scale = size * 0.37;

    // Normalised coords, y pointing up
    let x  = (px - cx) / scale;
    let y  = -(py - cy) / scale;
    let x2 = x * x;
    let y2 = y * y;
    let r2 = x2 + y2;

    // Algebraic heart: f < 0 → inside, f > 0 → outside
    let f  = (r2 - 1.0).powi(3) - x2 * y2 * y;

    // Gradient → approximate signed distance in pixels
    let gx   = 6.0 * x * (r2 - 1.0).powi(2) - 2.0 * x * y2 * y;
    let gy   = 6.0 * y * (r2 - 1.0).powi(2) - 3.0 * x2 * y2;
    let grad = (gx * gx + gy * gy).sqrt().max(0.01);
    let dist = (f / grad) * scale;

    // Wavy boundary: two harmonics give an organic, hand-drawn edge
    let angle    = y.atan2(x);
    let wavy     = size * 0.013
        * ((angle * 3.1).sin() + 0.5 * (angle * 6.3 + 0.9).cos());
    let d_wavy   = dist + wavy;

    smoothstep(1.5, -1.5, d_wavy)
}

// ── Math helpers ──────────────────────────────────────────────────────────────

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
