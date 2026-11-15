# Independent reference for pwg-v1, written from the design doc (not from the Rust code).
import hashlib, json, sys
from argon2.low_level import hash_secret_raw, Type

LOWER = "abcdefghijklmnopqrstuvwxyz"
UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
DIGIT = "0123456789"
DEF_SYMBOLS = "!@#$%^&*-_"

def norm_site(s):
    s = s.strip()
    if "://" in s:
        s = s.split("://", 1)[1]
    for i, ch in enumerate(s):
        if ch in "/?#":
            s = s[:i]
            break
    return s.lower()  # ASCII-only inputs in these vectors

def norm_account(s):
    return "".join(c.lower() if c.isascii() else c for c in s.strip())

class Stream:
    def __init__(self, dk):
        self.buf = hashlib.shake_256(dk).digest(1 << 16)
        self.pos = 0
    def read(self, n):
        out = self.buf[self.pos:self.pos + n]
        self.pos += n
        return out

def below(st, n):
    limit = 65536 - (65536 % n)
    while True:
        v = int.from_bytes(st.read(2), "big")
        if v < limit:
            return v % n

def gen(key, site, account, counter, length, req, symbols=DEF_SYMBOLS):
    site, account = norm_site(site), norm_account(account)
    ctx = f"pwg-v1|{site}|{account}|{counter}".encode()
    salt = hashlib.sha3_256(ctx).digest()
    dk = hash_secret_raw(key.encode(), salt, time_cost=3, memory_cost=65536,
                         parallelism=4, hash_len=32, type=Type.ID, version=19)
    st = Stream(dk)
    classes = [(a, n) for a, n in zip([LOWER, UPPER, DIGIT, symbols], req) if n > 0]
    union = "".join(a for a, _ in classes)
    out = []
    for a, n in classes:
        for _ in range(n):
            out.append(a[below(st, len(a))])
    while len(out) < length:
        out.append(union[below(st, len(union))])
    for i in range(len(out) - 1, 0, -1):
        j = below(st, i + 1)
        out[i], out[j] = out[j], out[i]
    return "".join(out)

KEY = "pwg golden test key"
CASES = [
    # (cli args after the action, site, account, counter, length, req, symbols)
    ("default rules",        ["github.com", "--account", "budi@gmail.com"], "github.com", "budi@gmail.com", 1, 20, [1,1,1,1], DEF_SYMBOLS),
    ("counter 2",            ["github.com", "--account", "budi@gmail.com", "--counter", "2"], "github.com", "budi@gmail.com", 2, 20, [1,1,1,1], DEF_SYMBOLS),
    ("min length",           ["github.com", "--account", "budi@gmail.com", "--length", "1", "--require", "l1"], "github.com", "budi@gmail.com", 1, 1, [1,0,0,0], DEF_SYMBOLS),
    ("max length",           ["github.com", "--account", "budi@gmail.com", "--length", "128"], "github.com", "budi@gmail.com", 1, 128, [1,1,1,1], DEF_SYMBOLS),
    ("custom symbols",       ["example.org", "--account", "Ani", "--length", "24", "--require", "l1,u1,d1,s3", "--symbols", "-_"], "example.org", "ani", 1, 24, [1,1,1,3], "-_"),
    ("digits only",          ["bank.example", "--account", "ani", "--length", "8", "--require", "d1"], "bank.example", "ani", 1, 8, [0,0,1,0], DEF_SYMBOLS),
    ("upper only",           ["bank.example", "--account", "ani", "--length", "12", "--require", "u1"], "bank.example", "ani", 1, 12, [0,1,0,0], DEF_SYMBOLS),
    ("lower and digit",      ["localhost:8080", "--account", "root", "--length", "16", "--require", "l2,d2", "--counter", "7"], "localhost:8080", "root", 7, 16, [2,0,2,0], DEF_SYMBOLS),
    ("symbols only",         ["x.io", "--account", "a", "--length", "10", "--require", "s10", "--symbols", "!?"], "x.io", "a", 1, 10, [0,0,0,10], "!?"),
    ("spelling 1",           ["HTTPS://GitHub.com/x?y", "--account", "  Budi@Gmail.com "], "github.com", "budi@gmail.com", 1, 20, [1,1,1,1], DEF_SYMBOLS),
    ("spelling 2",           ["  github.com ", "--account", "budi@gmail.com"], "github.com", "budi@gmail.com", 1, 20, [1,1,1,1], DEF_SYMBOLS),
    ("spelling 3",           ["github.com/login#top", "--account", "BUDI@GMAIL.COM"], "github.com", "budi@gmail.com", 1, 20, [1,1,1,1], DEF_SYMBOLS),
]

if __name__ == "__main__":
    rows = []
    for name, cli, site, acct, counter, length, req, syms in CASES:
        pw = gen(KEY, site, acct, counter, length, req, syms)
        rows.append({"name": name, "args": ["gen"] + cli, "password": pw})
    json.dump({"key": KEY, "cases": rows}, open(sys.argv[1], "w"), indent=2)
    print(len(rows), "vectors")
