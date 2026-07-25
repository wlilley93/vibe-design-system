//! The preimage limb: a design value stored as a digest of itself.
//!
//! `no_stored_values` scans for realisations written AS realisations, and until
//! this module existed it declared, on every record it captured, that it did not
//! discharge this:
//!
//! > It does not enumerate the candidate space against the sha256, sha1 and md5
//! > digests harvested from the tree, so a design value stored as a digest of
//! > itself is NOT reached by this run and would pass it. That is the exact form
//! > the first token pin leaked in.
//!
//! That last sentence is why this module exists. VDS S-2(7) as originally drafted
//! REQUIRED a pin to carry `source_value_digest` and `target_value_digest`, and
//! called them "what keeps the pin a gate rather than a store". An adversarial
//! reading recovered all 52 values from a 26-row pin in 27 seconds on one core,
//! because a hex colour is a 24-bit domain and an unsalted digest over a 24-bit
//! domain is not one-way. The clause mandated the storing form it forbade, and
//! the guard specified to catch that would have certified it clean, because the
//! guard greps for colour literals and a digest is not a literal.
//!
//! So the guard has to do what the attacker did.
//!
//! # Why this is a search and not a lookup
//!
//! The obvious shape - build a table from candidate digest to candidate value,
//! then look each harvested digest up - needs roughly 67 million entries and two
//! to three gigabytes of resident memory, which is not a thing a gate may cost.
//!
//! It is also the wrong way round. The harvested set is small (a few hundred
//! digests in a real `.vds/`) and the candidate set is large, so the small set is
//! the one to hold. This module puts the harvested digests in a hash set and
//! STREAMS the candidate space past it, hashing each candidate and asking whether
//! the answer is one of the few it is looking for. Memory is O(harvested), and
//! the whole space costs one pass.
//!
//! The pass is about 67 million short SHA-256 computations, which is four or five
//! seconds on one core, so it is sharded across the available cores and lands in
//! well under a second. Sharding cannot change the result: every shard covers a
//! disjoint slice of one deterministic enumeration, findings are merged and
//! sorted, and `candidates_enumerated` is a function of the code alone. Two runs
//! on machines with different core counts produce byte-identical output, which
//! VDS S-7(2)(1) requires of anything a warrant cites.
//!
//! # Three algorithms, computed only when the record contains one
//!
//! VDS writes sha256 and nothing else, but the limb names sha256, sha1 and md5,
//! and closing two of three would leave the note claiming a discharge it had not
//! made. The cost is avoided rather than paid: an algorithm is only computed if
//! the harvest actually found a digest of that width, so a record containing only
//! sha256 pays for one pass and not three.
//!
//! # What it still does not reach, said plainly rather than glossed
//!
//! A SALTED digest, an HMAC, a digest of a value concatenated with anything, an
//! iterated or key-derived digest, and a digest of a value written in a form
//! outside the enumerated space - of which the largest named omission is the
//! eight-digit hex colour `#rrggbbaa`, a 2^32 domain that would add twenty
//! seconds to every run. Widening the enumeration without limit is not possible,
//! and pretending otherwise would be exactly the overclaim about the enforcement
//! surface that VDS S-8(5) forbids. What this closes is the specific, measured,
//! demonstrated hole: the plain digest of a plain value.
//!
//! # A finding never carries the recovered value
//!
//! It names the class. A finding that printed `#ebebeb` would write that colour
//! into a proof record, which lands under the very tree this proof scans, and the
//! gate would then fail forever on a file it wrote itself - and it would be the
//! storing form, committed by the guard against the thing it guards. The reader
//! opens the named file at the named line, which holds the digest, and runs the
//! same recovery.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest as _, Sha256};

/// The three algorithms the limb names.
///
/// A closed enum rather than a string, so a fourth cannot be handled in one place
/// and forgotten in another: adding one stops the build everywhere it matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DigestAlgo {
    Md5,
    Sha1,
    Sha256,
}

impl DigestAlgo {
    /// The length in hexadecimal characters of a digest of this algorithm.
    pub const fn hex_len(self) -> usize {
        match self {
            DigestAlgo::Md5 => 32,
            DigestAlgo::Sha1 => 40,
            DigestAlgo::Sha256 => 64,
        }
    }

