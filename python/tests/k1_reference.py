"""Reference Keccak-256 and secp256k1 for verifying signatures the way the
Swig program does. Test-only; not constant time."""

from __future__ import annotations

import hashlib

_ROUND_CONSTANTS = (
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808A,
    0x8000000080008000,
    0x000000000000808B,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008A,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000A,
    0x000000008000808B,
    0x800000000000008B,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800A,
    0x800000008000000A,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
)
_ROTATIONS = (
    (0, 36, 3, 41, 18),
    (1, 44, 10, 45, 2),
    (62, 6, 43, 15, 61),
    (28, 55, 25, 21, 56),
    (27, 20, 39, 8, 14),
)
_MASK = (1 << 64) - 1


def _rotl(value: int, shift: int) -> int:
    return ((value << shift) | (value >> (64 - shift))) & _MASK if shift else value


def _keccak_f(state: list[list[int]]) -> None:
    for constant in _ROUND_CONSTANTS:
        parity = [
            state[x][0] ^ state[x][1] ^ state[x][2] ^ state[x][3] ^ state[x][4]
            for x in range(5)
        ]
        for x in range(5):
            mix = parity[(x - 1) % 5] ^ _rotl(parity[(x + 1) % 5], 1)
            for y in range(5):
                state[x][y] ^= mix
        moved = [[0] * 5 for _ in range(5)]
        for x in range(5):
            for y in range(5):
                moved[y][(2 * x + 3 * y) % 5] = _rotl(state[x][y], _ROTATIONS[x][y])
        for x in range(5):
            for y in range(5):
                state[x][y] = moved[x][y] ^ (
                    ~moved[(x + 1) % 5][y] & moved[(x + 2) % 5][y]
                )
        state[0][0] ^= constant


def keccak256(data: bytes) -> bytes:
    rate = 136
    padded = bytearray(data) + b"\x01"
    padded += bytes(-len(padded) % rate)
    padded[-1] |= 0x80
    state = [[0] * 5 for _ in range(5)]
    for start in range(0, len(padded), rate):
        block = padded[start : start + rate]
        for index in range(rate // 8):
            lane = int.from_bytes(block[index * 8 : index * 8 + 8], "little")
            state[index % 5][index // 5] ^= lane
        _keccak_f(state)
    return b"".join(
        state[index % 5][index // 5].to_bytes(8, "little") for index in range(4)
    )


P = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
N = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
G = (
    0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798,
    0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8,
)
Point = tuple[int, int] | None


def _add(left: Point, right: Point) -> Point:
    if left is None:
        return right
    if right is None:
        return left
    if left[0] == right[0] and (left[1] + right[1]) % P == 0:
        return None
    if left == right:
        slope = 3 * left[0] * left[0] * pow(2 * left[1], -1, P)
    else:
        slope = (right[1] - left[1]) * pow(right[0] - left[0], -1, P)
    x = (slope * slope - left[0] - right[0]) % P
    return x, (slope * (left[0] - x) - left[1]) % P


def _mul(scalar: int, point: Point) -> Point:
    result: Point = None
    while scalar:
        if scalar & 1:
            result = _add(result, point)
        point = _add(point, point)
        scalar >>= 1
    return result


def compressed_public_key(private_key: int) -> bytes:
    point = _mul(private_key, G)
    assert point is not None
    return bytes((2 + (point[1] & 1),)) + point[0].to_bytes(32, "big")


def sign_digest(private_key: int, digest: bytes) -> bytes:
    """Low-s r || s || v (v = 27 or 28) over a 32-byte digest."""
    z = int.from_bytes(digest, "big")
    k = (
        int.from_bytes(
            hashlib.sha256(private_key.to_bytes(32, "big") + digest).digest(), "big"
        )
        % N
    )
    point = _mul(k, G)
    assert point is not None
    r = point[0] % N
    s = pow(k, -1, N) * (z + r * private_key) % N
    recovery = point[1] & 1
    if s > N // 2:
        s = N - s
        recovery ^= 1
    return r.to_bytes(32, "big") + s.to_bytes(32, "big") + bytes((27 + recovery,))


def recover_compressed(digest: bytes, signature: bytes) -> bytes:
    """Recover the signer as sol_secp256k1_recover does, rejecting high s."""
    r = int.from_bytes(signature[:32], "big")
    s = int.from_bytes(signature[32:64], "big")
    if s > N // 2:
        raise ValueError("program rejects high-s signatures")
    recovery = 0 if signature[64] == 27 else 1
    y = pow((r * r * r + 7) % P, (P + 1) // 4, P)
    if y & 1 != recovery:
        y = P - y
    z = int.from_bytes(digest, "big")
    r_inverse = pow(r, -1, N)
    point = _add(_mul(s * r_inverse % N, (r, y)), _mul(-z * r_inverse % N, G))
    assert point is not None
    return bytes((2 + (point[1] & 1),)) + point[0].to_bytes(32, "big")
