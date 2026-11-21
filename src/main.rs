#![windows_subsystem = "windows"]
// Shamsi Widget — Rust/egui Persian (Jalali) calendar widget.
// Rebuilt from scratch in the same style as Audio Widget: borderless semi-transparent
// glass card, tray-only icon, settings panel. Persian text is shaped by Windows GDI
// (Vazirmatn embedded) and shown as textures, since egui has no bidi/shaping.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

const PALETTE: [(&str, Color32); 6] = [
    ("blue", Color32::from_rgb(47, 111, 237)),
    ("cyan", Color32::from_rgb(77, 208, 225)),
    ("green", Color32::from_rgb(80, 220, 140)),
    ("amber", Color32::from_rgb(255, 180, 70)),
    ("violet", Color32::from_rgb(170, 120, 255)),
    ("white", Color32::from_rgb(230, 230, 230)),
];
const SIZES: [f32; 3] = [170.0, 190.0, 220.0];
const BGS: [(u8,u8,u8); 5] = [(255,255,255),(235,235,235),(30,30,30),(10,10,10),(24,42,64)];
const SETTINGS_H: f32 = 286.0;
const MONTHS: [&str; 12] = ["فروردین", "اردیبهشت", "خرداد", "تیر", "مرداد", "شهریور", "مهر", "آبان", "آذر", "دی", "بهمن", "اسفند"];
const WEEK_FULL: [&str; 7] = ["شنبه", "یکشنبه", "دوشنبه", "سه‌شنبه", "چهارشنبه", "پنجشنبه", "جمعه"];
const WEEK_SHORT: [&str; 7] = ["ش", "ی", "د", "س", "چ", "پ", "ج"];

fn fa(n: i32) -> String {
    let digits = ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'];
    let s = n.abs().to_string();
    let mut out = String::new();
    if n < 0 { out.push('−'); }
    for ch in s.chars() { out.push(digits[ch as usize - '0' as usize]); }
    out
}

// --- Jalali <-> Gregorian (standard algorithm) ---
fn g2j(gy: i32, gm: i32, gd: i32) -> (i32, i32, i32) {
    let g_d_m = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let gy2 = if gm > 2 { gy + 1 } else { gy };
    let mut days = 355666 + 365 * gy + (gy2 + 3) / 4 - (gy2 + 99) / 100 + (gy2 + 399) / 400 + gd + g_d_m[(gm - 1) as usize];
    let mut jy = -1595 + 33 * (days / 12053);
    days %= 12053;
    jy += 4 * (days / 1461);
    days %= 1461;
    if days > 365 { jy += (days - 1) / 365; days = (days - 1) % 365; }
    if days < 186 { (jy, 1 + days / 31, 1 + days % 31) } else { (jy, 7 + (days - 186) / 30, 1 + (days - 186) % 30) }
}
fn days_from_civil(y: i32, m: i32, d: i32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn j2g(jy: i32, jm: i32, jd: i32) -> (i32, i32, i32) {
    let jy2 = jy + 1595;
    let mut days = -355668 + 365 * jy2 + (jy2 / 33) * 8 + ((jy2 % 33) + 3) / 4 + jd
        + if jm < 7 { (jm - 1) * 31 } else { (jm - 7) * 30 + 186 };
    let mut gy = 400 * (days / 146097);
    days %= 146097;
    if days > 36524 { gy += 100 * (days / 36524); days %= 36524; if days >= 365 { days += 1; } }
    gy += 4 * (days / 1461);
    days %= 1461;
    if days > 365 { gy += (days - 1) / 365; days = (days - 1) % 365; }
    let mut gd = days + 1;
    let leap = (gy % 4 == 0 && gy % 100 != 0) || gy % 400 == 0;
    let sal_a = [0, 31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut gm = 1;
    while gm < 13 && gd > sal_a[gm as usize] { gd -= sal_a[gm as usize]; gm += 1; }
    (gy, gm, gd)
}
fn j_month_len(jy: i32, jm: i32) -> i32 {
    if jm <= 6 { 31 } else if jm <= 11 { 30 } else {
        let a = days_from_civil(j2g(jy, 12, 1).0, j2g(jy, 12, 1).1, j2g(jy, 12, 1).2);
        let b = days_from_civil(j2g(jy + 1, 1, 1).0, j2g(jy + 1, 1, 1).1, j2g(jy + 1, 1, 1).2);
        if b - a == 30 { 30 } else { 29 }
    }
}
fn weekday_idx(gy: i32, gm: i32, gd: i32) -> usize {
    ((days_from_civil(gy, gm, gd) + 5).rem_euclid(7)) as usize
}
fn today_j() -> (i32, i32, i32) {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    // local date via Win32 GetLocalTime
    #[repr(C)]
    #[derive(Default)]
    struct SYSTEMTIME { w_year: u16, w_month: u16, w_dow: u16, w_day: u16, w_hour: u16, w_minute: u16, w_second: u16, w_millis: u16 }
    extern "system" { fn GetLocalTime(st: *mut SYSTEMTIME); }
    let mut st = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut st); }
    let _ = now;
    g2j(st.w_year as i32, st.w_month as i32, st.w_day as i32)
}