    /// The algorithm a hex run of this length could be, if any.
    pub const fn for_hex_len(len: usize) -> Option<DigestAlgo> {
        match len {
            32 => Some(DigestAlgo::Md5),
            40 => Some(DigestAlgo::Sha1),
            64 => Some(DigestAlgo::Sha256),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            DigestAlgo::Md5 => "md5",
            DigestAlgo::Sha1 => "sha1",
            DigestAlgo::Sha256 => "sha256",
        }
    }

    /// The digest of `value` under this algorithm.
    ///
    /// [`sweep`] does not call this: its inner loop runs 65 million times and
    /// cannot afford the heap allocation this returns, so it dispatches on the
    /// three algorithms directly with fixed-size arrays. The two are held in
    /// agreement by `the_limb_is_closed_for_all_three_algorithms_it_names`, which
    /// builds its digests HERE and asserts the sweep finds them, so a variant
    /// wired to the wrong hash in one place fails the test rather than silently
    /// searching for something nothing will ever produce.
    pub fn digest(self, value: &[u8]) -> Vec<u8> {
        match self {
            DigestAlgo::Md5 => Md5::digest(value).to_vec(),
            DigestAlgo::Sha1 => Sha1::digest(value).to_vec(),
            DigestAlgo::Sha256 => Sha256::digest(value).to_vec(),
        }
    }
}

/// One digest found in the record, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// Repository-relative path of the file it was found in.
    pub location: String,
    /// 1-based line.
    pub line: usize,
    /// 1-based character column of the first hex character.
    pub column: usize,
    pub algo: DigestAlgo,
    /// The digest itself, lowercased hex, without any `sha256:` prefix.
    ///
    /// Held because it is not a design value: it is what a reader needs in order
    /// to reproduce the recovery, and it is already in the file at the named
    /// line, so carrying it here adds nothing to the tree that was not there.
    pub hex: String,
}

/// A digest that turned out to be the digest of a design value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recovered {
    /// Index into the `sites` slice passed to [`sweep`].
    pub site: usize,
    /// What class of realisation the preimage is. Deliberately NOT the value.
    pub class: &'static str,
}

/// What one sweep did, reportable without any of it being a design value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sweep {
    /// How many candidate strings were enumerated. A function of the code alone,
    /// so it is identical on every machine and belongs in a captured record.
    pub candidates_enumerated: u64,
    /// Which algorithms were actually computed, in a stable order.
    pub algorithms: Vec<DigestAlgo>,
    /// How many harvested digests were tested.
    pub sites_tested: usize,
    /// How many distinct digest values those sites held. Lower than
    /// `sites_tested` when one digest appears in several places.
    pub distinct_digests: usize,
    /// Sorted by site index.
    pub recovered: Vec<Recovered>,
}

// ---------------------------------------------------------------------------
// Harvest
// ---------------------------------------------------------------------------

/// Every digest-shaped hex run in one file's text.
///
/// A run is a MAXIMAL sequence of hexadecimal characters, so a 64-character run
/// embedded in a 70-character one is not reported as a sha256: it is not one, and
/// chasing the substring would sweep for something nobody wrote.
///
/// The `sha256:` prefix VDS writes is not required. A bare run of the right width
/// is taken on its width alone, because the thing guarded against is somebody
/// storing a digest, and somebody storing a digest is under no obligation to
/// label it.
pub fn harvest(location: &str, text: &str) -> Vec<Site> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if !chars[i].is_ascii_hexdigit() {
                i += 1;
                continue;
            }
            let start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            let Some(algo) = DigestAlgo::for_hex_len(i - start) else {
                continue;
            };
            let hex: String = chars[start..i]
                .iter()
                .map(|c| c.to_ascii_lowercase())
                .collect();
            out.push(Site {
                location: location.to_owned(),
                line: index + 1,
                column: start + 1,
                algo,
                hex,
            });
        }
    }
    out
}

fn hex_to_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        out.push((high * 16 + low) as u8);
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Sweep
// ---------------------------------------------------------------------------

