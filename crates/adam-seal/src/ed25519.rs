//! Ed25519 (RFC 8032), pure Rust, no dependencies.
//!
//! Correctness over speed: field/scalar reduction is plain binary long
//! division rather than pseudo-Mersenne folding.  A protocol is signed
//! once per briefing and verified rarely, so the extra microseconds are
//! irrelevant; what matters is that the whole path is auditable in-tree
//! and reproduces the official RFC 8032 known-answer vectors bit-for-bit
//! (see `tests/rfc8032.rs`).
//!
//! NOT constant-time: comparisons and the double-and-add ladder branch on
//! secret bits.  Acceptable for offline on-device sealing — there is no
//! remote timing oracle — but do not reuse this for an online service.

use crate::sha2::sha512;

// 256-bit little-endian unsigned integer (limb 0 = least significant).
type U256 = [u64; 4];

// p = 2^255 - 19
const P: U256 = [
    0xffff_ffff_ffff_ffed,
    0xffff_ffff_ffff_ffff,
    0xffff_ffff_ffff_ffff,
    0x7fff_ffff_ffff_ffff,
];

// L = 2^252 + 27742317777372353535851937790883648493  (group order)
const L: U256 = [
    0x5812_631a_5cf5_d3ed,
    0x14de_f9de_a2f7_9cd6,
    0x0000_0000_0000_0000,
    0x1000_0000_0000_0000,
];

// ----------------------------------------------------------- big-int core

fn cmp(a: &U256, b: &U256) -> core::cmp::Ordering {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    core::cmp::Ordering::Equal
}

fn is_zero(a: &U256) -> bool {
    a == &[0, 0, 0, 0]
}

// a + b mod 2^256, returning (sum, carry).
fn adc(a: &U256, b: &U256) -> (U256, u64) {
    let mut out = [0u64; 4];
    let mut carry = 0u128;
    for i in 0..4 {
        let t = a[i] as u128 + b[i] as u128 + carry;
        out[i] = t as u64;
        carry = t >> 64;
    }
    (out, carry as u64)
}

// a - b mod 2^256, returning (diff, borrow).
fn sbb(a: &U256, b: &U256) -> (U256, u64) {
    let mut out = [0u64; 4];
    let mut borrow = 0i128;
    for i in 0..4 {
        let t = a[i] as i128 - b[i] as i128 - borrow;
        if t < 0 {
            out[i] = (t + (1i128 << 64)) as u64;
            borrow = 1;
        } else {
            out[i] = t as u64;
            borrow = 0;
        }
    }
    (out, borrow as u64)
}

// Schoolbook 256x256 -> 512-bit product (8 limbs, little-endian).
fn mul_wide(a: &U256, b: &U256) -> [u64; 8] {
    let mut out = [0u64; 8];
    for i in 0..4 {
        let mut carry: u128 = 0;
        for j in 0..4 {
            let t = a[i] as u128 * b[j] as u128 + out[i + j] as u128 + carry;
            out[i + j] = t as u64;
            carry = t >> 64;
        }
        out[i + 4] = carry as u64;
    }
    out
}

// x mod m via binary long division.  `m` must be < 2^255 so that the
// running remainder never overflows 256 bits after the left shift.
fn reduce_wide(x: &[u64; 8], m: &U256) -> U256 {
    let mut r: U256 = [0, 0, 0, 0];
    for bit in (0..512).rev() {
        // r <<= 1
        let mut c = (x[bit / 64] >> (bit % 64)) & 1;
        for limb in r.iter_mut() {
            let new = (*limb << 1) | c;
            c = *limb >> 63;
            *limb = new;
        }
        // (top carry `c` is always 0 here because r < m < 2^255)
        if cmp(&r, m) != core::cmp::Ordering::Less {
            r = sbb(&r, m).0;
        }
    }
    r
}

// ------------------------------------------------------------- field mod p

fn fe_add(a: &U256, b: &U256) -> U256 {
    let (s, _carry) = adc(a, b); // a,b < p < 2^255 so a+b < 2^256, no real carry
    if cmp(&s, &P) != core::cmp::Ordering::Less {
        sbb(&s, &P).0
    } else {
        s
    }
}

fn fe_sub(a: &U256, b: &U256) -> U256 {
    if cmp(a, b) != core::cmp::Ordering::Less {
        sbb(a, b).0
    } else {
        let (t, _) = adc(a, &P);
        sbb(&t, b).0
    }
}

fn fe_mul(a: &U256, b: &U256) -> U256 {
    reduce_wide(&mul_wide(a, b), &P)
}