// --- settings ---
#[derive(Clone, Debug, PartialEq)]
struct Settings { accent: usize, opacity: u8, radius: u8, size: usize, bg: usize, dark: bool, ontop: bool, autostart: bool, px: i32, py: i32 }
impl Default for Settings {
    fn default() -> Self { Self { accent: 0, opacity: 235, radius: 22, size: 1, bg: 0, dark: false, ontop: true, autostart: false, px: 160, py: 160 } }
}
fn cfg_path() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(std::env::var("APPDATA").unwrap_or_default());
    p.push("ShamsiWidget"); p.push("settings.ini"); p
}
fn load_settings() -> Settings {
    let mut s = Settings::default();
    if let Ok(txt) = std::fs::read_to_string(cfg_path()) {
        for line in txt.lines() {
            let mut it = line.splitn(2, '=');
            let k = it.next().unwrap_or("").trim();
            let v = it.next().unwrap_or("").trim();
            match k {
                "accent" => s.accent = v.parse().unwrap_or(0),
                "opacity" => s.opacity = v.parse().unwrap_or(235),
                "radius" => s.radius = v.parse().unwrap_or(22),
                "size" => s.size = v.parse().unwrap_or(1),
                "bg" => s.bg = v.parse().unwrap_or(0),
                "dark" => s.dark = v == "1",
                "ontop" => s.ontop = v == "1",
                "autostart" => s.autostart = v == "1",
                "px" => s.px = v.parse().unwrap_or(160),
                "py" => s.py = v.parse().unwrap_or(160),
                _ => {}
            }
        }
    }
    s.accent = s.accent.min(PALETTE.len() - 1);
    s.size = s.size.min(SIZES.len() - 1);
    s.bg = s.bg.min(BGS.len() - 1);
    s
}
fn save_settings(s: &Settings) {
    let p = cfg_path();
    if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
    let body = format!("accent={}\nopacity={}\nradius={}\nbg={}\nsize={}\ndark={}\nontop={}\nautostart={}\npx={}\npy={}\n",
        s.accent, s.opacity, s.radius, s.bg, s.size, s.dark as u8, s.ontop as u8, s.autostart as u8, s.px, s.py);
    let _ = std::fs::write(p, body);
}
fn set_autostart(on: bool) {
    use std::os::windows::process::CommandExt;
    const NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe().unwrap_or_default();
    let key = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    if on {
        let _ = std::process::Command::new("reg").args(["add", key, "/v", "ShamsiWidget", "/t", "REG_SZ", "/d", &exe.to_string_lossy(), "/f"]).creation_flags(NO_WINDOW).status();
    } else {
        let _ = std::process::Command::new("reg").args(["delete", key, "/v", "ShamsiWidget", "/f"]).creation_flags(NO_WINDOW).status();
    }
}

