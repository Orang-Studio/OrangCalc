// function and data plotting with pan, zoom, styles and PNG export
use crate::engine::*;
use crate::ui::app::{App, Page};
use crate::ui::widgets::*;
use gtk::prelude::*;
use gtk::{cairo, gdk, glib};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, PartialEq, Default)]
enum Source {
    #[default]
    Function,
    Vector,
    Paired,
}

#[derive(Clone, Default)]
struct Series {
    title: String,
    expression: String,
    variable: String,
    source: Source,
    style: usize,
    smooth: usize,
    secondary: bool,
    imag_parts: bool,
    points: Vec<(f64, f64)>,
    imag: Vec<(f64, f64)>,
    error: Option<String>,
    color: (f64, f64, f64),
}

impl Series {
    fn name(&self) -> String {
        format!("{}{}", if self.title.is_empty() { &self.expression } else { &self.title }, if self.error.is_some() { "  (error)" } else { "" })
    }
}

const PALETTE_REST: [(u8, u8, u8); 7] = [(96, 160, 232), (110, 196, 130), (196, 140, 230), (232, 206, 90), (232, 100, 100), (80, 200, 200), (200, 200, 200)];

fn palette_color(i: usize) -> (u8, u8, u8) {
    if i == 0 {
        crate::ui::theme::accent_rgb()
    } else {
        PALETTE_REST[(i - 1) % PALETTE_REST.len()]
    }
}

#[derive(Default)]
struct View {
    title: String,
    xlabel: String,
    ylabel: String,
    grid: bool,
    border: bool,
    log_x: bool,
    log_y: bool,
    polar: bool,
    log_base: f64,
    legend: usize,
    line_width: f64,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    vx0: f64,
    vx1: f64,
    vy0: f64,
    vy1: f64,
    sy0: f64,
    sy1: f64,
    mouse: Option<(f64, f64)>,
}

struct Canvas {
    series: RefCell<Vec<Series>>,
    v: RefCell<View>,
}

fn nice_step(span: f64, target: f64) -> f64 {
    let raw = span / target.max(1.0);
    let mag = 10f64.powf(raw.log10().floor());
    let n = raw / mag;
    (if n < 1.5 {
        1.0
    } else if n < 3.0 {
        2.0
    } else if n < 7.0 {
        5.0
    } else {
        10.0
    }) * mag
}

fn fmt(v: f64) -> String {
    if v.abs() < 1e-12 {
        return "0".into();
    }
    let a = v.abs();
    if !(1e-3..1e6).contains(&a) {
        let e = a.log10().floor() as i32;
        let m = v / 10f64.powi(e);
        return format!("{}E{}", trim(format!("{:.2}", m)), e);
    }
    trim(format!("{:.4}", v))
}