/// The digests of ONE algorithm that this sweep is looking for.
///
/// Two layers, because the inner loop runs about 65 million times per shard and
/// what happens per candidate is the whole cost of the gate.
///
/// The outer layer is a 64Kbit table indexed by the first two bytes of the
/// digest: one array index and one bit test, a nanosecond. It answers "could this
/// possibly be one of ours" and it is wrong in the safe direction only. With a
/// few hundred targets, it says no to better than 99% of candidates, and the
/// second layer never runs for them. Without it, a SipHash over a 32-byte key per
/// candidate costs about as much as the SHA-256 that produced the key, which
/// would double the runtime of the whole proof to answer a question the bit test
/// already answered.
struct Targets<const N: usize> {
    /// 2^16 bits. A hit means "maybe"; a miss means "definitely not".
    coarse: Box<[u64; 1024]>,
    exact: HashSet<[u8; N]>,
}

impl<const N: usize> Targets<N> {
    fn new(digests: impl Iterator<Item = [u8; N]>) -> Targets<N> {
        let mut coarse = Box::new([0u64; 1024]);
        let mut exact = HashSet::new();
        for digest in digests {
            let index = Self::coarse_index(&digest);
            coarse[index / 64] |= 1u64 << (index % 64);
            exact.insert(digest);
        }
        Targets { coarse, exact }
    }

    fn coarse_index(digest: &[u8; N]) -> usize {
        ((digest[0] as usize) << 8) | (digest[1] as usize)
    }

    fn contains(&self, digest: &[u8; N]) -> bool {
        let index = Self::coarse_index(digest);
        if self.coarse[index / 64] & (1u64 << (index % 64)) == 0 {
            return false;
        }
        self.exact.contains(digest)
    }
}

/// Test every harvested digest against the candidate space.
///
/// Returns what was SEARCHED as well as what was found, because a proof that
/// reports only its findings cannot be told apart from one that searched nothing.
pub fn sweep(sites: &[Site]) -> Sweep {
    // One entry per (algorithm, digest bytes), holding every site that carries
    // it. A digest repeated across ten records is searched for once.
    let mut wanted: HashMap<(DigestAlgo, Vec<u8>), Vec<usize>> = HashMap::new();
    for (index, site) in sites.iter().enumerate() {
        let Some(bytes) = hex_to_bytes(&site.hex) else {
            continue;
        };
        wanted.entry((site.algo, bytes)).or_default().push(index);
    }

    let mut algorithms: Vec<DigestAlgo> = wanted.keys().map(|(algo, _)| *algo).collect();
    algorithms.sort();
    algorithms.dedup();

    let distinct_digests = wanted.len();

    if wanted.is_empty() {
        return Sweep {
            candidates_enumerated: 0,
            algorithms,
            sites_tested: sites.len(),
            distinct_digests: 0,
            recovered: Vec::new(),
        };
    }

    // Only the widths present are looked for, so an algorithm nothing in the
    // record could be is not computed at all. `None` is the whole optimisation:
    // a record holding only sha256 runs one hash per candidate, not three.
    let fixed = |algo: DigestAlgo| -> Vec<&Vec<u8>> {
        wanted
            .keys()
            .filter(|(a, _)| *a == algo)
            .map(|(_, bytes)| bytes)
            .collect()
    };
    let build = |algo: DigestAlgo| -> Option<Vec<&Vec<u8>>> {
        let found = fixed(algo);
        (!found.is_empty()).then_some(found)
    };
    let md5_targets = build(DigestAlgo::Md5).map(|v| {
        Targets::<16>::new(v.into_iter().map(|b| {
            let mut out = [0u8; 16];
            out.copy_from_slice(b);
            out
        }))
    });
    let sha1_targets = build(DigestAlgo::Sha1).map(|v| {
        Targets::<20>::new(v.into_iter().map(|b| {
            let mut out = [0u8; 20];
            out.copy_from_slice(b);
            out
        }))
    });
    let sha256_targets = build(DigestAlgo::Sha256).map(|v| {
        Targets::<32>::new(v.into_iter().map(|b| {
            let mut out = [0u8; 32];
            out.copy_from_slice(b);
            out
        }))
    });

    let shards = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 32) as u32;

    let md5_ref = md5_targets.as_ref();
    let sha1_ref = sha1_targets.as_ref();
    let sha256_ref = sha256_targets.as_ref();

    let mut hits: Vec<(DigestAlgo, Vec<u8>, &'static str)> = Vec::new();
    let mut enumerated = 0u64;

    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..shards)
            .map(|shard| {
                scope.spawn(move || {
                    let mut local: Vec<(DigestAlgo, Vec<u8>, &'static str)> = Vec::new();
                    let count = enumerate_shard(shard, shards, &mut |value, class| {
                        let bytes = value.as_bytes();
                        if let Some(targets) = sha256_ref {
                            let digest: [u8; 32] = Sha256::digest(bytes).into();
                            if targets.contains(&digest) {
                                local.push((DigestAlgo::Sha256, digest.to_vec(), class));
                            }
                        }
                        if let Some(targets) = sha1_ref {
                            let digest: [u8; 20] = Sha1::digest(bytes).into();
                            if targets.contains(&digest) {
                                local.push((DigestAlgo::Sha1, digest.to_vec(), class));
                            }
                        }
                        if let Some(targets) = md5_ref {
                            let digest: [u8; 16] = Md5::digest(bytes).into();
                            if targets.contains(&digest) {
                                local.push((DigestAlgo::Md5, digest.to_vec(), class));
                            }
                        }
                    });
                    (count, local)
                })
            })
            .collect();
        for handle in handles {
            // A panicking shard would silently shrink the space searched, and a
            // sweep that searched less than it says it did is the failure this
            // whole module exists to close. Propagate rather than absorb.
            let (count, local) = handle.join().expect("a sweep shard panicked");
            enumerated += count;
            hits.extend(local);
        }
    });

    let mut recovered: Vec<Recovered> = Vec::new();
    for (algo, digest, class) in hits {
        if let Some(indices) = wanted.get(&(algo, digest)) {
            for index in indices {
                recovered.push(Recovered {
                    site: *index,
                    class,
                });
            }
        }
    }
    // Sorted and deduplicated so the shard count cannot reach the output.
    recovered.sort_by(|a, b| a.site.cmp(&b.site).then(a.class.cmp(b.class)));
    recovered.dedup();

    Sweep {
        candidates_enumerated: enumerated,
        algorithms,
        sites_tested: sites.len(),
        distinct_digests,
        recovered,
    }
}