// --- Win32: tray + GDI text shaping ---
#[repr(C)] #[derive(Default, Clone, Copy)] struct WRECT { l: i32, t: i32, r: i32, b: i32 }
#[repr(C)]
struct NOTIFYICONDATAW { cbSize: u32, hWnd: isize, uID: u32, uFlags: u32, uCallbackMessage: u32, hIcon: isize, szTip: [u16; 128] }
#[repr(C)]
struct WNDCLASSW { style: u32, lpfnWndProc: extern "system" fn(isize, u32, usize, isize) -> isize, cbClsExtra: i32, cbWndExtra: i32, hInstance: isize, hIcon: isize, hCursor: isize, hbrBackground: isize, lpszMenuName: *const u16, lpszClassName: *const u16 }
#[repr(C)]
struct BITMAPINFOHEADER { biSize: u32, biWidth: i32, biHeight: i32, biPlanes: u16, biBitCount: u16, biCompression: u32, biSizeImage: u32, biXPelsPerMeter: i32, biYPelsPerMeter: i32, biClrUsed: u32, biClrImportant: u32 }
#[repr(C)]
struct BITMAPINFO { bmiHeader: BITMAPINFOHEADER, bmiColors: [u32; 1] }
#[link(name = "shell32")]
extern "system" { fn Shell_NotifyIconW(msg: u32, data: *mut NOTIFYICONDATAW) -> i32; }
#[link(name = "user32")]
extern "system" {
    fn LoadImageW(h: isize, name: *const u16, typ: u32, cx: i32, cy: i32, flags: u32) -> isize;
    fn RegisterClassW(wc: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(ex: u32, cls: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: isize, menu: isize, hinst: isize, param: *mut std::ffi::c_void) -> isize;
    fn DefWindowProcW(h: isize, msg: u32, w: usize, l: isize) -> isize;
    fn DestroyIcon(h: isize) -> i32;
    fn GetModuleHandleW(name: *const u16) -> isize;
    fn SetWindowRgn(h: isize, rgn: isize, redraw: i32) -> i32;
    fn FindWindowW(cls: *const u16, name: *const u16) -> isize;
}
#[link(name = "gdi32")]
extern "system" {
    fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, ew: i32, eh: i32) -> isize;
}
#[link(name = "gdi32")]
extern "system" {
    fn AddFontMemResourceEx(pb: *const u8, cj: u32, pv: *mut u32, pc: *mut u32) -> isize;
    fn CreateFontW(h: i32, w: i32, esc: i32, orient: i32, weight: i32, italic: u32, underline: u32, strike: u32, charset: u32, outprec: u32, clipprec: u32, quality: u32, pitch: u32, face: *const u16) -> isize;
    fn CreateCompatibleDC(hdc: isize) -> isize;
    fn CreateDIBSection(hdc: isize, bmi: *const BITMAPINFO, usage: u32, bits: *mut *mut u8, sec: isize, off: u32) -> isize;
    fn SelectObject(hdc: isize, h: isize) -> isize;
    fn DeleteObject(h: isize) -> i32;
    fn DeleteDC(hdc: isize) -> i32;
    fn DrawTextW(hdc: isize, txt: *const u16, n: i32, r: *mut WRECT, fmt: u32) -> i32;
    fn SetTextColor(hdc: isize, c: u32) -> u32;
    fn SetBkColor(hdc: isize, c: u32) -> u32;
    fn SetBkMode(hdc: isize, m: i32) -> i32;
}
use std::sync::atomic::{AtomicU8, Ordering};
static CAL_RGN_KEY: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
static TRAY_CMD: AtomicU8 = AtomicU8::new(0); // 1=toggle, 2=exit
static mut TRAY_HWND: isize = 0;
static mut TRAY_ICON: isize = 0;
const WM_TRAY: u32 = 0x0400 + 1;
extern "system" fn tray_wndproc(h: isize, msg: u32, w: usize, l: isize) -> isize {
    if msg == WM_TRAY {
        let ev = (l as u32) & 0xffff;
        if ev == 0x0203 { TRAY_CMD.store(1, Ordering::SeqCst); }
        else if ev == 0x0205 { TRAY_CMD.store(2, Ordering::SeqCst); }
        return 0;
    }
    unsafe { DefWindowProcW(h, msg, w, l) }
}
fn tray_init() {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls: Vec<u16> = "SCWTrayCls".encode_utf16().chain(Some(0)).collect();
        let wc = WNDCLASSW { style: 0, lpfnWndProc: tray_wndproc, cbClsExtra: 0, cbWndExtra: 0, hInstance: hinst, hIcon: 0, hCursor: 0, hbrBackground: 0, lpszMenuName: std::ptr::null(), lpszClassName: cls.as_ptr() };
        RegisterClassW(&wc);
        TRAY_HWND = CreateWindowExW(0, cls.as_ptr(), cls.as_ptr(), 0, 0, 0, 0, 0, -3, 0, hinst, std::ptr::null_mut());
        let exe = std::env::current_exe().unwrap_or_default();
        let ico = exe.parent().map(|p| p.join("widget.ico")).unwrap_or_default();
        let wide: Vec<u16> = ico.to_string_lossy().encode_utf16().chain(Some(0)).collect();
        TRAY_ICON = LoadImageW(0, wide.as_ptr(), 1, 0, 0, 0x10);
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = TRAY_HWND; nid.uID = 1;
        nid.uFlags = 0x1 | 0x2 | 0x4;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = TRAY_ICON;
        let tip: Vec<u16> = "تقویم شمسی".encode_utf16().chain(Some(0)).collect();
        nid.szTip[..tip.len()].copy_from_slice(&tip);
        Shell_NotifyIconW(0, &mut nid);
    }
}
fn tray_remove() {
    unsafe {
        if TRAY_HWND != 0 {
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = TRAY_HWND; nid.uID = 1;
            Shell_NotifyIconW(2, &mut nid);
        }
        if TRAY_ICON != 0 { DestroyIcon(TRAY_ICON); TRAY_ICON = 0; }
    }
}
fn fonts_init() {
    unsafe {
        let mut n = 0u32;
        let m = include_bytes!("../fonts/vazirmatn_medium.ttf");
        AddFontMemResourceEx(m.as_ptr(), m.len() as u32, std::ptr::null_mut(), &mut n);
        let b = include_bytes!("../fonts/vazirmatn_bold.ttf");
        AddFontMemResourceEx(b.as_ptr(), b.len() as u32, std::ptr::null_mut(), &mut n);
    }
}
const DT_SINGLELINE: u32 = 0x20;
const DT_CALCRECT: u32 = 0x400;
const DT_NOPREFIX: u32 = 0x800;
const DT_RTLREADING: u32 = 0x20000;
const DT_CENTER: u32 = 0x1;
const DT_VCENTER: u32 = 0x4;
// Render Persian/RTL text with Windows shaping into an RGBA bitmap (white-on-black -> alpha).
// Supersampled: the glyphs are drawn at SS x the requested pixel size and box-downscaled, so the
// coverage ramp has SS*SS times more levels -> small Vazirmatn text stops looking chunky.
// Snap a text image rect onto the physical pixel grid: 1 texture texel = 1 screen pixel,
// no fractional offset -> egui samples it 1:1 and glyphs stay sharp.
fn snap_rect(center: Pos2, sz: Vec2, ppp: f32) -> Rect {
    let sz = Vec2::new((sz.x * ppp).round() / ppp, (sz.y * ppp).round() / ppp);
    let mn = Pos2::new(((center.x - sz.x * 0.5) * ppp).round() / ppp, ((center.y - sz.y * 0.5) * ppp).round() / ppp);
    Rect::from_min_size(mn, sz)
}

