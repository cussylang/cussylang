use crate::{
    ast::{Span, Type},
    diagnostic::{Diagnostic, Result},
    runtime::Runtime,
    value::*,
};
use std::{
    collections::BTreeMap,
    io::{self, Read},
};
impl Runtime {
    pub fn native(&mut self, n: &str, a: Vec<Value>, s: &Span) -> Result<Value> {
        use Value::*;
        let err = |m: &str| Diagnostic::new("DOMAIN", s, m);
        let num = |i: usize| a.get(i).ok_or_else(|| err("missing argument"))?.number(s);
        let text = |i: usize| a.get(i).ok_or_else(|| err("missing argument"))?.string(s);
        let int = |i: usize| a.get(i).ok_or_else(|| err("missing argument"))?.integer(s);
        let finite = |v: f64| {
            if v.is_finite() {
                Ok(Float(v))
            } else {
                Err(err("math domain error or non-finite result"))
            }
        };
        match n {
            "jole" => {
                let mut text = if a.is_empty() {
                    "jole".into()
                } else {
                    a.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                self.emit(&text, s)?;
                Ok(Void)
            }
            "yap" => {
                let result = format_yap(&text(0)?, &a[1..], s)?;
                self.emit(&result, s)?;
                Ok(Void)
            }
            "listen" => {
                self.require_host_io("standard input access", s)?;
                let mut text = std::string::String::new();
                io::stdin()
                    .read_line(&mut text)
                    .map_err(|e| Diagnostic::new("IO", s, e.to_string()))?;
                Ok(String(text.trim_end_matches(['\r', '\n']).into()))
            }
            "assert" => {
                if !a[0].truth() {
                    return Err(Diagnostic::new(
                        "ASSERT",
                        s,
                        a.get(1)
                            .map_or("assertion failed".into(), ToString::to_string),
                    ));
                }
                Ok(Void)
            }
            "len" => Ok(Int(match &a[0] {
                String(t) => t.chars().count(),
                List(v) | Array(v, _) => v.len(),
                _ => return Err(err("len requires string, list or array")),
            } as i64)),
            "hitbox" => Ok(Int(logical_size(&a[0]) as i64)),
            "to_string" => Ok(String(a[0].to_string())),
            "to_int" => Ok(Int(match &a[0] {
                Int(n) => *n,
                UInt(n) => {
                    i64::try_from(*n).map_err(|_| err("unsigned value exceeds int range"))?
                }
                Float(n) if *n >= i64::MIN as f64 && *n < (i64::MAX as f64) => *n as i64,
                Bool(b) => i64::from(*b),
                Char(c) => *c as i64,
                String(t) => t
                    .parse()
                    .map_err(|_| err("string is not a signed integer"))?,
                _ => return Err(err("to_int value out of range or unsupported")),
            })),
            "to_float" => {
                if let String(t) = &a[0] {
                    finite(t.parse().map_err(|_| err("string is not a float"))?)
                } else {
                    finite(num(0)?)
                }
            }
            "alloc" => {
                let v = a[0].copy();
                let t = v.ty();
                let c = cell(v, t.clone(), true);
                c.borrow_mut().heap = true;
                Ok(Pointer(Some(c), t))
            }
            "free" => {
                match &a[0] {
                    Pointer(None, _) => {}
                    Pointer(Some(c), _) => {
                        let slot = c.borrow();
                        if !slot.alive {
                            return Err(Diagnostic::new(
                                "COOKED",
                                s,
                                "double free or expired storage",
                            ));
                        }
                        if !slot.heap {
                            return Err(Diagnostic::new(
                                "COOKED",
                                s,
                                "free only accepts storage returned by alloc",
                            ));
                        }
                        drop(slot);
                        invalidate(c);
                    }
                    _ => return Err(Diagnostic::new("COOKED", s, "free requires pointer")),
                }
                Ok(Void)
            }
            "__sin" => finite(num(0)?.sin()),
            "__cos" => finite(num(0)?.cos()),
            "__tan" => finite(num(0)?.tan()),
            "__sqrt" => finite(num(0)?.sqrt()),
            "__abs" => finite(num(0)?.abs()),
            "__floor" => finite(num(0)?.floor()),
            "__ceil" => finite(num(0)?.ceil()),
            "__round" => finite(num(0)?.round()),
            "__exp" => finite(num(0)?.exp()),
            "__log" => finite(num(0)?.ln()),
            "__pow" => finite(num(0)?.powf(num(1)?)),
            "__min" => finite(min_number(num(0)?, num(1)?)),
            "__max" => finite(max_number(num(0)?, num(1)?)),
            "__atan2" => finite(num(0)?.atan2(num(1)?)),
            "__clock" => finite(self.started.elapsed().as_secs_f64()),
            "__sleep" => {
                self.require_host_io("sleep", s)?;
                let t = num(0)?;
                if !(0.0..=60.0).contains(&t) {
                    return Err(err("sleep seconds must be in 0..60"));
                }
                std::thread::sleep(std::time::Duration::from_secs_f64(t));
                Ok(Void)
            }
            "__random" => {
                self.rng ^= self.rng << 13;
                self.rng ^= self.rng >> 7;
                self.rng ^= self.rng << 17;
                finite((self.rng >> 11) as f64 / ((1u64 << 53) as f64))
            }
            "__seed" => {
                self.rng = int(0)? as u64;
                if self.rng == 0 {
                    self.rng = 1;
                }
                Ok(Void)
            }
            "__read_file" => {
                self.require_host_io("file reads", s)?;
                let path = text(0)?;
                let file = std::fs::File::open(&path)
                    .map_err(|e| Diagnostic::new("IO", s, format!("cannot read {path}: {e}")))?;
                Ok(String(read_text(file, s)?))
            }
            "__write_file" => {
                self.require_host_io("file writes", s)?;
                std::fs::write(text(0)?, text(1)?)
                    .map_err(|e| Diagnostic::new("IO", s, e.to_string()))?;
                Ok(Void)
            }
            "__file_exists" => {
                self.require_host_io("filesystem access", s)?;
                Ok(Bool(std::path::Path::new(&text(0)?).is_file()))
            }
            "__env" => {
                self.require_host_io("environment access", s)?;
                Ok(String(std::env::var(text(0)?).unwrap_or_default()))
            }
            "__arg" => {
                let i = int(0)?;
                if i < 0 {
                    return Err(err("argument index must be nonnegative"));
                }
                Ok(String(
                    self.options
                        .args
                        .get(i as usize)
                        .cloned()
                        .ok_or_else(|| err("argument index out of bounds"))?,
                ))
            }
            "__argc" => Ok(Int(self.options.args.len() as i64)),
            "__slice" => {
                let chars: Vec<char> = text(0)?.chars().collect();
                let start = int(1)?;
                let count = int(2)?;
                if start < 0
                    || count < 0
                    || (start as usize) > chars.len()
                    || (count as usize) > chars.len() - start as usize
                {
                    return Err(err("slice exceeds string bounds"));
                }
                Ok(String(
                    chars[start as usize..start as usize + count as usize]
                        .iter()
                        .collect(),
                ))
            }
            "__contains" => Ok(Bool(text(0)?.contains(&text(1)?))),
            "__replace" => {
                let out = text(0)?.replace(&text(1)?, &text(2)?);
                if out.len() > 8 * 1024 * 1024 {
                    return Err(err("replacement exceeds 8 MiB"));
                }
                Ok(String(out))
            }
            "__upper" => Ok(String(text(0)?.to_uppercase())),
            "__lower" => Ok(String(text(0)?.to_lowercase())),
            "__sum" | "__mean" => {
                let values = a[0].elements(s)?;
                if n == "__mean" && values.is_empty() {
                    return Err(err("mean requires at least one observation"));
                }
                let mut sum = 0.;
                for v in &values {
                    sum += v.number(s)?;
                }
                finite(if n == "__mean" {
                    sum / values.len() as f64
                } else {
                    sum
                })
            }
            "__linspace" => {
                let lo = num(0)?;
                let hi = num(1)?;
                let n = int(2)?;
                if !(2..=100000).contains(&n) {
                    return Err(err("linspace count must be 2..100000"));
                }
                let mut out = Vec::new();
                for i in 0..n {
                    self.tick(s)?;
                    let v = lo + (hi - lo) * (i as f64 / (n - 1) as f64);
                    if !v.is_finite() {
                        return Err(err("linspace overflow"));
                    }
                    out.push(cell(Float(v), Type::Float, true));
                }
                Ok(List(out))
            }
            "__regress" => {
                let x = a[0].elements(s)?;
                let y = a[1].elements(s)?;
                if x.len() != y.len() || x.len() < 2 {
                    return Err(err(
                        "regression requires equal lists of at least two observations",
                    ));
                }
                let xs: Vec<f64> = x.iter().map(|v| v.number(s)).collect::<Result<_>>()?;
                let ys: Vec<f64> = y.iter().map(|v| v.number(s)).collect::<Result<_>>()?;
                let count = x.len() as f64;
                let mx = xs.iter().sum::<f64>() / count;
                let my = ys.iter().sum::<f64>() / count;
                let xx = xs.iter().map(|v| (v - mx).powi(2)).sum::<f64>();
                let yy = ys.iter().map(|v| (v - my).powi(2)).sum::<f64>();
                let xy = xs
                    .iter()
                    .zip(&ys)
                    .map(|(x, y)| (x - mx) * (y - my))
                    .sum::<f64>();
                if xx == 0. {
                    return Err(err("regression needs distinct x values"));
                }
                let slope = xy / xx;
                let intercept = my - slope * mx;
                let r2 = if yy == 0. {
                    1.
                } else {
                    (xy * xy / (xx * yy)).clamp(0., 1.)
                };
                let mut fields = BTreeMap::new();
                for (k, v) in [("slope", slope), ("intercept", intercept), ("r2", r2)] {
                    finite(v)?;
                    fields.insert(k.into(), cell(Float(v), Type::Float, true));
                }
                Ok(Record("Regression".into(), fields))
            }
            "__plot_points" => {
                self.require_host_io("plot file output", s)?;
                let mut points = Vec::new();
                for p in a[0].elements(s)? {
                    if let Point(x, y, _) = p {
                        points.push(Some((
                            x.borrow().value.number(s)?,
                            y.borrow().value.number(s)?,
                        )));
                    } else {
                        return Err(err("plot_points requires point[]"));
                    }
                }
                crate::plot::write_svg(&text(1)?, "Cussy point graph", &points, None, true, s)?;
                Ok(Void)
            }
            "__streamstatus" => Ok(Bool(self.stream_depth > 0 || self.hopped)),
            "__hoponstream" => {
                self.hopped = true;
                self.emit("whitecaplol hop on stream\n", s)?;
                Ok(Void)
            }
            "__native1" => {
                self.require_host_io("native FFI", s)?;
                if !self.options.allow_ffi {
                    return Err(Diagnostic::new("FFI",s,"native calls require --allow-ffi").help("only enable for a trusted library and a known double(double) C ABI symbol"));
                }
                finite(crate::ffi::native1(&text(0)?, &text(1)?, num(2)?, s)?)
            }
            _ => Err(Diagnostic::new(
                "D404",
                s,
                format!("unknown native function `{n}`"),
            )),
        }
    }
}
// Give signed-zero ties a language-defined result, independent of LLVM/libm.
fn min_number(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else {
        a.min(b)
    }
}
fn max_number(a: f64, b: f64) -> f64 {
    if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() && b.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else {
        a.max(b)
    }
}

fn read_text(input: impl Read, s: &Span) -> Result<String> {
    const MAX_BYTES: usize = 8 * 1024 * 1024;
    let mut bytes = Vec::new();
    // Read only one byte beyond the limit, including for streams without a known size.
    input
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| Diagnostic::new("IO", s, e.to_string()))?;
    if bytes.len() > MAX_BYTES {
        return Err(Diagnostic::new("DOMAIN", s, "file exceeds 8 MiB"));
    }
    String::from_utf8(bytes).map_err(|_| Diagnostic::new("DOMAIN", s, "file is not UTF-8"))
}

