fn prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

fn main() {
    let mut count: i64 = 0;
    for n in 2..=3000 {
        if prime(n) {
            count += 1;
        }
    }
    println!("{}", count);
}