const SS: i32 = 3;
const GAMMA: f32 = 0.88; // >1 hardens the edge ramp a little (crisper, less washed out)

fn gdi_text(text: &str, px: i32, bold: bool, color: Color32) -> (Vec<u8>, usize, usize) {
    let px = px.max(4);
    unsafe {
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let face: Vec<u16> = if bold { "Vazirmatn" } else { "Vazirmatn Medium" }.encode_utf16().chain(Some(0)).collect();
        let hdc = CreateCompatibleDC(0);
        let font = CreateFontW(-px * SS, 0, 0, 0, if bold { 700 } else { 500 }, 0, 0, 0, 1, 0, 0, 4, 0, face.as_ptr());
        let oldf = SelectObject(hdc, font);
        let mut r = WRECT::default();
        DrawTextW(hdc, wide.as_ptr(), -1, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX | DT_RTLREADING);
        let w = ((r.r - r.l).max(SS) / SS + 4) as i32;   // final size, 2px padding each side
        let h = ((r.b - r.t).max(SS) / SS + 4) as i32;
        let (sw, sh) = (w * SS, h * SS);
        let bmi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: sw, biHeight: -sh, biPlanes: 1, biBitCount: 32, biCompression: 0, biSizeImage: 0, biXPelsPerMeter: 0, biYPelsPerMeter: 0, biClrUsed: 0, biClrImportant: 0 }, bmiColors: [0] };
        let mut bits: *mut u8 = std::ptr::null_mut();
        let dib = CreateDIBSection(hdc, &bmi, 0, &mut bits, 0, 0);
        let oldb = SelectObject(hdc, dib);
        std::ptr::write_bytes(bits, 0, (sw * sh * 4) as usize);
        SetBkMode(hdc, 2);
        SetBkColor(hdc, 0);
        SetTextColor(hdc, 0x00FF_FFFF);
        let mut dr = WRECT { l: 0, t: 0, r: sw, b: sh };
        DrawTextW(hdc, wide.as_ptr(), -1, &mut dr, DT_SINGLELINE | DT_NOPREFIX | DT_RTLREADING | DT_CENTER | DT_VCENTER);
        // box-downscale SS*SS -> 1, alpha from the blue channel, then gamma-tighten the ramp
        let mut lut = [0u8; 256];
        for (i, e) in lut.iter_mut().enumerate() {
            *e = (255.0 * (i as f32 / 255.0).powf(GAMMA)).round().clamp(0.0, 255.0) as u8;
        }
        let mut out = vec![0u8; (w * h * 4) as usize];
        let n = (SS * SS) as u32;
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0u32;
                for sy in 0..SS {
                    for sx in 0..SS {
                        acc += *bits.add((((y * SS + sy) * sw + x * SS + sx) * 4 + 2) as usize) as u32;
                    }
                }
                let a = lut[(acc / n) as usize];
                let o = ((y * w + x) * 4) as usize;
                out[o] = (color.r() as u32 * a as u32 / 255) as u8;
                out[o + 1] = (color.g() as u32 * a as u32 / 255) as u8;
                out[o + 2] = (color.b() as u32 * a as u32 / 255) as u8;
                out[o + 3] = a;
            }
        }
        SelectObject(hdc, oldb);
        SelectObject(hdc, oldf);
        DeleteObject(dib);
        DeleteObject(font);
        DeleteDC(hdc);
        (out, w as usize, h as usize)
    }
}

