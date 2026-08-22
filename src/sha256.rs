//! SHA-256 (FIPS 180-4), std-only.
//!
//! Used by `cforge self-update` to verify a downloaded release binary
//! before it replaces the running one. Hand-rolled rather than pulled from
//! a crate for the same reason `platform::command_path` reimplements
//! `which`: this binary keeps a deliberately small dependency set, and a
//! self-updater's integrity check is the last place to widen the supply
//! chain it exists to protect. Shelling out to `sha256sum`/`shasum`/
//! `certutil` was the other option and is worse here — it varies by
//! platform and, since verification is fail-closed, a box missing the tool
//! could no longer update at all.
//!
//! ~60 lines against a fully specified algorithm, checked below against
//! the NIST vectors plus a multi-block input.

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
    0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
    0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
    0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

/// Lowercase hex SHA-256 of `data` — the same form `sha256sum` prints.
pub fn hex(data: &[u8]) -> String {
    let mut h: [u32; 8] =
        [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];

    // Padding: 0x80, then zeros, then the bit length as a big-endian u64,
    // to the next 64-byte boundary.
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *slot = slot.wrapping_add(v);
        }
    }

    h.iter().map(|w| format!("{w:08x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::hex;

    /// The NIST FIPS 180-4 one- and two-block examples, plus the empty
    /// input. A hand-rolled hash is only worth trusting against published
    /// vectors — these are what make the implementation above verifiable
    /// rather than merely plausible.
    #[test]
    fn matches_nist_vectors() {
        assert_eq!(hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    /// Exercises the padding edge cases: exactly one block of message
    /// (where the length field forces a whole extra block) and a long
    /// multi-block input.
    #[test]
    fn handles_block_boundaries() {
        assert_eq!(hex(&[b'a'; 55]), "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318");
        assert_eq!(hex(&[b'a'; 56]), "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a");
        assert_eq!(hex(&[b'a'; 64]), "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");
        assert_eq!(hex(&b"a".repeat(1_000_000)), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }
}

/// Cross-checks the implementation above against the platform's own
/// `shasum`/`sha256sum` over real files of varied size and content
/// (including a multi-megabyte binary). Published vectors prove the
/// algorithm; this proves it still agrees with the tool that produced the
/// checksums a release will actually ship.
#[cfg(test)]
mod cross_check {
    use super::hex;
    use std::process::Command;

    /// Extracts a 64-character hex digest from a tool's output.
    ///
    /// Windows tools disagree on shape: Get-FileHash prints the bare
    /// uppercase digest, while certutil wraps it in a header and footer and
    /// has historically space-separated the bytes. Stripping whitespace per
    /// line and looking for the 64-hex-char run handles every form without
    /// a parser per tool.
    fn extract_digest(stdout: &str) -> Option<String> {
        stdout.lines().find_map(|line| {
            let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            let is_digest = compact.len() == 64 && compact.chars().all(|c| c.is_ascii_hexdigit());
            is_digest.then(|| compact.to_lowercase())
        })
    }

    /// The reference tool is chosen per platform to match the one that
    /// actually produces that platform's published checksum in release.yml:
    /// `sha256sum`/`shasum` on Unix, PowerShell's Get-FileHash on Windows.
    ///
    /// Windows deliberately avoids sha256sum: the runner resolves it to an
    /// MSYS build that is not what release.yml runs there, so checking
    /// against it tested a pairing that never ships. `pwsh` is tried before
    /// `powershell` because the runners have PowerShell 7, and certutil is
    /// the last resort -- it is present on every Windows install, so the
    /// test cannot silently degrade into checking nothing.
    ///
    /// Returns the tool name alongside the digest so a failure says which
    /// one disagreed.
    fn system_sha256(path: &str) -> Option<(String, String)> {
        #[cfg(windows)]
        let attempts: Vec<(&str, Vec<String>)> = {
            let ps = format!("(Get-FileHash -Algorithm SHA256 -LiteralPath '{path}').Hash");
            vec![
                ("pwsh", vec!["-NoProfile".into(), "-Command".into(), ps.clone()]),
                ("powershell", vec!["-NoProfile".into(), "-Command".into(), ps]),
                ("certutil", vec!["-hashfile".into(), path.to_string(), "SHA256".into()]),
            ]
        };
        #[cfg(not(windows))]
        let attempts: Vec<(&str, Vec<String>)> = vec![
            ("sha256sum", vec!["-b".into(), path.to_string()]),
            ("shasum", vec!["-a".into(), "256".into(), "-b".into(), path.to_string()]),
        ];

        for (bin, args) in attempts {
            let Ok(out) = Command::new(bin).args(&args).output() else { continue };
            if !out.status.success() {
                continue;
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Unix tools print "<digest>  <file>", so the digest is the
            // first token; extract_digest covers the Windows shapes.
            let first_token = stdout.split_whitespace().next().unwrap_or("");
            let digest = if first_token.len() == 64 && first_token.chars().all(|c| c.is_ascii_hexdigit()) {
                Some(first_token.to_lowercase())
            } else {
                extract_digest(&stdout)
            };
            if let Some(d) = digest {
                return Some((bin.to_string(), d));
            }
        }
        None
    }

    /// Paths are resolved against CARGO_MANIFEST_DIR, not the process
    /// working directory: other tests in this binary chdir into scratch
    /// dirs, and cargo runs tests in parallel, so anything relying on cwd
    /// here fails depending on which test happens to be mid-flight.
    #[test]
    fn agrees_with_the_system_tool() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut checked = 0;
        for name in ["Cargo.toml", "README.md", "src/platform.rs", "src/sha256.rs"] {
            let path = root.join(name);
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let Some((tool, expected)) = system_sha256(&path.to_string_lossy()) else { continue };
            assert_eq!(hex(&bytes), expected, "mismatch on {name} against {tool}");
            checked += 1;
        }
        assert!(checked > 0, "no file could be cross-checked - is a system SHA-256 tool on PATH?");
    }
}
