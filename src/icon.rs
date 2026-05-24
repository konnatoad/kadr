/// Generates the kadr icon as RGBA bytes.
/// Design: dark rounded square, white outline photo frame, sun + mountain silhouette inside.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let s     = size as f32;
    let half  = s / 2.0;

    // Outer icon: generous rounding (approx iOS-style squircle feel)
    let outer_r = s / 4.2;

    // Photo frame rectangle
    let margin  = s * 0.13;               // inset from icon edge to frame
    let fhalf   = half - margin;          // frame outer half-size
    let fb      = (s / 10.5).max(1.5);   // frame border thickness
    let fi_half = fhalf - fb;             // content area half-size
    let fr      = fhalf / 5.0;            // frame outer corner radius
    let fi_r    = (fi_half / 5.5).max(0.5);

    // Colors (RGBA)
    let bg     = [12u8, 12, 16, 255];
    let light  = [192u8, 196, 210, 255];
    let transp = [0u8,   0,   0,   0];

    let mnt_slope  = 1.30_f32;
    let mnt_lift   = fi_half * 0.30;

    let mut out = Vec::with_capacity((size * size * 4) as usize);

    for py in 0..size {
        for px in 0..size {
            let x  = px as f32 + 0.5;
            let y  = py as f32 + 0.5;
            let cx = x - half;
            let cy = y - half;
            let ax = cx.abs();
            let ay = cy.abs();

            // Outer rounded-square boundary + AA
            let outer_d = rrect_sdf(ax, ay, half - 0.5, outer_r);
            if outer_d > 0.8 {
                out.extend_from_slice(&transp);
                continue;
            }
            let aa = ((0.8 - outer_d.max(0.0)) / 0.8 * 255.0).clamp(0.0, 255.0) as u8;

            // Frame ring
            let d_out = rrect_sdf(ax, ay, fhalf,   fr);
            let d_in  = rrect_sdf(ax, ay, fi_half, fi_r);
            let on_frame   = d_out <= 0.6 && d_in > -0.6;
            let in_content = d_in <= -0.6;

            // Sun: filled circle, upper-right of content
            let sun_r  = fi_half * 0.21;
            let sun_ox = fi_half * 0.40;
            let sun_oy = -(fi_half * 0.40);
            let sun_d  = ((cx - sun_ox).powi(2) + (cy - sun_oy).powi(2)).sqrt();
            let on_sun = in_content && sun_d <= sun_r + 0.6;

            // Mountain: isoceles triangle, peak ~30% above centre
            // on_mnt = cy > |cx| * slope - lift  (region below the V-line)
            let on_mnt = in_content && !on_sun
                && cy > cx.abs() * mnt_slope - mnt_lift;

            let pixel = if on_frame || on_sun || on_mnt { light } else { bg };

            let a = ((pixel[3] as u32 * aa as u32) / 255) as u8;
            out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], a]);
        }
    }

    out
}

pub fn egui_icon() -> egui::IconData {
    let size = 64u32;
    let rgba = icon_rgba(size);
    egui::IconData { rgba, width: size, height: size }
}

/// Returns positive when outside, negative when inside.
fn rrect_sdf(ax: f32, ay: f32, half: f32, r: f32) -> f32 {
    let qx = (ax - (half - r)).max(0.0);
    let qy = (ay - (half - r)).max(0.0);
    (qx * qx + qy * qy).sqrt() - r
}