// --- app ---
struct App {
    set: Settings,
    applied: Option<(usize, bool, bool)>,
    show_settings: bool,
    cal_open: bool,
    cal_y: i32, cal_m: i32,
    tray_show: bool, hide_cnt: u32,
    tex: HashMap<(String, i32, bool, u32), egui::TextureHandle>,
    pos_dirty: bool,
    last_day: (i32, i32, i32),
    tick: Instant,
}
impl App {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        fonts_init();
        tray_init();
        let (jy, jm, jd) = today_j();
        Self {
            set: load_settings(), applied: None,
            show_settings: std::env::var("SCW_SETTINGS").is_ok(),
            cal_open: std::env::var("SCW_CAL").is_ok(),
            cal_y: jy, cal_m: jm,
            tray_show: false, hide_cnt: 0,
            tex: HashMap::new(), pos_dirty: false,
            last_day: (jy, jm, jd), tick: Instant::now(),
        }
    }
    fn colors(&self) -> (Color32, Color32, Color32) {
        let a = self.set.opacity;
        let (r, g, b) = BGS[self.set.bg];
        let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
        let bgc = Color32::from_rgba_unmultiplied(r, g, b, a);
        if lum < 128.0 {
            (bgc, Color32::from_rgb(255, 255, 255), Color32::from_rgb(204, 204, 204))
        } else {
            (bgc, Color32::from_rgb(0, 0, 0), Color32::from_rgb(68, 68, 68))
        }
    }
    fn text(&mut self, ctx: &egui::Context, s: &str, px: i32, bold: bool, c: Color32) -> (egui::TextureHandle, Vec2) {
        let ppp = ctx.pixels_per_point();
        let key = (s.to_string(), (px as f32 * ppp).round() as i32, bold, u32::from_le_bytes(c.to_array()));
        if !self.tex.contains_key(&key) {
            let (bytes, w, h) = gdi_text(s, (px as f32 * ppp).round() as i32, bold, c);
            let img = egui::ColorImage::from_rgba_premultiplied([w, h], &bytes);
            let t = ctx.load_texture(format!("t{}", self.tex.len()), img, egui::TextureOptions::LINEAR);
            self.tex.insert(key.clone(), t);
        }
        let t = self.tex[&key].clone();
        let sz = t.size_vec2() / ppp;
        (t, sz)
    }
    fn put_text(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, s: &str, px: i32, bold: bool, c: Color32, center: Pos2) -> Rect {
        let (t, sz) = self.text(ctx, s, px, bold, c);
        let r = snap_rect(center, sz, ctx.pixels_per_point());
        ui.put(r, egui::Image::new((t.id(), sz)));
        r
    }
    fn win_h(&self) -> f32 { SIZES[self.set.size] + if self.show_settings { SETTINGS_H } else { 0.0 } }
}