fn fe_sq(a: &U256) -> U256 {
    fe_mul(a, a)
}

fn fe_neg(a: &U256) -> U256 {
    if is_zero(a) { *a } else { sbb(&P, a).0 }
}

// a^e mod p, exponent as U256.
fn fe_pow(a: &U256, e: &U256) -> U256 {
    let mut result: U256 = [1, 0, 0, 0];
    let mut base = *a;
    for i in 0..256 {
        if (e[i / 64] >> (i % 64)) & 1 == 1 {
            result = fe_mul(&result, &base);
        }
        base = fe_sq(&base);
    }
    result
}

fn fe_inv(a: &U256) -> U256 {
    // a^(p-2) mod p
    let (pm2, _) = sbb(&P, &[2, 0, 0, 0]);
    fe_pow(a, &pm2)
}

fn fe_from_u64(x: u64) -> U256 {
    [x, 0, 0, 0]
}

fn fe_is_odd(a: &U256) -> bool {
    a[0] & 1 == 1
}

fn fe_from_bytes(b: &[u8; 32]) -> U256 {
    let mut o = [0u64; 4];
    for i in 0..4 {
        o[i] = u64::from_le_bytes([
            b[i * 8],
            b[i * 8 + 1],
            b[i * 8 + 2],
            b[i * 8 + 3],
            b[i * 8 + 4],
            b[i * 8 + 5],
            b[i * 8 + 6],
            b[i * 8 + 7],
        ]);
    }
    o
}

fn fe_to_bytes(a: &U256) -> [u8; 32] {
    let mut o = [0u8; 32];
    for i in 0..4 {
        o[i * 8..i * 8 + 8].copy_from_slice(&a[i].to_le_bytes());
    }
    o
}

// (p-5)/8 and (p-1)/4, computed once from P.
fn exp_p_minus_5_div_8() -> U256 {
    let (pm5, _) = sbb(&P, &[5, 0, 0, 0]);
    shr3(&pm5)
}

fn shr3(a: &U256) -> U256 {
    let mut o = [0u64; 4];
    for i in 0..4 {
        o[i] = a[i] >> 3;
        if i + 1 < 4 {
            o[i] |= a[i + 1] << 61;
        }
    }
    o
}

// sqrt(-1) mod p = 2^((p-1)/4)
fn sqrt_m1() -> U256 {
    // (p-1)/4
    let (pm1, _) = sbb(&P, &[1, 0, 0, 0]);
    let mut e = [0u64; 4];
    for i in 0..4 {
        e[i] = pm1[i] >> 2;
        if i + 1 < 4 {
            e[i] |= pm1[i + 1] << 62;
        }
    }
    fe_pow(&fe_from_u64(2), &e)
}

// Edwards curve constant d = -121665 / 121666 mod p.
fn curve_d() -> U256 {
    let num = fe_neg(&fe_from_u64(121665));
    let den = fe_inv(&fe_from_u64(121666));
    fe_mul(&num, &den)
}

// ------------------------------------------------------------- curve points

// Extended twisted-Edwards coordinates (X : Y : Z : T), T = XY/Z.
#[derive(Clone)]
struct Point {
    x: U256,
    y: U256,
    z: U256,
    t: U256,
}

fn identity() -> Point {
    Point {
        x: [0, 0, 0, 0],
        y: [1, 0, 0, 0],
        z: [1, 0, 0, 0],
        t: [0, 0, 0, 0],
    }
}

// Unified/complete addition for twisted Edwards a=-1 (RFC 8032 §5.1.4).
// Complete: also correct when p == q, so it doubles as well.
fn point_add(p: &Point, q: &Point, d2: &U256) -> Point {
    let a = fe_mul(&fe_sub(&p.y, &p.x), &fe_sub(&q.y, &q.x));
    let b = fe_mul(&fe_add(&p.y, &p.x), &fe_add(&q.y, &q.x));
    let c = fe_mul(&fe_mul(&p.t, d2), &q.t);
    let dd = fe_mul(&fe_add(&p.z, &p.z), &q.z);
    let e = fe_sub(&b, &a);
    let f = fe_sub(&dd, &c);
    let g = fe_add(&dd, &c);
    let h = fe_add(&b, &a);
    Point {
        x: fe_mul(&e, &f),
        y: fe_mul(&g, &h),
        t: fe_mul(&e, &h),
        z: fe_mul(&f, &g),
    }
}