// ---------------------------------------------------------------------------
// The candidate space
// ---------------------------------------------------------------------------

pub const CLASS_HEX_COLOUR: &str = "a hex colour";
pub const CLASS_NAMED_COLOUR: &str = "a named CSS colour";
pub const CLASS_LENGTH: &str = "a CSS length";
pub const CLASS_DURATION: &str = "a CSS duration";
pub const CLASS_EASING: &str = "an easing curve";
pub const CLASS_FONT: &str = "a generic font family";

/// What the space is, for the captured record. A reader must be able to tell
/// what a pass covered without reading this file.
pub const SPACE_NOTE: &str = "[space] the candidate space is every 24-bit hex colour in the four spellings a stylesheet \
     and a Figma export produce, every three- and four-digit hex colour, the 148 CSS named \
     colours, numbers carrying a CSS length unit from -200 to 2000 at whole, tenth and \
     hundredth precision, durations to ten seconds in milliseconds and to sixty seconds in \
     hundredths, the named easing keywords with the common cubic-beziers, and the generic font \
     families. It does NOT include the eight-digit hex colour with an alpha channel, whose 2^32 \
     domain would add twenty seconds to every run, nor an arbitrary cubic-bezier, whose textual \
     form is unbounded. It is a floor, and it is the floor at which the demonstrated leak sits.";

/// The CSS named colours, as NAMES only.
///
/// `vds_css::colour` holds seventeen of these WITH their RGB triples and says why
/// it stops there: a hand-entered table of 148 triples is a place for a wrong
/// digit to hide, and a wrong digit produces a confident wrong contrast ratio.
/// That reasoning does not apply here, because this list carries no values. An
/// entry misspelled or missing costs reach and can never cost correctness, since
/// a match here is an exact digest equality.
///
/// It is also why a class the literal limb cannot safely check appears in this
/// one. A rule that failed a record for containing the word "silver" would fire
/// on prose, and a gate that cries wolf gets switched off. A digest match cannot
/// fire on prose.
const NAMED_COLOURS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
];

