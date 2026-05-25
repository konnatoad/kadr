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
    let sizes: &[u32] = &[16, 24, 32, 48, 64];
    let images: Vec<Vec<u8>> = sizes.iter().map(|&s| make_bmp_image(s)).collect();

    let header_size = 6u32 + sizes.len() as u32 * 16;
    let mut offsets = Vec::new();
    let mut off = header_size;
    for img in &images {
        offsets.push(off);
        off += img.len() as u32;
    }

    let mut ico: Vec<u8> = Vec::new();
    push_u16(&mut ico, 0);
    push_u16(&mut ico, 1);
    push_u16(&mut ico, sizes.len() as u16);

    for (i, &s) in sizes.iter().enumerate() {
        ico.push(if s >= 256 { 0 } else { s as u8 });
        ico.push(if s >= 256 { 0 } else { s as u8 });
        ico.push(0);
        ico.push(0);
        push_u16(&mut ico, 1);
        push_u16(&mut ico, 32);
        push_u32(&mut ico, images[i].len() as u32);
        push_u32(&mut ico, offsets[i]);
    }

    for img in &images {
        ico.extend_from_slice(img);
    }

    std::fs::write(path, ico).expect("failed to write icon");
}

fn make_bmp_image(size: u32) -> Vec<u8> {
    let pixels = make_icon_pixels(size);
    let and_stride = ((size + 31) / 32) * 4;
    let and_mask = vec![0u8; (and_stride * size) as usize];

    let mut bmp: Vec<u8> = Vec::new();
    push_u32(&mut bmp, 40);
    push_i32(&mut bmp, size as i32);
    push_i32(&mut bmp, (size * 2) as i32);
    push_u16(&mut bmp, 1);
    push_u16(&mut bmp, 32);
    for _ in 0..6 { push_u32(&mut bmp, 0); }
    bmp.extend_from_slice(&pixels);
    bmp.extend_from_slice(&and_mask);
    bmp
}

// BGRA, bottom-up row order
fn make_icon_pixels(size: u32) -> Vec<u8> {
    let fi = size as f32;
    let half = fi / 2.0;
    let mut out = vec![0u8; (size * size * 4) as usize];

    for py in 0..size {
        let bmp_row = size - 1 - py;
        for px in 0..size {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;
            let cx = fx - half;
            let cy = fy - half;

            let bg_d = rrect_sdf(cx, cy, half - 1.5, half - 1.5, half * 0.28);
            let bg_a = smoothstep(-0.7, 0.7, bg_d);
            if bg_a < 0.005 { continue; }

            let la = ka_alpha(fx, fy, fi);

            // Dark bg → accent-blue letters (BGRA order for BMP)
            let b = lerp(18.0, 255.0, la) as u8;
            let g = lerp(12.0, 155.0, la) as u8;
            let r = lerp(12.0,  99.0, la) as u8;
            let a = (bg_a * 255.0) as u8;

            let idx = ((bmp_row * size + px) * 4) as usize;
            out[idx]     = b;
            out[idx + 1] = g;
            out[idx + 2] = r;
            out[idx + 3] = a;
        }
    }
    out
}

fn ka_alpha(px: f32, py: f32, size: f32) -> f32 {
    let sw  = (size * 0.115).max(2.0);
    let pad = size * 0.150;
    let h   = size - 2.0 * pad;

    let k_stem_x = pad + sw * 0.85;
    let k_arm_x  = pad + size * 0.310;
    let k_top    = pad;
    let k_bot    = size - pad;
    let k_mid    = pad + h * 0.50;

    let d_k = seg(px, py, k_stem_x, k_top, k_stem_x, k_bot)
        .min(seg(px, py, k_stem_x, k_mid, k_arm_x, k_top))
        .min(seg(px, py, k_stem_x, k_mid, k_arm_x, k_bot));

    let a_ox       = pad + size * 0.410;
    let a_bowl_cx  = a_ox + size * 0.148;
    let a_bowl_cy  = pad + h * 0.50;
    let a_bowl_r   = h * 0.295;
    let a_stem_x   = a_ox + size * 0.290;
    let a_stem_top = pad + h * 0.17;

    let dist_c  = ((px - a_bowl_cx).powi(2) + (py - a_bowl_cy).powi(2)).sqrt();
    let angle   = (py - a_bowl_cy).atan2(px - a_bowl_cx);
    let opening = px > a_bowl_cx && angle.abs() < 0.50;
    let d_bowl  = if opening { f32::MAX } else { (dist_c - a_bowl_r).abs() };

    let d_a = d_bowl.min(seg(px, py, a_stem_x, a_stem_top, a_stem_x, k_bot));

    smoothstep(sw - 0.7, sw + 0.7, d_k.min(d_a))
}

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

fn push_u16(v: &mut Vec<u8>, n: u16) { v.extend_from_slice(&n.to_le_bytes()); }
fn push_u32(v: &mut Vec<u8>, n: u32) { v.extend_from_slice(&n.to_le_bytes()); }
fn push_i32(v: &mut Vec<u8>, n: i32) { v.extend_from_slice(&n.to_le_bytes()); }