fn logical_size(v: &Value) -> usize {
    match v {
        Value::Bool(_) => 1,
        Value::Char(_) => 4,
        Value::String(s) => s.len(),
        Value::Void => 0,
        Value::List(a) | Value::Array(a, _) => {
            a.iter().map(|v| logical_size(&v.borrow().value)).sum()
        }
        Value::Point(..) => 16,
        Value::Record(_, m) => m.values().map(|v| logical_size(&v.borrow().value)).sum(),
        _ => 8,
    }
}
fn format_yap(fmt: &str, args: &[Value], s: &Span) -> Result<String> {
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    let mut i = 0;
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push('%');
            continue;
        }
        let mut width = String::new();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            width.push(chars.next().unwrap());
        }
        let width: usize = if width.is_empty() {
            0
        } else {
            width
                .parse()
                .map_err(|_| Diagnostic::new("FORMAT", s, "invalid format width"))?
        };
        if width > 1000 {
            return Err(Diagnostic::new("FORMAT", s, "format width exceeds 1000"));
        }
        let precision = if chars.peek() == Some(&'.') {
            chars.next();
            let mut p = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                p.push(chars.next().unwrap());
            }
            let p = p
                .parse::<usize>()
                .map_err(|_| Diagnostic::new("FORMAT", s, "invalid precision"))?;
            if p > 15 {
                return Err(Diagnostic::new("FORMAT", s, "precision exceeds 15"));
            }
            Some(p)
        } else {
            None
        };
        let spec = chars
            .next()
            .ok_or_else(|| Diagnostic::new("FORMAT", s, "unfinished percent format"))?;
        let a = args
            .get(i)
            .ok_or_else(|| Diagnostic::new("FORMAT", s, "not enough format arguments"))?;
        i += 1;
        let v = match (spec, a) {
            ('d', Value::Int(v)) => v.to_string(),
            ('u', Value::UInt(v)) => v.to_string(),
            ('f', v) if v.ty().numeric() => format!("{:.*}", precision.unwrap_or(6), v.number(s)?),
            ('s', Value::String(v)) => v.clone(),
            ('c', Value::Char(v)) => v.to_string(),
            ('b', Value::Bool(_)) | ('v', _) => a.to_string(),
            _ => {
                return Err(Diagnostic::new(
                    "FORMAT",
                    s,
                    format!("format %{spec} does not accept {}", a.ty()),
                ));
            }
        };
        out.push_str(&format!("{v:>width$}"));
    }
    if i != args.len() {
        return Err(Diagnostic::new("FORMAT", s, "too many format arguments"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_read_limit_bounds_consumption() {
        let mut source = io::repeat(b'x').take(16 * 1024 * 1024);
        let err = read_text(&mut source, &Span::default()).unwrap_err();
        assert_eq!(err.code, "DOMAIN");
        assert_eq!(source.limit(), 8 * 1024 * 1024 - 1);
    }

    #[test]
    fn file_read_accepts_utf8_and_rejects_invalid_bytes() {
        assert_eq!(
            read_text("jole λ".as_bytes(), &Span::default()).unwrap(),
            "jole λ"
        );
        assert_eq!(
            read_text(&[0xff][..], &Span::default()).unwrap_err().code,
            "DOMAIN"
        );
    }
}