/// Kept in step with `no_stored_values::LENGTH_UNITS` by a test in this module.
///
/// The two lists say the same thing for different reasons - one decides what the
/// literal limb catches, the other what the preimage limb sweeps - and a limb
/// that reached fewer units than the other would be a hole with no name.
const LENGTH_UNITS: &[&str] = &[
    "px", "rem", "em", "ex", "ch", "vh", "vw", "vmin", "vmax", "pt", "pc", "cm", "mm", "in",
];

const EASING: &[&str] = &[
    "ease",
    "ease-in",
    "ease-out",
    "ease-in-out",
    "linear",
    "step-start",
    "step-end",
    "cubic-bezier(0.4, 0, 0.2, 1)",
    "cubic-bezier(0.4,0,0.2,1)",
    "cubic-bezier(0, 0, 0.2, 1)",
    "cubic-bezier(0.4, 0, 1, 1)",
    "cubic-bezier(0.25, 0.1, 0.25, 1)",
    "cubic-bezier(0.42, 0, 0.58, 1)",
];

const GENERIC_FAMILIES: &[&str] = &[
    "serif",
    "sans-serif",
    "monospace",
    "cursive",
    "fantasy",
    "system-ui",
    "ui-serif",
    "ui-sans-serif",
    "ui-monospace",
    "ui-rounded",
    "math",
    "emoji",
    "fangsong",
];

/// The number of 24-bit colours, which is the whole reason a digest of one is not
/// a hiding place.
const COLOUR_DOMAIN: u32 = 0x0100_0000;

/// Whether this value's hexadecimal spelling contains a letter, and therefore has
/// two cases rather than one.
///
/// `#123456` is the same string in both cases, so emitting an "uppercase"
/// spelling of it would enumerate the same candidate twice: 6% of the colour
/// domain is all-digits. The waste is small; the double count is not, because
/// `candidates_enumerated` goes into a captured record and a number that
/// overstates the search by six percent is a number that lies by six percent.
const fn has_letter_nibble(value: u32, nibbles: u32) -> bool {
    let mut index = 0;
    while index < nibbles {
        if ((value >> (index * 4)) & 0xF) >= 10 {
            return true;
        }
        index += 1;
    }
    false
}

