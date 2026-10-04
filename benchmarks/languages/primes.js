function prime(n) {
    if (n < 2) { return false; }
    for (let d = 2; d * d <= n; d++) {
        if (n % d === 0) { return false; }
    }
    return true;
}

let count = 0;
for (let n = 2; n <= 3000; n++) {
    if (prime(n)) { count++; }
}
console.log(count);