// [s]P via double-and-add over the low 256 bits of the scalar.
fn scalar_mul(s: &U256, p: &Point, d2: &U256) -> Point {
    let mut r = identity();
    for i in (0..256).rev() {
        r = point_add(&r, &r, d2);
        if (s[i / 64] >> (i % 64)) & 1 == 1 {
            r = point_add(&r, p, d2);
        }
    }
    r
}

fn point_eq(p: &Point, q: &Point) -> bool {
    // Compare affine coordinates: X1*Z2 == X2*Z1 and Y1*Z2 == Y2*Z1.
    fe_to_bytes(&fe_mul(&p.x, &q.z)) == fe_to_bytes(&fe_mul(&q.x, &p.z))
        && fe_to_bytes(&fe_mul(&p.y, &q.z)) == fe_to_bytes(&fe_mul(&q.y, &p.z))
}

// Compress a point to 32 bytes: little-endian y with the low bit of x in bit 255.
fn point_encode(p: &Point) -> [u8; 32] {
    let zinv = fe_inv(&p.z);
    let x = fe_mul(&p.x, &zinv);
    let y = fe_mul(&p.y, &zinv);
    let mut out = fe_to_bytes(&y);
    if fe_is_odd(&x) {
        out[31] |= 0x80;
    }
    out
}

// Decompress 32 bytes to a curve point, or None if not a valid encoding.
fn point_decode(b: &[u8; 32]) -> Option<Point> {
    let sign = (b[31] >> 7) & 1;
    let mut yb = *b;
    yb[31] &= 0x7f;
    let y = fe_from_bytes(&yb);
    if cmp(&y, &P) != core::cmp::Ordering::Less {
        return None; // y must be a canonical field element
    }

    let d = curve_d();
    let y2 = fe_sq(&y);
    let u = fe_sub(&y2, &fe_from_u64(1)); // y^2 - 1
    let v = fe_add(&fe_mul(&d, &y2), &fe_from_u64(1)); // d*y^2 + 1

    // x = (u/v)^((p+3)/8) computed as u*v^3 * (u*v^7)^((p-5)/8)
    let v3 = fe_mul(&fe_sq(&v), &v);
    let v7 = fe_mul(&fe_sq(&v3), &v);
    let uv7 = fe_mul(&u, &v7);
    let pow = fe_pow(&uv7, &exp_p_minus_5_div_8());
    let mut x = fe_mul(&fe_mul(&u, &v3), &pow);

    // Check v*x^2 == ±u; multiply by sqrt(-1) if it is the negative case.
    let vxx = fe_mul(&v, &fe_sq(&x));
    if fe_to_bytes(&vxx) != fe_to_bytes(&u) {
        if fe_to_bytes(&vxx) == fe_to_bytes(&fe_neg(&u)) {
            x = fe_mul(&x, &sqrt_m1());
        } else {
            return None; // no square root: not on curve
        }
    }

    if is_zero(&x) && sign == 1 {
        return None; // x == 0 with sign bit set is illegal
    }
    if fe_is_odd(&x) != (sign == 1) {
        x = fe_neg(&x);
    }

    let t = fe_mul(&x, &y);
    Some(Point {
        x,
        y,
        z: [1, 0, 0, 0],
        t,
    })
}

// Base point B (y = 4/5, x recovered with even sign).
fn base_point() -> Point {
    let y = fe_mul(&fe_from_u64(4), &fe_inv(&fe_from_u64(5)));
    let mut enc = fe_to_bytes(&y); // sign bit 0 -> even x
    enc[31] &= 0x7f;
    point_decode(&enc).expect("base point is valid")
}

// --------------------------------------------------------------- scalars

// Reduce a 64-byte little-endian value mod L.
fn scalar_reduce_wide(bytes: &[u8; 64]) -> U256 {
    // Long division of the full 512-bit value by L.
    let mut x = [0u64; 8];
    for i in 0..8 {
        x[i] = u64::from_le_bytes([
            bytes[i * 8],
            bytes[i * 8 + 1],
            bytes[i * 8 + 2],
            bytes[i * 8 + 3],
            bytes[i * 8 + 4],
            bytes[i * 8 + 5],
            bytes[i * 8 + 6],
            bytes[i * 8 + 7],
        ]);
    }
    reduce_wide(&x, &L)
}

// (a + b) mod L, inputs already < L.
fn scalar_add(a: &U256, b: &U256) -> U256 {
    let (s, carry) = adc(a, b);
    let mut r = s;
    if carry == 1 || cmp(&r, &L) != core::cmp::Ordering::Less {
        r = sbb(&r, &L).0;
    }
    r
}