/// Enumerate one shard of the candidate space, calling `f` on each candidate.
///
/// Shard `n` of `shards` takes the 24-bit colour values congruent to `n`, and
/// shard 0 additionally takes every small group. The union over all shards is one
/// fixed set whatever `shards` is, which is what lets the shard count vary with
/// the machine without the result varying with it.
///
/// Returns the number of candidates it emitted.
fn enumerate_shard(shard: u32, shards: u32, f: &mut dyn FnMut(&str, &'static str)) -> u64 {
    let mut count = 0u64;
    let mut buffer = String::with_capacity(32);

    // Six-digit hex, in the four spellings a stylesheet and a Figma export
    // actually produce: with and without the sigil, lower and upper case. This is
    // the domain the 27-second recovery ran over, and it is 99% of the space.
    let mut value = shard;
    while value < COLOUR_DOMAIN {
        buffer.clear();
        let _ = write!(buffer, "#{value:06x}");
        f(&buffer, CLASS_HEX_COLOUR);
        f(&buffer[1..], CLASS_HEX_COLOUR);
        count += 2;
        if has_letter_nibble(value, 6) {
            buffer.clear();
            let _ = write!(buffer, "#{value:06X}");
            f(&buffer, CLASS_HEX_COLOUR);
            f(&buffer[1..], CLASS_HEX_COLOUR);
            count += 2;
        }
        value += shards;
    }

    if shard != 0 {
        return count;
    }

    let mut emit = |value: &str, class: &'static str| {
        f(value, class);
        count += 1;
    };

    // Three- and four-digit hex. A distinct string even where it names a colour
    // the six-digit sweep already covered, because a digest is over the string.
    for value in 0u32..=0xFFF {
        emit(&format!("#{value:03x}"), CLASS_HEX_COLOUR);
        if has_letter_nibble(value, 3) {
            emit(&format!("#{value:03X}"), CLASS_HEX_COLOUR);
        }
    }
    for value in 0u32..=0xFFFF {
        emit(&format!("#{value:04x}"), CLASS_HEX_COLOUR);
        if has_letter_nibble(value, 4) {
            emit(&format!("#{value:04X}"), CLASS_HEX_COLOUR);
        }
    }

    for name in NAMED_COLOURS {
        emit(name, CLASS_NAMED_COLOUR);
        let mut capitalised = String::with_capacity(name.len());
        let mut chars = name.chars();
        if let Some(first) = chars.next() {
            capitalised.extend(first.to_uppercase());
            capitalised.push_str(chars.as_str());
        }
        emit(&capitalised, CLASS_NAMED_COLOUR);
    }

    // Lengths. The range is what a design system plausibly writes: whole values
    // to 2000, tenths and hundredths to 20, and negatives to -200, since a
    // negative margin is the ordinary case rather than the exotic one.
    for unit in LENGTH_UNITS {
        for whole in 0u32..=2000 {
            emit(&format!("{whole}{unit}"), CLASS_LENGTH);
        }
        for tenths in 1u32..=2000 {
            if tenths.is_multiple_of(10) {
                continue;
            }
            emit(
                &format!("{}.{}{unit}", tenths / 10, tenths % 10),
                CLASS_LENGTH,
            );
        }
        for hundredths in 1u32..=2000 {
            if hundredths.is_multiple_of(10) {
                continue;
            }
            emit(
                &format!("{}.{:02}{unit}", hundredths / 100, hundredths % 100),
                CLASS_LENGTH,
            );
        }
        for whole in 1u32..=200 {
            emit(&format!("-{whole}{unit}"), CLASS_LENGTH);
        }
    }

    // Durations. Milliseconds to ten seconds, and seconds to a minute in
    // hundredths, which covers every transition and every animation delay.
    for milliseconds in 0u32..=10_000 {
        emit(&format!("{milliseconds}ms"), CLASS_DURATION);
    }
    for hundredths in 0u32..=6000 {
        if hundredths.is_multiple_of(100) {
            emit(&format!("{}s", hundredths / 100), CLASS_DURATION);
        } else if hundredths.is_multiple_of(10) {
            emit(
                &format!("{}.{}s", hundredths / 100, (hundredths % 100) / 10),
                CLASS_DURATION,
            );
        } else {
            emit(
                &format!("{}.{:02}s", hundredths / 100, hundredths % 100),
                CLASS_DURATION,
            );
        }
    }

    for value in EASING {
        emit(value, CLASS_EASING);
    }
    for value in GENERIC_FAMILIES {
        emit(value, CLASS_FONT);
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(algo: DigestAlgo, value: &str) -> Site {
        Site {
            location: "test".into(),
            line: 1,
            column: 1,
            algo,
            hex: hex::encode(algo.digest(value.as_bytes())),
        }
    }

    /// One sweep, many assertions.
    ///
    /// The space costs a fraction of a second and the cost is per SWEEP rather
    /// than per digest, so a test that swept once per case would be the slowest
    /// thing in the suite for no extra coverage. Every positive and every
    /// negative case goes into one call.
    #[test]
    fn the_recovery_that_broke_the_first_pin_is_the_recovery_this_gate_runs() {
        let positives: Vec<(&str, &str)> = vec![
            // The exact leak. VDS S-2(7) as drafted REQUIRED this field.
            ("#ebebeb", CLASS_HEX_COLOUR),
            ("ebebeb", CLASS_HEX_COLOUR),
            ("#EBEBEB", CLASS_HEX_COLOUR),
            ("EBEBEB", CLASS_HEX_COLOUR),
            ("#fff", CLASS_HEX_COLOUR),
            ("#FFF", CLASS_HEX_COLOUR),
            ("#0a0b", CLASS_HEX_COLOUR),
            ("cornflowerblue", CLASS_NAMED_COLOUR),
            ("Silver", CLASS_NAMED_COLOUR),
            ("12px", CLASS_LENGTH),
            ("1.5rem", CLASS_LENGTH),
            ("0.25rem", CLASS_LENGTH),
            ("-8px", CLASS_LENGTH),
            ("1920px", CLASS_LENGTH),
            ("100vh", CLASS_LENGTH),
            ("160ms", CLASS_DURATION),
            ("0.3s", CLASS_DURATION),
            ("0.15s", CLASS_DURATION),
            ("2s", CLASS_DURATION),
            ("ease-in-out", CLASS_EASING),
            ("cubic-bezier(0.4, 0, 0.2, 1)", CLASS_EASING),
            ("sans-serif", CLASS_FONT),
        ];

        // The direction that decides whether this gate survives contact with a
        // real record. Every one of these is a digest VDS itself writes or a
        // value a governance record legitimately holds, and a gate that fires on
        // a proof record's own inputs_digest is a gate somebody switches off.
        let negatives: Vec<&str> = vec![
            "app/dashboard/page.tsx",
            "CMP-0001",
            "PROOF-20260725-100000",
            "control-border",
            "WCAG 2.2 SC 1.4.11",
            "min_ratio: 3.0",
            "3.0",
            "registered",
            "",
            "{\"kind\":\"composition\",\"status\":\"passed\"}",
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "the quick brown fox jumps over the lazy dog",
            "#ebebebeb",
            "12 px",
            "12PX",
        ];

        let mut sites = Vec::new();
        for (value, _) in &positives {
            sites.push(site(DigestAlgo::Sha256, value));
        }
        let first_negative = sites.len();
        for value in &negatives {
            sites.push(site(DigestAlgo::Sha256, value));
        }

        let result = sweep(&sites);
        assert_eq!(result.sites_tested, sites.len());
        assert_eq!(result.algorithms, vec![DigestAlgo::Sha256]);

        let found: HashMap<usize, &'static str> =
            result.recovered.iter().map(|r| (r.site, r.class)).collect();

        for (index, (value, class)) in positives.iter().enumerate() {
            assert_eq!(
                found.get(&index),
                Some(class),
                "{value:?} was NOT recovered from its digest, so a record could store it in \
                 that form and pass this gate"
            );
        }
        for (offset, value) in negatives.iter().enumerate() {
            assert!(
                !found.contains_key(&(first_negative + offset)),
                "false positive: the digest of {value:?} was reported as a design value, and a \
                 gate that fires on a record's own digests is a gate somebody switches off"
            );
        }
    }

    #[test]
    fn the_limb_is_closed_for_all_three_algorithms_it_names() {
        let sites = vec![
            site(DigestAlgo::Md5, "#ebebeb"),
            site(DigestAlgo::Sha1, "#ebebeb"),
            site(DigestAlgo::Sha256, "#ebebeb"),
        ];
        let result = sweep(&sites);
        assert_eq!(
            result.algorithms,
            vec![DigestAlgo::Md5, DigestAlgo::Sha1, DigestAlgo::Sha256]
        );
        assert_eq!(result.recovered.len(), 3, "{:?}", result.recovered);
    }

    #[test]
    fn an_algorithm_the_record_cannot_contain_is_not_computed() {
        let result = sweep(&[site(DigestAlgo::Sha256, "#ebebeb")]);
        assert_eq!(
            result.algorithms,
            vec![DigestAlgo::Sha256],
            "a record holding only sha256 must not pay for three passes"
        );
    }

    #[test]
    fn a_record_with_no_digest_costs_nothing() {
        let result = sweep(&[]);
        assert_eq!(result.candidates_enumerated, 0);
        assert!(result.recovered.is_empty());
        assert_eq!(result.distinct_digests, 0);
    }

    /// The property that lets the shard count vary with the machine.
    #[test]
    fn the_space_is_the_same_size_however_it_is_sharded() {
        let counted = |shards: u32| -> u64 {
            (0..shards)
                .map(|shard| enumerate_shard(shard, shards, &mut |_, _| {}))
                .sum()
        };
        let one = counted(1);
        for shards in [2u32, 3, 5, 7, 16, 31] {
            assert_eq!(
                counted(shards),
                one,
                "sharding by {shards} changed the size of the space, so two machines would \
                 search different spaces and report the same number for it"
            );
        }
        assert!(
            (65_000_000..70_000_000).contains(&one),
            "the space is {one} candidates, which is not the roughly 65 million the module \
             documents. If it shrank, something stopped being enumerated."
        );
    }

    /// Sharding must PARTITION rather than overlap. An overlap would inflate the
    /// reported size while searching no more than before, which is the same lie
    /// as searching nothing.
    #[test]
    fn sharding_partitions_the_space_rather_than_overlapping_it() {
        let collect = |shards: u32, limit: usize| -> Vec<String> {
            let mut out = Vec::new();
            for shard in 0..shards {
                let mut seen = 0usize;
                enumerate_shard(shard, shards, &mut |value, class| {
                    if class == CLASS_HEX_COLOUR && value.starts_with('#') && value.len() == 7 {
                        seen += 1;
                        if seen <= limit {
                            out.push(value.to_owned());
                        }
                    }
                });
            }
            out.sort();
            out
        };
        let sharded = collect(4, 250);
        let mut deduped = sharded.clone();
        deduped.dedup();
        assert_eq!(
            sharded.len(),
            deduped.len(),
            "two shards emitted the same candidate, so the reported size overstates the search"
        );
    }

    #[test]
    fn a_digest_is_found_by_its_width_whether_or_not_it_is_labelled() {
        let digest = hex::encode(Sha256::digest(b"#ebebeb"));
        for text in [
            format!("source_value_digest: sha256:{digest}"),
            format!("x: {digest}"),
            format!("x: \"{}\"", digest.to_uppercase()),
        ] {
            let sites = harvest("f.yaml", &text);
            assert_eq!(sites.len(), 1, "{text}");
            assert_eq!(sites[0].algo, DigestAlgo::Sha256);
            assert_eq!(sites[0].hex, digest);
        }
    }

    #[test]
    fn a_hex_run_of_the_wrong_width_is_not_a_digest() {
        // Maximal-run matching: 64 hex characters inside a 70-character run is
        // not a sha256, and chasing the substring would sweep for something
        // nobody wrote.
        assert!(harvest("f", &"a".repeat(70)).is_empty());
        assert!(harvest("f", &"a".repeat(63)).is_empty());
        assert!(harvest("f", "#ebebeb").is_empty());
        assert_eq!(harvest("f", &"a".repeat(64)).len(), 1);
        assert_eq!(harvest("f", &"a".repeat(40))[0].algo, DigestAlgo::Sha1);
        assert_eq!(harvest("f", &"a".repeat(32))[0].algo, DigestAlgo::Md5);
    }

    #[test]
    fn a_site_records_where_a_reader_would_look() {
        let digest = hex::encode(Sha256::digest(b"#ebebeb"));
        let text = format!("line one\n  value: sha256:{digest}\n");
        let sites = harvest("a/b.yaml", &text);
        assert_eq!(sites[0].line, 2);
        assert_eq!(
            sites[0].column, 17,
            "the column of the first hex character. Note that `sha256` itself ends in a hex run \
             (`a256`), which is four characters and therefore not a digest of any width."
        );
        assert_eq!(sites[0].location, "a/b.yaml");
    }

    #[test]
    fn one_digest_in_several_places_is_searched_for_once_and_reported_everywhere() {
        let one = site(DigestAlgo::Sha256, "#ebebeb");
        let sites = vec![one.clone(), one.clone(), one];
        let result = sweep(&sites);
        assert_eq!(result.sites_tested, 3);
        assert_eq!(result.distinct_digests, 1);
        assert_eq!(result.recovered.len(), 3, "every site must be reported");
    }

    #[test]
    fn a_finding_never_carries_the_recovered_value() {
        let result = sweep(&[site(DigestAlgo::Sha256, "#ebebeb")]);
        let rendered = format!("{:?}", result.recovered);
        assert!(
            !rendered.contains("ebebeb"),
            "the finding leaked the value it recovered, which would write that value into a \
             proof record under the tree this proof scans: {rendered}"
        );
    }

    #[test]
    fn the_two_limbs_reach_the_same_length_units() {
        let mut here: Vec<&str> = LENGTH_UNITS.to_vec();
        let mut there: Vec<&str> = crate::no_stored_values::LENGTH_UNITS.to_vec();
        here.sort();
        there.sort();
        assert_eq!(
            here, there,
            "the literal limb and the preimage limb cover different length units, so a \
             realisation is caught written one way and not the other"
        );
    }
}
