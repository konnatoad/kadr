fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let ico_path = format!("{out_dir}/kadr.ico");
        generate_ico(&ico_path);

        let mut res = winres::WindowsResource::new();
        res.set_icon(&ico_path);
        res.set("ProductName", "kadr");
        res.set("FileDescription", "kadr image viewer");
        res.compile().expect("winres failed — is a C compiler (MSVC or MinGW) available?");
    }
}

fn generate_ico(path: &str) {
    let size: u32 = 32;
    let pixels = make_icon_pixels(size);
    let and_stride = ((size + 31) / 32) * 4;
    let and_mask = vec![0u8; (and_stride * size) as usize];
    let bmp_size = 40u32 + size * size * 4 + and_stride * size;

    let mut ico: Vec<u8> = Vec::new();
    // ICONDIR
    push_u16(&mut ico, 0); push_u16(&mut ico, 1); push_u16(&mut ico, 1);
    // ICONDIRENTRY
    ico.push(size as u8); ico.push(size as u8); ico.push(0); ico.push(0);
    push_u16(&mut ico, 1); push_u16(&mut ico, 32);
    push_u32(&mut ico, bmp_size);
    push_u32(&mut ico, 22);
    // BITMAPINFOHEADER
    push_u32(&mut ico, 40);
    push_i32(&mut ico, size as i32);
    push_i32(&mut ico, (size * 2) as i32);
    push_u16(&mut ico, 1); push_u16(&mut ico, 32);
    for _ in 0..6 { push_u32(&mut ico, 0); }
    // Pixel data + AND mask
    ico.extend_from_slice(&pixels);
    ico.extend_from_slice(&and_mask);

    std::fs::write(path, ico).expect("failed to write icon");
}

/// Same design as src/icon.rs but output is BGRA (Windows ICO format).
fn make_icon_pixels(size: u32) -> Vec<u8> {
    let s     = size as f32;
    let half  = s / 2.0;

    let outer_r = s / 4.2;
    let margin  = s * 0.13;
    let fhalf   = half - margin;
    let fb      = (s / 10.5_f32).max(1.5);
    let fi_half = fhalf - fb;
    let fr      = fhalf / 5.0;
    let fi_r    = (fi_half / 5.5_f32).max(0.5);

    // BGRA colors (B and R swapped vs RGBA in icon.rs)
    let bg    = [16u8, 12, 12, 255]; // rgb(12,12,16) → bgra
    let light = [210u8, 196, 192, 255]; // rgb(192,196,210) → bgra
    let transp = [0u8, 0, 0, 0];

    let mnt_slope = 1.30_f32;
    let mnt_lift  = fi_half * 0.30;

    let mut out = vec![0u8; (size * size * 4) as usize];

    for py in 0..size {
        for px in 0..size {
            let bmp_row = size - 1 - py; // BMP is bottom-up
            let idx = ((bmp_row * size + px) * 4) as usize;

            let x  = px as f32 + 0.5;
            let y  = py as f32 + 0.5;
            let cx = x - half;
            let cy = y - half;
            let ax = cx.abs();
            let ay = cy.abs();

            let outer_d = rrect_sdf(ax, ay, half - 0.5, outer_r);
            let aa = if outer_d > 0.8 {
                out[idx]   = transp[0];
                out[idx+1] = transp[1];
                out[idx+2] = transp[2];
                out[idx+3] = transp[3];
                continue;
            } else {
                ((0.8 - outer_d.max(0.0)) / 0.8 * 255.0).clamp(0.0, 255.0) as u8
            };

            let d_out = rrect_sdf(ax, ay, fhalf,   fr);
            let d_in  = rrect_sdf(ax, ay, fi_half, fi_r);
            let on_frame   = d_out <= 0.6 && d_in > -0.6;
            let in_content = d_in <= -0.6;

            let sun_r  = fi_half * 0.21;
            let sun_ox = fi_half * 0.40;
            let sun_oy = -(fi_half * 0.40);
            let sun_d  = ((cx - sun_ox).powi(2) + (cy - sun_oy).powi(2)).sqrt();
            let on_sun = in_content && sun_d <= sun_r + 0.6;

            let on_mnt = in_content && !on_sun
                && cy > cx.abs() * mnt_slope - mnt_lift;

            let pixel = if on_frame || on_sun || on_mnt { light } else { bg };

            out[idx]   = pixel[0]; // B
            out[idx+1] = pixel[1]; // G
            out[idx+2] = pixel[2]; // R
            out[idx+3] = ((pixel[3] as u32 * aa as u32) / 255) as u8;
        }
    }
    out
}

fn rrect_sdf(ax: f32, ay: f32, half: f32, r: f32) -> f32 {
    let qx = (ax - (half - r)).max(0.0);
    let qy = (ay - (half - r)).max(0.0);
    (qx * qx + qy * qy).sqrt() - r
}

fn push_u16(v: &mut Vec<u8>, n: u16) { v.extend_from_slice(&n.to_le_bytes()); }
fn push_u32(v: &mut Vec<u8>, n: u32) { v.extend_from_slice(&n.to_le_bytes()); }
fn push_i32(v: &mut Vec<u8>, n: i32) { v.extend_from_slice(&n.to_le_bytes()); }