fn paint_gear(p: &egui::Painter, r: Rect, c: Color32) {
    let (cx, cy, rad) = (r.center().x, r.center().y, r.width() * 0.28);
    p.circle_stroke(Pos2::new(cx, cy), rad, Stroke::new(1.6, c));
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        p.line_segment([Pos2::new(cx + rad * 1.1 * a.cos(), cy + rad * 1.1 * a.sin()), Pos2::new(cx + rad * 1.6 * a.cos(), cy + rad * 1.6 * a.sin())], Stroke::new(1.6, c));
    }
}
fn paint_min(p: &egui::Painter, r: Rect, c: Color32) {
    p.line_segment([Pos2::new(r.left() + r.width() * 0.2, r.center().y), Pos2::new(r.right() - r.width() * 0.2, r.center().y)], Stroke::new(2.0, c));
}
fn paint_close(p: &egui::Painter, r: Rect, c: Color32) {
    p.line_segment([r.left_top() + Vec2::new(r.width() * 0.2, r.height() * 0.2), r.right_bottom() - Vec2::new(r.width() * 0.2, r.height() * 0.2)], Stroke::new(2.0, c));
    p.line_segment([Pos2::new(r.right() - r.width() * 0.2, r.top() + r.height() * 0.2), Pos2::new(r.left() + r.width() * 0.2, r.bottom() - r.height() * 0.2)], Stroke::new(2.0, c));
}
fn paint_arrow(p: &egui::Painter, r: Rect, c: Color32, left: bool) {
    let (cx, cy) = (r.center().x, r.center().y);
    let s = r.width() * 0.22;
    let dir = if left { -1.0 } else { 1.0 };
    p.line_segment([Pos2::new(cx - dir * s, cy - s), Pos2::new(cx + dir * s, cy)], Stroke::new(2.0, c));
    p.line_segment([Pos2::new(cx + dir * s, cy), Pos2::new(cx - dir * s, cy + s)], Stroke::new(2.0, c));
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] { [0.0, 0.0, 0.0, 0.0] }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match TRAY_CMD.swap(0, Ordering::SeqCst) {
            1 => {
                self.tray_show = !self.tray_show;
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(!self.tray_show));
                if self.tray_show { ctx.send_viewport_cmd(egui::ViewportCommand::Focus); }
            }
            2 => { tray_remove(); save_settings(&self.set); ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
            _ => {}
        }
        self.hide_cnt += 1;
        if self.hide_cnt == 3 && std::env::var("SCW_HIDE").is_ok() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        if self.hide_cnt == 2 {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(self.set.px as f32, self.set.py as f32)));
        }
        // apply settings side-effects once per change
        let want = (self.set.size, self.set.ontop, self.set.autostart);
        if self.applied.as_ref() != Some(&want) {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(SIZES[self.set.size], self.win_h())));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.set.ontop { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal }));
            set_autostart(self.set.autostart);
            self.applied = Some(want);
            save_settings(&self.set);
        }
        // roll over at midnight
        if self.tick.elapsed() > Duration::from_secs(20) {
            self.tick = Instant::now();
            let t = today_j();
            if t != self.last_day { self.last_day = t; self.cal_y = t.0; self.cal_m = t.1; }
        }
        let (bg, primary, secondary) = self.colors();
        let accent = PALETTE[self.set.accent].1;
        let sz = SIZES[self.set.size];
        let (ty, tm, td) = self.last_day;

        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ctx, |ui| {
            ui.set_min_size(Vec2::new(sz, self.win_h()));
            let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(sz, sz), Sense::click_and_drag());
            let p = ui.painter();
            p.rect_filled(card_rect, Rounding::same(self.set.radius), bg);
            // top-right controls
            let bs = 16.0;
            let close_r = Rect::from_min_size(Pos2::new(card_rect.right() - bs - 8.0, card_rect.top() + 8.0), Vec2::splat(bs));
            let min_r = Rect::from_min_size(Pos2::new(close_r.left() - bs - 6.0, close_r.top()), Vec2::splat(bs));
            let gear_r = Rect::from_min_size(Pos2::new(min_r.left() - bs - 6.0, close_r.top()), Vec2::splat(bs));
            paint_close(p, close_r, secondary);
            paint_min(p, min_r, secondary);
            paint_gear(p, gear_r, secondary);
            if ui.interact(close_r, egui::Id::new("scw_close"), Sense::click()).clicked() {
                tray_remove(); save_settings(&self.set); ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.interact(min_r, egui::Id::new("scw_min"), Sense::click()).clicked() {
                self.tray_show = false; ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            if ui.interact(gear_r, egui::Id::new("scw_gear"), Sense::click()).clicked() {
                self.show_settings = !self.show_settings;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(sz, self.win_h())));
            }
            // date texts
            let (gy, gm, gd) = j2g(ty, tm, td);
            let wd = WEEK_FULL[weekday_idx(gy, gm, gd)];
            let px_s = (sz * 0.102) as i32;
            let px_b = (sz * 0.30) as i32;
            self.put_text(ui, ctx, wd, px_s, true, accent, Pos2::new(card_rect.center().x, card_rect.top() + sz * 0.20));
            ui.painter().rect_filled(Rect::from_center_size(Pos2::new(card_rect.center().x, card_rect.top() + sz * 0.62), Vec2::new(sz * 0.30, 3.0)), Rounding::same(2u8), accent);
            self.put_text(ui, ctx, &fa(td), px_b, true, primary, Pos2::new(card_rect.center().x, card_rect.top() + sz * 0.47));
            self.put_text(ui, ctx, &format!("{} {}", MONTHS[(tm - 1) as usize], fa(ty)), px_s, false, secondary, Pos2::new(card_rect.center().x, card_rect.top() + sz * 0.78));
            // drag / click  (native window drag: smooth, follows the mouse 1:1)
            if resp.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                self.pos_dirty = true;
            }
            if self.pos_dirty {
                ctx.request_repaint();
                if !ctx.input(|i| i.pointer.any_down()) {
                    // ViewportInfo::outer_rect is already in POINTS/egui coords -> no ppp divide
                    if let Some(r) = ctx.input(|i| i.viewport().outer_rect) {
                        self.set.px = r.min.x.round() as i32;
                        self.set.py = r.min.y.round() as i32;
                        save_settings(&self.set);
                    }
                    self.pos_dirty = false;
                }
            }
            if resp.clicked() && !ui.rect_contains_pointer(close_r) && !ui.rect_contains_pointer(min_r) && !ui.rect_contains_pointer(gear_r) {
                self.cal_open = true; self.cal_y = ty; self.cal_m = tm;
            }
            // settings panel
            if self.show_settings {
                ui.add_space(8.0);
                let (sr, _) = ui.allocate_exact_size(Vec2::new(sz, SETTINGS_H - 8.0), Sense::hover());
                ui.painter().rect_filled(sr, Rounding::same(self.set.radius), { let (r, g, b) = BGS[self.set.bg]; Color32::from_rgba_unmultiplied(r, g, b, self.set.opacity) });
                ui.allocate_ui_at_rect(sr.shrink(12.0), |cui| {
                cui.spacing_mut().item_spacing = Vec2::new(6.0, 7.0);
                let lab = |cui: &mut egui::Ui, app: &mut App, s: &str| {
                    let (t, sz2) = app.text(ctx, s, 12, false, secondary);
                    cui.add(egui::Image::new((t.id(), sz2)));
                };
                cui.horizontal(|h| {
                    lab(h, self, "رنگ");
                    for (i, (_, c)) in PALETTE.iter().enumerate() {
                        let (r, rr) = h.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                        h.painter().rect_filled(r, Rounding::same(4u8), *c);
                        if self.set.accent == i { h.painter().rect_stroke(r, Rounding::same(4u8), Stroke::new(2.0, primary), egui::StrokeKind::Middle); }
                        if rr.clicked() { self.set.accent = i; save_settings(&self.set); }
                    }
                });
                cui.horizontal(|h| {
                    lab(h, self, "بکگراند");
                    for (i, (r, g, b)) in BGS.iter().enumerate() {
                        let (r2, rr) = h.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                        h.painter().rect_filled(r2, Rounding::same(4u8), Color32::from_rgb(*r, *g, *b));
                        h.painter().rect_stroke(r2, Rounding::same(4u8), Stroke::new(1.0, secondary), egui::StrokeKind::Middle);
                        if self.set.bg == i { h.painter().rect_stroke(r2, Rounding::same(4u8), Stroke::new(2.0, accent), egui::StrokeKind::Middle); }
                        if rr.clicked() { self.set.bg = i; self.set.dark = *r < 128; save_settings(&self.set); }
                    }
                });
                cui.horizontal(|h| {
                    lab(h, self, "شفافیت");
                    let mut o = self.set.opacity as f32;
                    if h.add(egui::Slider::new(&mut o, 60.0..=255.0).show_value(false)).changed() { self.set.opacity = o as u8; save_settings(&self.set); }
                });
                cui.horizontal(|h| {
                    lab(h, self, "گردی");
                    let mut r = self.set.radius as f32;
                    if h.add(egui::Slider::new(&mut r, 0.0..=40.0).show_value(false)).changed() { self.set.radius = r as u8; save_settings(&self.set); }
                });
                cui.horizontal(|h| {
                    lab(h, self, "اندازه");
                    for (i, name) in ["S", "M", "L"].iter().enumerate() {
                        let (t, tsz) = self.text(ctx, name, 12, true, if self.set.size == i { accent } else { secondary });
                        if h.add(egui::Button::image(egui::Image::new((t.id(), tsz))).frame(false)).clicked() { self.set.size = i; save_settings(&self.set); }
                    }
                });
                cui.horizontal(|h| {
                    let mut d = self.set.dark;
                    if h.add(egui::Checkbox::new(&mut d, "")).changed() { self.set.dark = d; self.set.bg = if d { 2 } else { 0 }; save_settings(&self.set); }
                    lab(h, self, "حالت تیره");
                });
                cui.horizontal(|h| {
                    let mut o = self.set.ontop;
                    if h.add(egui::Checkbox::new(&mut o, "")).changed() { self.set.ontop = o; save_settings(&self.set); }
                    lab(h, self, "همیشه روی صفحه");
                });
                cui.horizontal(|h| {
                    let mut a = self.set.autostart;
                    if h.add(egui::Checkbox::new(&mut a, "")).changed() { self.set.autostart = a; save_settings(&self.set); }
                    lab(h, self, "اجرا با ویندوز");
                });
                });
            }
        });

        // calendar window (separate viewport)
        if self.cal_open {
            let mut nav = 0i32;
            let mut close = false;
            let (cbg, cprim, csec) = self.colors();
            let cacc = accent;
            let cy = self.cal_y; let cm = self.cal_m;
            let (ty2, tm2, td2) = self.last_day;
            let mut texcache = std::mem::take(&mut self.tex);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("scw_cal"),
                egui::ViewportBuilder::default()
                    .with_title("تقویم شمسی")
                    .with_inner_size([300.0, 340.0])
                    .with_position(Pos2::new(self.set.px as f32 + sz + 24.0, self.set.py as f32))
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_resizable(false)
                    .with_taskbar(false)
                    .with_always_on_top(),
                |vctx, _class| {
                    if vctx.input(|i| i.viewport().close_requested()) { close = true; }
                    // local text helper using shared cache
                    let mut put = |ui: &mut egui::Ui, s: &str, px: i32, bold: bool, c: Color32, center: Pos2| -> Rect {
                        let ppp = vctx.pixels_per_point();
                        let key = (s.to_string(), (px as f32 * ppp).round() as i32, bold, u32::from_le_bytes(c.to_array()));
                        if !texcache.contains_key(&key) {
                            let (bytes, w, h) = gdi_text(s, (px as f32 * ppp).round() as i32, bold, c);
                            let img = egui::ColorImage::from_rgba_premultiplied([w, h], &bytes);
                            texcache.insert(key.clone(), vctx.load_texture(format!("ct{}", texcache.len()), img, egui::TextureOptions::LINEAR));
                        }
                        let t = texcache[&key].clone();
                        let sz2 = t.size_vec2() / ppp;
                        let r = snap_rect(center, sz2, ppp);
                        ui.put(r, egui::Image::new((t.id(), sz2)));
                        r
                    };
                    {
                        let ppp = vctx.pixels_per_point();
                        let wpx = (300.0 * ppp).round() as i32;
                        let hpx = (340.0 * ppp).round() as i32;
                        let rpx = ((self.set.radius as f32 * ppp).round() as i32).clamp(1, (wpx / 2).min(hpx / 2));
                        let key = ((rpx as i64) << 48) | ((hpx as i64) << 24) | (wpx as i64);
                        if CAL_RGN_KEY.swap(key, Ordering::Relaxed) != key {
                            let t: Vec<u16> = "تقویم شمسی".encode_utf16().chain(std::iter::once(0)).collect();
                            unsafe {
                                let hw = FindWindowW(std::ptr::null(), t.as_ptr());
                                if hw != 0 {
                                    let rgn = CreateRoundRectRgn(0, 0, wpx + 1, hpx + 1, rpx * 2, rpx * 2);
                                    SetWindowRgn(hw, rgn, 1);
                                }
                            }
                        }
                    }
                    egui::CentralPanel::default().frame(egui::Frame::NONE).show(vctx, |ui| {
                        ui.set_min_size(Vec2::new(300.0, 340.0));
                        let (root, _) = ui.allocate_exact_size(Vec2::new(300.0, 340.0), Sense::hover());
                        let cbf = cbg;
                        ui.painter().rect_filled(root, Rounding::same(self.set.radius), cbf);
                        // header
                        let dresp = ui.interact(Rect::from_min_size(root.min, Vec2::new(300.0, 340.0)), egui::Id::new("scw_cal_dragbody"), Sense::click_and_drag());
                        if dresp.drag_started() { vctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
                        let close_r = Rect::from_min_size(Pos2::new(root.right() - 24.0, root.top() + 8.0), Vec2::splat(16.0));
                        paint_close(ui.painter(), close_r, csec);
                        if ui.interact(close_r, egui::Id::new("cal_close"), Sense::click()).clicked() { close = true; }
                        let prev_r = Rect::from_min_size(Pos2::new(root.left() + 10.0, root.top() + 8.0), Vec2::splat(16.0));
                        let next_r = Rect::from_min_size(Pos2::new(root.left() + 32.0, root.top() + 8.0), Vec2::splat(16.0));
                        paint_arrow(ui.painter(), prev_r, csec, true);
                        paint_arrow(ui.painter(), next_r, csec, false);
                        if ui.interact(prev_r, egui::Id::new("cal_prev"), Sense::click()).clicked() { nav = -1; }
                        if ui.interact(next_r, egui::Id::new("cal_next"), Sense::click()).clicked() { nav = 1; }
                        put(ui, &format!("{} {}", MONTHS[(cm - 1) as usize], fa(cy)), 17, true, cprim, Pos2::new(root.center().x, root.top() + 16.0));
                        put(ui, "تقویم شمسی", 12, false, csec, Pos2::new(root.center().x, root.top() + 36.0));
                        // grid
                        let gx = root.left() + 10.0;
                        let gy0 = root.top() + 48.0;
                        let cw = (300.0 - 20.0) / 7.0;
                        let ch = 40.0;
                        for c in 0..7 {
                            put(ui, WEEK_SHORT[c], 13, true, csec, Pos2::new(gx + cw * (6.0 - c as f32 + 0.5), gy0 + 6.0));
                        }
                        let (gy, gm, gd) = j2g(cy, cm, 1);
                        let off = weekday_idx(gy, gm, gd);
                        let len = j_month_len(cy, cm);
                        for cell in 0..42 {
                            let dnum = cell as i32 - off as i32 + 1;
                            if dnum < 1 || dnum > len { continue; }
                            let row = cell / 7; let col = 6 - (cell % 7);
                            let cr = Rect::from_min_size(Pos2::new(gx + cw * col as f32 + 3.0, gy0 + 22.0 + ch * row as f32 + 2.0), Vec2::new(cw - 6.0, ch - 6.0));
                            let is_today = cy == ty2 && cm == tm2 && dnum == td2;
                            if is_today { ui.painter().rect_filled(cr, Rounding::same(6u8), cacc); }
                            let tc = if is_today { Color32::WHITE } else { cprim };
                            put(ui, &fa(dnum), 16, is_today, tc, cr.center());
                            if ui.interact(cr, egui::Id::new(("calcell", cell)), Sense::click()).clicked() { /* selection noop */ }
                        }
                    });
                    vctx.request_repaint_after(Duration::from_millis(500));
                },
            );
            self.tex = texcache;
            if close { self.cal_open = false; CAL_RGN_KEY.store(0, Ordering::Relaxed); }
            if nav != 0 {
                let mut m = self.cal_m + nav;
                let mut y = self.cal_y;
                if m < 1 { m = 12; y -= 1; }
                if m > 12 { m = 1; y += 1; }
                self.cal_m = m; self.cal_y = y;
            }
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

fn main() -> eframe::Result {
    {
        #[link(name = "kernel32")]
        extern "system" { fn CreateMutexW(a: *mut std::ffi::c_void, b: i32, n: *const u16) -> isize; fn GetLastError() -> u32; }
        let nm: Vec<u16> = "Local\\ShamsiWidgetRustSingle".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let h = CreateMutexW(std::ptr::null_mut(), 0, nm.as_ptr());
            if h == 0 || GetLastError() == 183 { return Ok(()); }
        }
    }
    let vp = egui::ViewportBuilder::default()
        .with_title("Shamsi Widget")
        .with_inner_size([190.0, 190.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(false)
        .with_visible(true)
        .with_taskbar(false);
    let opts = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native("shamsi_widget", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
