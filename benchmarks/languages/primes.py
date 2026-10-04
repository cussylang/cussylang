def prime(n):
    if n < 2:
        return False
    d = 2
    while d * d <= n:
        if n % d == 0:
            return False
        d += 1
    return True


count = 0
for n in range(2, 3001):
    if prime(n):
        count += 1
print(count)