// (a * b) mod L.
fn scalar_mul_mod(a: &U256, b: &U256) -> U256 {
    reduce_wide(&mul_wide(a, b), &L)
}

// Reduce a 256-bit value mod L.
fn scalar_reduce_256(a: &U256) -> U256 {
    let mut x = [0u64; 8];
    x[..4].copy_from_slice(a);
    reduce_wide(&x, &L)
}

// ------------------------------------------------------------- public API

/// Length of an Ed25519 secret seed, public key, and signature.
pub const SEED_LEN: usize = 32;
pub const PUBLIC_KEY_LEN: usize = 32;
pub const SIGNATURE_LEN: usize = 64;

/// Derive the 32-byte public key from a 32-byte secret seed.
pub fn public_key(seed: &[u8; 32]) -> [u8; 32] {
    let h = sha512(seed);
    let mut a_bytes = [0u8; 32];
    a_bytes.copy_from_slice(&h[0..32]);
    clamp(&mut a_bytes);
    let a = fe_from_bytes(&a_bytes); // scalar; used as integer for [a]B
    let d2 = fe_add(&curve_d(), &curve_d());
    let a_point = scalar_mul(&a, &base_point(), &d2);
    point_encode(&a_point)
}

/// Sign `msg` with the 32-byte secret seed, returning a 64-byte signature.
pub fn sign(seed: &[u8; 32], msg: &[u8]) -> [u8; 64] {
    let d2 = fe_add(&curve_d(), &curve_d());
    let b = base_point();

    let h = sha512(seed);
    let mut a_bytes = [0u8; 32];
    a_bytes.copy_from_slice(&h[0..32]);
    clamp(&mut a_bytes);
    let a_scalar = fe_from_bytes(&a_bytes);
    let a_point = scalar_mul(&a_scalar, &b, &d2);
    let a_enc = point_encode(&a_point);

    let prefix = &h[32..64];

    // r = H(prefix || msg) mod L
    let mut r_input = Vec::with_capacity(32 + msg.len());
    r_input.extend_from_slice(prefix);
    r_input.extend_from_slice(msg);
    let r = scalar_reduce_wide(&sha512(&r_input));
    let r_point = scalar_mul(&r, &b, &d2);
    let r_enc = point_encode(&r_point);

    // k = H(R || A || msg) mod L
    let mut k_input = Vec::with_capacity(64 + msg.len());
    k_input.extend_from_slice(&r_enc);
    k_input.extend_from_slice(&a_enc);
    k_input.extend_from_slice(msg);
    let k = scalar_reduce_wide(&sha512(&k_input));

    // S = (r + k*a) mod L
    let ka = scalar_mul_mod(&k, &scalar_reduce_256(&a_scalar));
    let s = scalar_add(&r, &ka);

    let mut sig = [0u8; 64];
    sig[0..32].copy_from_slice(&r_enc);
    sig[32..64].copy_from_slice(&fe_to_bytes(&s));
    sig
}

/// Verify a 64-byte signature over `msg` against a 32-byte public key.
pub fn verify(public: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool {
    let d2 = fe_add(&curve_d(), &curve_d());
    let b = base_point();

    let a_point = match point_decode(public) {
        Some(p) => p,
        None => return false,
    };
    let mut r_enc = [0u8; 32];
    r_enc.copy_from_slice(&sig[0..32]);
    let r_point = match point_decode(&r_enc) {
        Some(p) => p,
        None => return false,
    };
    let mut s_bytes = [0u8; 32];
    s_bytes.copy_from_slice(&sig[32..64]);
    let s = fe_from_bytes(&s_bytes);
    // S must be canonical (< L), else the signature is malleable/invalid.
    if cmp(&s, &L) != core::cmp::Ordering::Less {
        return false;
    }

    // k = H(R || A || msg) mod L
    let mut k_input = Vec::with_capacity(64 + msg.len());
    k_input.extend_from_slice(&r_enc);
    k_input.extend_from_slice(public);
    k_input.extend_from_slice(msg);
    let k = scalar_reduce_wide(&sha512(&k_input));

    // Check [S]B == R + [k]A
    let lhs = scalar_mul(&s, &b, &d2);
    let ka = scalar_mul(&k, &a_point, &d2);
    let rhs = point_add(&r_point, &ka, &d2);
    point_eq(&lhs, &rhs)
}

fn clamp(a: &mut [u8; 32]) {
    a[0] &= 248;
    a[31] &= 127;
    a[31] |= 64;
}
