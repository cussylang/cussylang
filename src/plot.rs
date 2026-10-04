use crate::{
    ast::Span,
    diagnostic::{Diagnostic, Result},
    runtime::Runtime,
    value::Value,
};
use std::fmt::Write;
impl Runtime {
    pub fn plot(&mut self, name: &str, path: &str, s: &Span) -> Result<()> {
        self.require_host_io("plot file output", s)?;
        let (lo, hi) = self.bounds.get(name).copied().unwrap_or((-10., 10.));
        let mut points = Vec::new();
        for i in 0..=800 {
            let t = lo + (hi - lo) * (i as f64 / 800.);
            let p = match self.call(name, vec![Value::Float(t)], s) {
                Ok(Value::Point(x, y, _)) => {
                    Some((x.borrow().value.number(s)?, y.borrow().value.number(s)?))
                }
                Ok(v) => Some((t, v.number(s)?)),
                Err(e) if e.code == "DOMAIN" || e.code == "AURA_OVERFLOW" => None,
                Err(e) => return Err(e),
            };
            points.push(p);
        }
        let equal_aspect = matches!(
            self.checked.functions[name].ret,
            crate::ast::Type::Point | crate::ast::Type::Vector
        );
        write_svg(
            path,
            name,
            &points,
            self.ranges.get(name).copied(),
            equal_aspect,
            s,
        )
    }
}
pub fn write_svg(
    path: &str,
    title: &str,
    points: &[Option<(f64, f64)>],
    range: Option<(f64, f64)>,
    equal_aspect: bool,
    s: &Span,
) -> Result<()> {
    let finite: Vec<(f64, f64)> = points
        .iter()
        .flatten()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    if finite.is_empty() {
        return Err(Diagnostic::new("DOMAIN", s, "plot has no finite samples"));
    }
    let mut xmin = finite.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let mut xmax = finite.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let (mut ymin, mut ymax) = range.unwrap_or_else(|| {
        (
            finite.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
            finite.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        )
    });
    if xmin == xmax {
        xmin -= 1.;
        xmax += 1.;
    }
    if ymin == ymax {
        ymin -= 1.;
        ymax += 1.;
    }
    if !(xmax - xmin).is_finite() || !(ymax - ymin).is_finite() {
        return Err(Diagnostic::new(
            "DOMAIN",
            s,
            "plot bounds overflow; choose smaller domain/range",
        ));
    }
    if range.is_none() {
        let pad = (ymax - ymin) * 0.05;
        ymin -= pad;
        ymax += pad;
    }
    // A parametric unit circle must look circular. Expand the viewport, never
    // stretch coordinates differently on the two axes for point-valued plots.
    if equal_aspect {
        let pixels_per_unit = (720. / (xmax - xmin)).min(440. / (ymax - ymin));
        let (cx, cy) = (xmin / 2. + xmax / 2., ymin / 2. + ymax / 2.);
        let (half_x, half_y) = (360. / pixels_per_unit, 220. / pixels_per_unit);
        xmin = cx - half_x;
        xmax = cx + half_x;
        ymin = cy - half_y;
        ymax = cy + half_y;
    }
    if ![xmin, xmax, ymin, ymax].iter().all(|v| v.is_finite()) || xmax <= xmin || ymax <= ymin {
        return Err(Diagnostic::new(
            "DOMAIN",
            s,
            "plot viewport cannot represent these bounds; rescale the data",
        ));
    }
    let sx = |x: f64| 60. + (x - xmin) / (xmax - xmin) * 720.;
    let sy = |y: f64| 510. - (y - ymin) / (ymax - ymin) * 440.;
    let escape = |t: &str| {
        t.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="840" height="580" viewBox="0 0 840 580" role="img" aria-label="{}"><rect width="840" height="580" fill="#101520"/><style>text{{font-family:monospace;fill:#c2cddd;font-size:12px}}.grid{{stroke:#293244;stroke-width:1}}</style><text x="60" y="32" style="font-size:20px;fill:#66efbd">CUSSY / {}</text><text x="60" y="555">compile, graph, jole. / {} finite samples</text><defs><clipPath id="plot"><rect x="60" y="70" width="720" height="440"/></clipPath></defs>"##,
        escape(title),
        escape(title),
        finite.len()
    );
    for i in 0..=10 {
        let x = 60. + 72. * i as f64;
        let y = 70. + 44. * i as f64;
        write!(
            svg,
            r#"<path class="grid" d="M {x} 70 V 510 M 60 {y} H 780"/>"#
        )
        .unwrap();
        if i % 2 == 0 {
            write!(svg,r#"<text x="{x}" y="533" text-anchor="middle">{:.2}</text><text x="52" y="{}" text-anchor="end">{:.2}</text>"#,xmin+(xmax-xmin)*i as f64/10.,y+4.,ymax-(ymax-ymin)*i as f64/10.).unwrap();
        }
    }
    if xmin <= 0. && xmax >= 0. {
        write!(
            svg,
            r##"<path stroke="#748299" d="M {} 70 V 510"/>"##,
            sx(0.)
        )
        .unwrap();
    }
    if ymin <= 0. && ymax >= 0. {
        write!(
            svg,
            r##"<path stroke="#748299" d="M 60 {} H 780"/>"##,
            sy(0.)
        )
        .unwrap();
    }
    let mut d = String::new();
    let mut previous: Option<(f64, f64)> = None;
    for p in points {
        if let Some((x, y)) =
            p.filter(|(x, y)| x.is_finite() && y.is_finite() && *y >= ymin && *y <= ymax)
        {
            let (x, y) = (sx(x), sy(y));
            let connect =
                previous.is_some_and(|(px, py)| (py - y).abs() < 220. && (px - x).abs() < 200.);
            write!(d, "{} {x:.3} {y:.3} ", if connect { "L" } else { "M" }).unwrap();
            previous = Some((x, y));
        } else {
            previous = None;
        }
    }
    write!(svg,r##"<path d="{d}" fill="none" stroke="#66efbd" stroke-width="2.5" stroke-linejoin="round" clip-path="url(#plot)"/>"##).unwrap();
    if finite.len() == 1 {
        write!(
            svg,
            r##"<circle cx="{}" cy="{}" r="4" fill="#66efbd" clip-path="url(#plot)"/>"##,
            sx(finite[0].0),
            sy(finite[0].1)
        )
        .unwrap();
    }
    svg.push_str("</svg>");
    std::fs::write(path, svg)
        .map_err(|e| Diagnostic::new("IO", s, format!("cannot write SVG {path}: {e}")))
}