fn trim(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

impl Canvas {
    fn tx(&self, v: &View, x: f64) -> f64 {
        if v.log_x {
            x.ln() / v.log_base.ln()
        } else {
            x
        }
    }
    fn ty(&self, v: &View, y: f64) -> f64 {
        if v.log_y {
            y.ln() / v.log_base.ln()
        } else {
            y
        }
    }
    fn ux(&self, v: &View, t: f64) -> f64 {
        if v.log_x {
            v.log_base.powf(t)
        } else {
            t
        }
    }
    fn uy(&self, v: &View, t: f64) -> f64 {
        if v.log_y {
            v.log_base.powf(t)
        } else {
            t
        }
    }
    fn ok(v: &View, p: (f64, f64)) -> bool {
        p.0.is_finite() && p.1.is_finite() && (!v.log_y || p.1 > 0.0) && (!v.log_x || p.0 > 0.0)
    }

    fn fit_view(&self) {
        let series = self.series.borrow();
        let mut v = self.v.borrow_mut();
        let pts: Vec<(f64, f64)> = series.iter().filter(|s| !s.secondary).flat_map(|s| s.points.iter().chain(if s.imag_parts { s.imag.iter() } else { [].iter() }).cloned()).filter(|p| Self::ok(&v, *p)).collect();
        let spts: Vec<(f64, f64)> = series.iter().filter(|s| s.secondary).flat_map(|s| s.points.iter().cloned()).filter(|p| Self::ok(&v, *p)).collect();
        if v.polar || series.iter().any(|s| s.source != Source::Function) {
            let all: Vec<(f64, f64)> = pts.iter().chain(spts.iter()).cloned().collect();
            if !all.is_empty() {
                v.vx0 = all.iter().map(|p| self.tx(&v, p.0)).fold(f64::MAX, f64::min);
                v.vx1 = all.iter().map(|p| self.tx(&v, p.0)).fold(f64::MIN, f64::max);
            } else {
                v.vx0 = self.tx(&v, v.x_min);
                v.vx1 = self.tx(&v, v.x_max);
            }
        } else {
            let lo = if v.log_x { v.x_min.max(1e-12) } else { v.x_min };
            v.vx0 = self.tx(&v, lo);
            v.vx1 = self.tx(&v, v.x_max);
        }
        let (a, b) = self.y_bounds(&v, &pts, true);
        v.vy0 = a;
        v.vy1 = b;
        let (a, b) = self.y_bounds(&v, &spts, false);
        v.sy0 = a;
        v.sy1 = b;
        if v.polar && !pts.is_empty() {
            let m = v.vx0.abs().max(v.vx1.abs()).max(v.vy0.abs().max(v.vy1.abs()));
            v.vx0 = -m;
            v.vy0 = -m;
            v.vx1 = m;
            v.vy1 = m;
        }
        if v.vx1 - v.vx0 < 1e-300 {
            v.vx0 -= 1.0;
            v.vx1 += 1.0;
        }
    }

    fn y_bounds(&self, v: &View, pts: &[(f64, f64)], primary: bool) -> (f64, f64) {
        let (mut lo, mut hi) = if pts.is_empty() {
            (-1.0, 1.0)
        } else {
            let mut ys: Vec<f64> = pts.iter().map(|p| self.ty(v, p.1)).collect();
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = ys.len();
            let (mut lo, mut hi) = (ys[0], ys[n - 1]);
            if n > 20 {
                let (q1, q3) = (ys[(n as f64 * 0.02) as usize], ys[(n as f64 * 0.98) as usize]);
                let span = (q3 - q1).max(1e-9);
                lo = lo.max(q1 - span * 0.5);
                hi = hi.min(q3 + span * 0.5);
            }
            let mut pad = (hi - lo) * 0.06;
            if pad < 1e-12 {
                pad = hi.abs().max(1.0) * 0.1;
            }
            (lo - pad, hi + pad)
        };
        if primary {
            if v.y_min.is_finite() {
                lo = self.ty(v, v.y_min);
            }
            if v.y_max.is_finite() {
                hi = self.ty(v, v.y_max);
            }
        }
        (lo, hi)
    }

    fn area(&self, w: f64, h: f64) -> (f64, f64, f64, f64) {
        let v = self.v.borrow();
        let secondary = self.series.borrow().iter().any(|s| s.secondary);
        let mut left = 64.0;
        let right = if secondary { 64.0 } else { 22.0 };
        let top = if v.title.is_empty() { 18.0 } else { 38.0 };
        let mut bottom = 42.0 + if v.legend == 5 { 24.0 } else { 0.0 };
        if !v.ylabel.is_empty() {
            left += 16.0;
        }
        if !v.xlabel.is_empty() {
            bottom += 16.0;
        }
        (left, top, (w - left - right).max(10.0), (h - top - bottom).max(10.0))
    }

    fn render(&self, cr: &cairo::Context, w: f64, h: f64, interactive: bool, fg: (f64, f64, f64), bg: Option<(f64, f64, f64)>) {
        if let Some(b) = bg {
            cr.set_source_rgb(b.0, b.1, b.2);
            let _ = cr.paint();
        }
        if self.v.borrow().vx1 <= self.v.borrow().vx0 {
            self.fit_view();
        }
        let (al, at, aw, ah) = self.area(w, h);
        let (ar, ab) = (al + aw, at + ah);
        let v = self.v.borrow();
        let set = |a: f64| cr.set_source_rgba(fg.0, fg.1, fg.2, a);
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        if !v.title.is_empty() {
            cr.set_font_size(15.0);
            set(1.0);
            let tw = cr.text_extents(&v.title).map(|e| e.width()).unwrap_or(0.0);
            cr.move_to((w - tw) / 2.0, 24.0);
            let _ = cr.show_text(&v.title);
        }
        let dx = v.vx1 - v.vx0;
        let dy = if v.vy1 - v.vy0 <= 0.0 { 1.0 } else { v.vy1 - v.vy0 };
        let dsy = if v.sy1 - v.sy0 <= 0.0 { 1.0 } else { v.sy1 - v.sy0 };
        let x_of = |t: f64| al + (t - v.vx0) / dx * aw;
        let y_of = |t: f64| ab - (t - v.vy0) / dy * ah;
        let sy_of = |t: f64| ab - (t - v.sy0) / dsy * ah;
        cr.set_font_size(10.0);
        cr.set_line_width(1.0);
        let xs = nice_step(dx, (aw / 90.0).max(2.0));
        let mut t = (v.vx0 / xs).ceil() * xs;
        while t <= v.vx1 + xs * 1e-9 {
            let px = x_of(t);
            if v.grid {
                set(0.12);
                cr.move_to(px, at);
                cr.line_to(px, ab);
                let _ = cr.stroke();
            }
            set(0.65);
            let s = fmt(if v.log_x { self.ux(&v, t) } else { t });
            let tw = cr.text_extents(&s).map(|e| e.width()).unwrap_or(0.0);
            cr.move_to(px - tw / 2.0, ab + 15.0);
            let _ = cr.show_text(&s);
            t += xs;
        }
        let ys = nice_step(dy, (ah / 60.0).max(2.0));
        let mut t = (v.vy0 / ys).ceil() * ys;
        while t <= v.vy1 + ys * 1e-9 {
            let py = y_of(t);
            if v.grid {
                set(0.12);
                cr.move_to(al, py);
                cr.line_to(ar, py);
                let _ = cr.stroke();
            }
            set(0.65);
            let s = fmt(if v.log_y { self.uy(&v, t) } else { t });
            let tw = cr.text_extents(&s).map(|e| e.width()).unwrap_or(0.0);
            cr.move_to(al - tw - 6.0, py + 4.0);
            let _ = cr.show_text(&s);
            t += ys;
        }
        let series = self.series.borrow();
        if series.iter().any(|s| s.secondary) {
            let ss = nice_step(dsy, (ah / 60.0).max(2.0));
            let mut t = (v.sy0 / ss).ceil() * ss;
            while t <= v.sy1 + ss * 1e-9 {
                set(0.65);
                cr.move_to(ar + 4.0, sy_of(t) + 4.0);
                let _ = cr.show_text(&fmt(t));
                t += ss;
            }
        }
        set(0.4);
        if v.vx0 < 0.0 && v.vx1 > 0.0 && !v.log_x {
            cr.move_to(x_of(0.0), at);
            cr.line_to(x_of(0.0), ab);
            let _ = cr.stroke();
        }
        if v.vy0 < 0.0 && v.vy1 > 0.0 && !v.log_y {
            cr.move_to(al, y_of(0.0));
            cr.line_to(ar, y_of(0.0));
            let _ = cr.stroke();
        }
        if v.border {
            cr.rectangle(al, at, aw, ah);
        } else {
            cr.move_to(al, at);
            cr.line_to(al, ab);
            cr.line_to(ar, ab);
        }
        let _ = cr.stroke();
        set(0.65);
        cr.set_font_size(12.0);
        if !v.xlabel.is_empty() {
            let tw = cr.text_extents(&v.xlabel).map(|e| e.width()).unwrap_or(0.0);
            cr.move_to(al + (aw - tw) / 2.0, ab + 34.0);
            let _ = cr.show_text(&v.xlabel);
        }
        if !v.ylabel.is_empty() {
            let tw = cr.text_extents(&v.ylabel).map(|e| e.width()).unwrap_or(0.0);
            let _ = cr.save();
            cr.translate(16.0, at + ah / 2.0 + tw / 2.0);
            cr.rotate(-std::f64::consts::FRAC_PI_2);
            cr.move_to(0.0, 0.0);
            let _ = cr.show_text(&v.ylabel);
            let _ = cr.restore();
        }
        let _ = cr.save();
        cr.rectangle(al, at, aw, ah);
        cr.clip();
        for s in series.iter() {
            let yf: &dyn Fn(f64) -> f64 = if s.secondary { &sy_of } else { &y_of };
            self.draw_series(cr, &v, &s.points, s, &x_of, yf, s.color, (at, ab), false);
            if s.imag_parts && !s.imag.is_empty() {
                let c = (s.color.0 + (1.0 - s.color.0) * 0.3, s.color.1 + (1.0 - s.color.1) * 0.3, s.color.2 + (1.0 - s.color.2) * 0.3);
                self.draw_series(cr, &v, &s.imag, s, &x_of, yf, c, (at, ab), true);
            }
        }
        let _ = cr.restore();
        if v.legend != 0 && !series.is_empty() {
            self.draw_legend(cr, &series, v.legend, (al, at, aw, ah), fg, bg);
        }
        if interactive {
            if let Some((mx, my)) = v.mouse {
                if mx >= al && mx <= ar && my >= at && my <= ab {
                    let x = self.ux(&v, v.vx0 + (mx - al) / aw * dx);
                    let y = self.uy(&v, v.vy0 + (ab - my) / ah * dy);
                    set(0.55);
                    cr.set_font_size(10.0);
                    cr.move_to(al + 6.0, at + 14.0);
                    let _ = cr.show_text(&format!("x = {}   y = {}", fmt(x), fmt(y)));
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_series(&self, cr: &cairo::Context, v: &View, raw: &[(f64, f64)], s: &Series, x_of: &dyn Fn(f64) -> f64, y_of: &dyn Fn(f64) -> f64, c: (f64, f64, f64), (at, ab): (f64, f64), dashed: bool) {
        let pts: Vec<(f64, f64)> = raw.iter().filter(|p| Self::ok(v, **p)).map(|p| (x_of(self.tx(v, p.0)), y_of(self.ty(v, p.1)))).collect();
        if pts.is_empty() {
            return;
        }
        cr.set_source_rgb(c.0, c.1, c.2);
        cr.set_line_width(v.line_width);
        cr.set_line_join(cairo::LineJoin::Round);
        cr.set_dash(if dashed { &[6.0, 4.0] } else { &[] }, 0.0);
        match s.style {
            1 | 2 => {
                if s.style == 2 {
                    self.draw_line(cr, v, raw, x_of, y_of, s.smooth);
                }
                let r = (v.line_width + 1.5).max(2.5);
                for p in &pts {
                    cr.arc(p.0, p.1, r, 0.0, std::f64::consts::TAU);
                    let _ = cr.fill();
                }
            }
            3 => {
                for p in &pts {
                    cr.rectangle(p.0 - 1.0, p.1 - 1.0, 2.0, 2.0);
                    let _ = cr.fill();
                }
            }
            4 | 5 => {
                let bw = if pts.len() > 1 { (pts[1].0 - pts[0].0).abs() * if s.style == 4 { 0.7 } else { 1.0 } } else { 20.0 };
                let zero = y_of(if v.log_y { v.vy0 } else { 0.0 }).clamp(at, ab);
                cr.set_line_width(1.0);
                for p in &pts {
                    cr.rectangle(p.0 - bw / 2.0, p.1.min(zero), bw, (zero - p.1).abs());
                    cr.set_source_rgba(c.0, c.1, c.2, 0.35);
                    let _ = cr.fill_preserve();
                    cr.set_source_rgb(c.0, c.1, c.2);
                    let _ = cr.stroke();
                }
            }
            6 => {
                cr.move_to(pts[0].0, pts[0].1);
                for i in 1..pts.len() {
                    cr.line_to(pts[i].0, pts[i - 1].1);
                    cr.line_to(pts[i].0, pts[i].1);
                }
                let _ = cr.stroke();
            }
            _ => self.draw_line(cr, v, raw, x_of, y_of, s.smooth),
        }
        cr.set_dash(&[], 0.0);
    }

    fn draw_line(&self, cr: &cairo::Context, v: &View, raw: &[(f64, f64)], x_of: &dyn Fn(f64) -> f64, y_of: &dyn Fn(f64) -> f64, smooth: usize) {
        let mut seg: Vec<(f64, f64)> = vec![];
        let mut last_y = f64::NAN;
        let flush = |seg: &mut Vec<(f64, f64)>| {
            if seg.len() > 1 {
                cr.move_to(seg[0].0, seg[0].1);
                if smooth == 0 || seg.len() < 3 {
                    for p in &seg[1..] {
                        cr.line_to(p.0, p.1);
                    }
                } else {
                    // cardinal spline through the points
                    let tension = match smooth {
                        1 => 0.25,
                        3 => 0.5,
                        _ => 0.6,
                    } / 3.0;
                    for i in 0..seg.len() - 1 {
                        let p0 = seg[i.saturating_sub(1)];
                        let (p1, p2) = (seg[i], seg[i + 1]);
                        let p3 = seg[(i + 2).min(seg.len() - 1)];
                        cr.curve_to(p1.0 + (p2.0 - p0.0) * tension, p1.1 + (p2.1 - p0.1) * tension, p2.0 - (p3.0 - p1.0) * tension, p2.1 - (p3.1 - p1.1) * tension, p2.0, p2.1);
                    }
                }
                let _ = cr.stroke();
            }
            seg.clear();
        };
        for p in raw {
            if !Self::ok(v, *p) {
                flush(&mut seg);
                last_y = f64::NAN;
                continue;
            }
            let ty = self.ty(v, p.1);
            if !last_y.is_nan() && (ty - last_y).abs() > (v.vy1 - v.vy0) * 3.0 {
                flush(&mut seg);
            }
            let sp = (x_of(self.tx(v, p.0)), y_of(ty));
            if sp.1.abs() > 1e6 {
                flush(&mut seg);
                last_y = f64::NAN;
                continue;
            }
            seg.push(sp);
            last_y = ty;
        }
        flush(&mut seg);
    }

    fn draw_legend(&self, cr: &cairo::Context, series: &[Series], place: usize, (al, at, aw, ah): (f64, f64, f64, f64), fg: (f64, f64, f64), bg: Option<(f64, f64, f64)>) {
        cr.set_font_size(12.0);
        let items: Vec<String> = series.iter().map(|s| if s.title.is_empty() { s.expression.clone() } else { s.title.clone() }).collect();
        if place == 5 {
            let mut x = al;
            for (i, it) in items.iter().enumerate() {
                let c = series[i].color;
                cr.set_source_rgb(c.0, c.1, c.2);
                cr.set_line_width(3.0);
                cr.move_to(x, at + ah + 58.0);
                cr.line_to(x + 18.0, at + ah + 58.0);
                let _ = cr.stroke();
                cr.set_source_rgb(fg.0, fg.1, fg.2);
                cr.move_to(x + 22.0, at + ah + 62.0);
                let _ = cr.show_text(it);
                x += cr.text_extents(it).map(|e| e.width()).unwrap_or(0.0) + 40.0;
            }
            return;
        }
        let lw = items.iter().map(|t| cr.text_extents(t).map(|e| e.width()).unwrap_or(0.0)).fold(0.0, f64::max) + 40.0;
        let lh = items.len() as f64 * 20.0 + 8.0;
        let (bx, by) = match place {
            1 => (al + 8.0, at + 8.0),
            3 => (al + 8.0, at + ah - lh - 8.0),
            4 => (al + aw - lw - 8.0, at + ah - lh - 8.0),
            _ => (al + aw - lw - 8.0, at + 8.0),
        };
        let b = bg.unwrap_or((0.15, 0.16, 0.19));
        cr.set_source_rgba(b.0, b.1, b.2, 0.92);
        cr.rectangle(bx, by, lw, lh);
        let _ = cr.fill_preserve();
        cr.set_source_rgba(fg.0, fg.1, fg.2, 0.25);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        for (i, it) in items.iter().enumerate() {
            let y = by + 4.0 + i as f64 * 20.0;
            let c = series[i].color;
            cr.set_source_rgb(c.0, c.1, c.2);
            cr.set_line_width(3.0);
            cr.move_to(bx + 8.0, y + 10.0);
            cr.line_to(bx + 26.0, y + 10.0);
            let _ = cr.stroke();
            cr.set_source_rgb(fg.0, fg.1, fg.2);
            cr.move_to(bx + 32.0, y + 14.0);
            let _ = cr.show_text(it);
        }
    }
}

pub struct PlotPage {
    app: Weak<App>,
    root: gtk::Box,
    area: gtk::DrawingArea,
    canvas: Rc<Canvas>,
    series_list: gtk::ListBox,
    expr: gtk::Entry,
    title: gtk::Entry,
    source: gtk::DropDown,
    variable: gtk::Entry,
    style: gtk::DropDown,
    smooth: gtk::DropDown,
    axis: gtk::DropDown,
    imag: gtk::CheckButton,
    xmin: gtk::Entry,
    xmax: gtk::Entry,
    samples: gtk::Entry,
    use_step: gtk::CheckButton,
    polar: gtk::CheckButton,
    ptitle: gtk::Entry,
    xlabel: gtk::Entry,
    ylabel: gtk::Entry,
    grid: gtk::CheckButton,
    border: gtk::CheckButton,
    ymin: gtk::Entry,
    ymax: gtk::Entry,
    logx: gtk::CheckButton,
    logy: gtk::CheckButton,
    log_base: gtk::Entry,
    legend: gtk::DropDown,
    width: gtk::SpinButton,
    color_index: Cell<usize>,
    drag: Cell<Option<(f64, f64)>>,
}

impl Page for PlotPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
}

thread_local! {
    static CURRENT: RefCell<Weak<PlotPage>> = RefCell::new(Weak::new());
}

fn parse_d(s: &str, def: f64) -> f64 {
    let s = s.trim().replace('−', "-");
    if s.is_empty() {
        return def;
    }
    s.parse().unwrap_or(def)
}

fn check(t: &str, on: bool) -> gtk::CheckButton {
    let c = gtk::CheckButton::with_label(t);
    c.set_active(on);
    c
}

impl PlotPage {
    pub fn from_page(_p: &Rc<dyn Page>) -> Option<Rc<PlotPage>> {
        CURRENT.with(|c| c.borrow().upgrade())
    }

    pub fn new(app: &Rc<App>) -> Rc<PlotPage> {
        let root = vbox(10);
        margins(&root, 16);
        let header = label("Plot");
        header.add_css_class("oc-page-title");
        root.append(&header);
        let body = gtk::Paned::new(gtk::Orientation::Horizontal);
        body.set_vexpand(true);
        let form_box = vbox(4);
        margins(&form_box, 6);
        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        area.set_focusable(true);
        let frame = gtk::Frame::new(None);
        frame.set_child(Some(&area));
        let side = scrolled(&form_box);
        side.set_size_request(320, -1);
        side.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        body.set_start_child(Some(&side));
        body.set_end_child(Some(&frame));
        body.set_shrink_start_child(false);
        body.set_resize_start_child(false);
        root.append(&body);

        let canvas = Rc::new(Canvas { series: RefCell::new(vec![]), v: RefCell::new(View { grid: true, border: true, log_base: 10.0, legend: 2, line_width: 2.0, x_min: -10.0, x_max: 10.0, y_min: f64::NAN, y_max: f64::NAN, ..Default::default() }) });
        let page = Rc::new(PlotPage {
            app: Rc::downgrade(app),
            root,
            area,
            canvas,
            series_list: gtk::ListBox::new(),
            expr: entry("sin(x)"),
            title: entry(""),
            source: combo(&["Function", "Vector/matrix", "Paired matrix"], 0),
            variable: entry("x"),
            style: combo(&["Line", "Points", "Line with points", "Dots", "Boxes/bars", "Histogram", "Steps"], 0),
            smooth: combo(&["None", "Monotonic", "Natural cubic splines", "Bezier"], 0),
            axis: combo(&["Primary", "Secondary"], 0),
            imag: check("Show real and imaginary parts", false),
            xmin: entry("-10"),
            xmax: entry("10"),
            samples: entry("1001"),
            use_step: check("Use the sampling value as step size", false),
            polar: check("Polar plot (expression is r(θ), x variable is θ)", false),
            ptitle: entry(""),
            xlabel: entry(""),
            ylabel: entry(""),
            grid: check("Display grid", true),
            border: check("Display full border", true),
            ymin: entry(""),
            ymax: entry(""),
            logx: check("Logarithmic x scale", false),
            logy: check("Logarithmic y scale", false),
            log_base: entry("10"),
            legend: combo(&["None", "Top-left", "Top-right", "Bottom-left", "Bottom-right", "Below"], 2),
            width: gtk::SpinButton::with_range(1.0, 10.0, 1.0),
            color_index: Cell::new(0),
            drag: Cell::new(None),
        });
        CURRENT.with(|c| *c.borrow_mut() = Rc::downgrade(&page));
        page.width.set_value(2.0);
        page.expr.add_css_class("oc-mono");
        let p = &page;
        let section = |t: &str| {
            let l = heading(t);
            l.set_margin_top(10);
            form_box.append(&l);
        };
        let mut f = Form::new();
        section("Data");
        f.add("Expression:", &p.expr);
        f.add("Title:", &p.title);
        f.add("Type:", &p.source);
        f.add("X variable:", &p.variable);
        f.add("Style:", &p.style);
        f.add("Smoothing:", &p.smooth);
        f.add("Y-axis:", &p.axis);
        f.full(&p.imag);
        form_box.append(&f.grid);
        let brow = hbox(6);
        let w = Rc::downgrade(p);
        let mkb = |t: &str, primary: bool, fnc: fn(&Rc<PlotPage>)| {
            let w = w.clone();
            let b = button(t, move || {
                if let Some(p) = w.upgrade() {
                    fnc(&p);
                }
            });
            if primary {
                b.add_css_class("suggested-action");
            }
            b
        };
        brow.append(&mkb("Add", true, |p| p.add_from_form()));
        brow.append(&mkb("Apply", false, |p| p.apply_to_selected()));
        brow.append(&mkb("Remove", false, |p| p.remove_selected()));
        form_box.append(&brow);
        let ls = scrolled(&p.series_list);
        ls.set_min_content_height(96);
        ls.set_vexpand(false);
        form_box.append(&ls);
        section("Function Range");
        let mut f2 = Form::new();
        f2.add("Minimum x value:", &p.xmin);
        f2.add("Maximum x value:", &p.xmax);
        f2.add("Sampling rate:", &p.samples);
        f2.full(&p.use_step);
        f2.full(&p.polar);
        form_box.append(&f2.grid);
        section("Appearance");
        let mut f3 = Form::new();
        f3.add("Title:", &p.ptitle);
        f3.add("X-axis label:", &p.xlabel);
        f3.add("Y-axis label:", &p.ylabel);
        f3.full(&p.grid);
        f3.full(&p.border);
        f3.add("Minimum y value:", &p.ymin);
        f3.add("Maximum y value:", &p.ymax);
        f3.full(&p.logx);
        f3.full(&p.logy);
        f3.add("Log base:", &p.log_base);
        f3.add("Legend placement:", &p.legend);
        f3.add("Line width:", &p.width);
        form_box.append(&f3.grid);
        let actions = gtk::FlowBox::new();
        actions.set_selection_mode(gtk::SelectionMode::None);
        actions.insert(&mkb("Plot", true, |p| p.replot()), -1);
        actions.insert(&mkb("Reset view", false, |p| {
            p.canvas.fit_view();
            p.area.queue_draw();
        }), -1);
        actions.insert(&mkb("Save PNG…", false, |p| p.save_png()), -1);
        actions.insert(&mkb("Copy image", false, |p| p.copy_image()), -1);
        form_box.append(&actions);

        let w = Rc::downgrade(p);
        p.area.set_draw_func(move |a, cr, wd, ht| {
            if let Some(p) = w.upgrade() {
                let c = a.color();
                p.canvas.render(cr, wd as f64, ht as f64, true, (c.red() as f64, c.green() as f64, c.blue() as f64), None);
            }
        });
        for cb in [&p.grid, &p.border, &p.logx, &p.logy, &p.polar] {
            let w = Rc::downgrade(p);
            let refit = cb == &p.polar || cb == &p.logx || cb == &p.logy;
            cb.connect_toggled(move |_| {
                if let Some(p) = w.upgrade() {
                    p.apply_appearance(refit);
                }
            });
        }
        let w = Rc::downgrade(p);
        p.legend.connect_selected_notify(move |_| {
            if let Some(p) = w.upgrade() {
                p.apply_appearance(false);
            }
        });
        let w = Rc::downgrade(p);
        p.width.connect_value_changed(move |_| {
            if let Some(p) = w.upgrade() {
                p.apply_appearance(false);
            }
        });
        let w = Rc::downgrade(p);
        p.expr.connect_activate(move |_| {
            if let Some(p) = w.upgrade() {
                if p.series_list.selected_row().is_some() {
                    p.apply_to_selected();
                } else {
                    p.add_from_form();
                }
            }
        });
        let w = Rc::downgrade(p);
        p.series_list.connect_row_selected(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.load_selected();
            }
        });
        // pan with the mouse, zoom with the wheel
        let drag = gtk::GestureDrag::new();
        let w = Rc::downgrade(p);
        drag.connect_drag_begin(move |_, _, _| {
            if let Some(p) = w.upgrade() {
                p.drag.set(Some((0.0, 0.0)));
                p.area.grab_focus();
            }
        });
        let w = Rc::downgrade(p);
        drag.connect_drag_update(move |_, ox, oy| {
            let Some(p) = w.upgrade() else { return };
            let Some((lx, ly)) = p.drag.get() else { return };
            let (ddx, ddy) = (ox - lx, oy - ly);
            p.drag.set(Some((ox, oy)));
            let (_, _, aw, ah) = p.canvas.area(p.area.width() as f64, p.area.height() as f64);
            let mut v = p.canvas.v.borrow_mut();
            let mx = ddx / aw * (v.vx1 - v.vx0);
            let my = ddy / ah * (v.vy1 - v.vy0);
            let ms = ddy / ah * (v.sy1 - v.sy0);
            v.vx0 -= mx;
            v.vx1 -= mx;
            v.vy0 += my;
            v.vy1 += my;
            v.sy0 += ms;
            v.sy1 += ms;
            drop(v);
            p.area.queue_draw();
        });
        let w = Rc::downgrade(p);
        drag.connect_drag_end(move |_, _, _| {
            if let Some(p) = w.upgrade() {
                p.drag.set(None);
                p.view_changed();
            }
        });
        p.area.add_controller(drag);
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(p);
        motion.connect_motion(move |_, x, y| {
            if let Some(p) = w.upgrade() {
                p.canvas.v.borrow_mut().mouse = Some((x, y));
                p.area.queue_draw();
            }
        });
        let w = Rc::downgrade(p);
        motion.connect_leave(move |_| {
            if let Some(p) = w.upgrade() {
                p.canvas.v.borrow_mut().mouse = None;
                p.area.queue_draw();
            }
        });
        p.area.add_controller(motion);
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let w = Rc::downgrade(p);
        scroll.connect_scroll(move |_, _, dy| {
            let Some(p) = w.upgrade() else { return glib::Propagation::Proceed };
            let (mx, my) = p.canvas.v.borrow().mouse.unwrap_or((p.area.width() as f64 / 2.0, p.area.height() as f64 / 2.0));
            let (al, at, aw, ah) = p.canvas.area(p.area.width() as f64, p.area.height() as f64);
            let f = if dy < 0.0 { 0.8 } else { 1.25 };
            let mut v = p.canvas.v.borrow_mut();
            let cx = v.vx0 + (mx - al) / aw * (v.vx1 - v.vx0);
            let cy = v.vy0 + (at + ah - my) / ah * (v.vy1 - v.vy0);
            v.vx0 = cx + (v.vx0 - cx) * f;
            v.vx1 = cx + (v.vx1 - cx) * f;
            v.vy0 = cy + (v.vy0 - cy) * f;
            v.vy1 = cy + (v.vy1 - cy) * f;
            let csy = (v.sy0 + v.sy1) / 2.0;
            v.sy0 = csy + (v.sy0 - csy) * f;
            v.sy1 = csy + (v.sy1 - csy) * f;
            drop(v);
            p.area.queue_draw();
            p.view_changed();
            glib::Propagation::Stop
        });
        p.area.add_controller(scroll);
        page
    }

    fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }

    fn view_changed(self: &Rc<Self>) {
        let (polar, func) = (self.canvas.v.borrow().polar, self.canvas.series.borrow().iter().any(|s| s.source == Source::Function));
        if polar || !func {
            return;
        }
        let v = self.canvas.v.borrow();
        let (a, b) = (self.canvas.ux(&v, v.vx0), self.canvas.ux(&v, v.vx1));
        drop(v);
        self.xmin.set_text(&format!("{:.6}", a));
        self.xmax.set_text(&format!("{:.6}", b));
        self.recompute(false);
    }

    fn from_form(&self, s: &mut Series) {
        s.expression = self.expr.text().trim().to_string();
        s.title = self.title.text().trim().to_string();
        s.source = [Source::Function, Source::Vector, Source::Paired][self.source.selected() as usize % 3];
        let v = self.variable.text().trim().to_string();
        s.variable = if v.is_empty() { "x".into() } else { v };
        s.style = self.style.selected() as usize;
        s.smooth = self.smooth.selected() as usize;
        s.secondary = self.axis.selected() == 1;
        s.imag_parts = self.imag.is_active();
    }

    fn add_from_form(self: &Rc<Self>) {
        if self.expr.text().trim().is_empty() {
            return;
        }
        let i = self.color_index.get();
        self.color_index.set(i + 1);
        let j = i % 8;
        let c = palette_color(j);
        let mut s = Series { color: (c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0), ..Default::default() };
        self.from_form(&mut s);
        self.canvas.series.borrow_mut().push(s);
        self.refresh_list();
        self.series_list.unselect_all();
        self.recompute(true);
    }

    pub fn add_series(self: &Rc<Self>, e: &str, a: f64, b: f64) {
        self.expr.set_text(e);
        self.title.set_text("");
        self.source.set_selected(0);
        self.xmin.set_text(&a.to_string());
        self.xmax.set_text(&b.to_string());
        self.canvas.series.borrow_mut().clear();
        self.color_index.set(0);
        self.add_from_form();
    }

    fn apply_to_selected(self: &Rc<Self>) {
        let Some(i) = self.series_list.selected_row().map(|r| r.index() as usize) else {
            self.add_from_form();
            return;
        };
        if let Some(s) = self.canvas.series.borrow_mut().get_mut(i) {
            self.from_form(s);
        }
        self.refresh_list();
        self.series_list.select_row(self.series_list.row_at_index(i as i32).as_ref());
        self.recompute(true);
    }

    fn remove_selected(self: &Rc<Self>) {
        let Some(i) = self.series_list.selected_row().map(|r| r.index() as usize) else { return };
        {
            let mut s = self.canvas.series.borrow_mut();
            if i < s.len() {
                s.remove(i);
            }
        }
        self.refresh_list();
        self.canvas.fit_view();
        self.area.queue_draw();
    }

    fn load_selected(&self) {
        let Some(i) = self.series_list.selected_row().map(|r| r.index() as usize) else { return };
        let Some(s) = self.canvas.series.borrow().get(i).cloned() else { return };
        self.expr.set_text(&s.expression);
        self.title.set_text(&s.title);
        self.source.set_selected(s.source as u32);
        self.variable.set_text(&s.variable);
        self.style.set_selected(s.style as u32);
        self.smooth.set_selected(s.smooth as u32);
        self.axis.set_selected(s.secondary as u32);
        self.imag.set_active(s.imag_parts);
    }

    fn refresh_list(&self) {
        let sel = self.series_list.selected_row().map(|r| r.index());
        while let Some(r) = self.series_list.row_at_index(0) {
            self.series_list.remove(&r);
        }
        for s in self.canvas.series.borrow().iter() {
            let l = label(&s.name());
            l.set_margin_top(3);
            l.set_margin_bottom(3);
            l.set_margin_start(6);
            self.series_list.append(&l);
        }
        if let Some(i) = sel {
            self.series_list.select_row(self.series_list.row_at_index(i).as_ref());
        }
    }

    fn apply_view_fields(&self) {
        let mut v = self.canvas.v.borrow_mut();
        v.title = self.ptitle.text().to_string();
        v.xlabel = self.xlabel.text().to_string();
        v.ylabel = self.ylabel.text().to_string();
        v.grid = self.grid.is_active();
        v.border = self.border.is_active();
        v.log_x = self.logx.is_active();
        v.log_y = self.logy.is_active();
        v.polar = self.polar.is_active();
        v.log_base = parse_d(&self.log_base.text(), 10.0).max(1.0001);
        v.legend = self.legend.selected() as usize;
        v.line_width = self.width.value();
        v.y_min = if self.ymin.text().trim().is_empty() { f64::NAN } else { parse_d(&self.ymin.text(), f64::NAN) };
        v.y_max = if self.ymax.text().trim().is_empty() { f64::NAN } else { parse_d(&self.ymax.text(), f64::NAN) };
    }

    fn apply_appearance(self: &Rc<Self>, refit: bool) {
        self.apply_view_fields();
        if refit {
            self.recompute(true);
        } else {
            self.area.queue_draw();
        }
    }

    fn replot(self: &Rc<Self>) {
        self.apply_view_fields();
        if self.canvas.series.borrow().is_empty() {
            self.add_from_form();
        } else {
            self.recompute(true);
        }
    }

    fn recompute(self: &Rc<Self>, fit: bool) {
        self.apply_view_fields();
        let (mut a, mut b) = (parse_d(&self.xmin.text(), -10.0), parse_d(&self.xmax.text(), 10.0));
        if b <= a {
            std::mem::swap(&mut a, &mut b);
            if a == b {
                b = a + 1.0;
            }
        }
        let sv = parse_d(&self.samples.text(), 1001.0);
        let n = if self.use_step.is_active() { ((b - a) / sv.max(1e-12) + 1.0).clamp(2.0, 200000.0) as usize } else { sv.clamp(2.0, 200000.0) as usize };
        {
            let mut v = self.canvas.v.borrow_mut();
            v.x_min = a;
            v.x_max = b;
        }
        let (polar, log_x) = (self.polar.is_active(), self.logx.is_active());
        let list: Vec<Series> = self.canvas.series.borrow().clone();
        let w = Rc::downgrade(self);
        self.app().svc.run(
            move |c| list.iter().map(|s| compute(c, s, a, b, n, polar, log_x)).collect::<Vec<_>>(),
            move |results| {
                let Some(p) = w.upgrade() else { return };
                {
                    let mut ss = p.canvas.series.borrow_mut();
                    for (s, (pts, im, err)) in ss.iter_mut().zip(results) {
                        s.points = pts;
                        s.imag = im;
                        s.error = err;
                    }
                }
                p.refresh_list();
                if fit {
                    p.canvas.fit_view();
                }
                p.area.queue_draw();
                let errs: Vec<String> = p.canvas.series.borrow().iter().filter_map(|s| s.error.as_ref().map(|e| format!("{}: {}", s.expression, e))).collect();
                if !errs.is_empty() && fit {
                    message(p.app().parent(), &errs.join("\n"));
                }
            },
        );
    }

    fn render_texture(&self, w: i32, h: i32) -> Option<gdk::Texture> {
        let surf = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).ok()?;
        {
            let cr = cairo::Context::new(&surf).ok()?;
            let dark = crate::ui::theme::is_dark();
            let (fg, bg) = if dark { ((0.9, 0.91, 0.93), (0.12, 0.13, 0.15)) } else { ((0.1, 0.1, 0.12), (1.0, 1.0, 1.0)) };
            self.canvas.render(&cr, w as f64, h as f64, false, fg, Some(bg));
        }
        surf.flush();
        let stride = surf.stride() as usize;
        let data = surf.take_data().ok()?;
        let bytes = glib::Bytes::from(&data[..]);
        Some(gdk::MemoryTexture::new(w, h, gdk::MemoryFormat::B8g8r8a8Premultiplied, &bytes, stride).upcast())
    }

    fn save_png(self: &Rc<Self>) {
        let (w, h) = (self.area.width().max(800), self.area.height().max(500));
        let Some(tex) = self.render_texture(w, h) else { return };
        let app = self.app();
        save_file(app.parent(), "Save PNG", "plot.png", move |p| {
            let _ = tex.save_to_png(p);
        });
    }

    fn copy_image(&self) {
        if let (Some(tex), Some(d)) = (self.render_texture(self.area.width().max(100), self.area.height().max(100)), gdk::Display::default()) {
            d.clipboard().set_texture(&tex);
        }
    }
}

type Computed = (Vec<(f64, f64)>, Vec<(f64, f64)>, Option<String>);

fn value_of(e: &Expr) -> f64 {
    match units::try_quantity(e) {
        Some((v, _, rest)) if rest.is_one() && v.is_real() => v.re.to_f64(),
        _ => f64::NAN,
    }
}

fn compute(c: &Calculator, s: &Series, a: f64, b: f64, n: usize, polar: bool, log_x: bool) -> Computed {
    let mut pts = vec![];
    let mut imag = vec![];
    let r = attempt(|| {
        let mut ps = parser::Parser::new(&s.expression, c);
        ps.local_symbols.insert(s.variable.clone());
        let Some(e) = ps.parse_all() else { return Some("empty expression".to_string()) };
        if s.source == Source::Function {
            let pre = c.with_exact(true, || c.ev.eval(&e));
            for i in 0..n {
                c.check_abort();
                let t = i as f64 / (n - 1) as f64;
                let x = if log_x && a > 0.0 { (a.ln() + (b.ln() - a.ln()) * t).exp() } else { a + (b - a) * t };
                let (re, im) = c.ev.eval_complex(&pre, &s.variable, x);
                let y = if im.abs() < 1e-12 * re.abs().max(1.0) || s.imag_parts { re } else { f64::NAN };
                if polar {
                    pts.push((y * x.cos(), y * x.sin()));
                } else {
                    pts.push((x, y));
                    if s.imag_parts {
                        imag.push((x, im));
                    }
                }
            }
            return None;
        }
        let v = c.run(&e, &[]);
        let E::Mat(rows, vector) = &v.e else { return Some("not a vector or matrix".to_string()) };
        if s.source == Source::Paired {
            if !vector && rows.first().map(|r| r.len()) == Some(2) {
                for r in rows {
                    pts.push((value_of(&r[0]), value_of(&r[1])));
                }
            } else if rows.len() == 2 {
                for j in 0..rows[0].len() {
                    pts.push((value_of(&rows[0][j]), value_of(&rows[1][j])));
                }
            } else {
                return Some("a paired matrix needs two rows or two columns".to_string());
            }
        } else {
            for (j, it) in rows.iter().flatten().enumerate() {
                pts.push(((j + 1) as f64, value_of(it)));
            }
        }
        None
    });
    let err = match r {
        Ok(e) => e,
        Err(m) => Some(m),
    };
    (pts, imag, err)
}