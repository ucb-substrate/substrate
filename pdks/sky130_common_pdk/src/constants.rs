pub const DIFF_POLY_EXTENSION: i64 = 250;
pub const GATE_LICON_SPACE: i64 = 55;
pub const LICON_WIDTH: i64 = 170;
pub const LICON_DIFF_ENCLOSURE: i64 = 40;
pub const LI1_WIDTH: i64 = 170;
pub const POLY_SPACE: i64 = 210;
pub const DIFF_NWELL_SPACE: i64 = 340;
pub const DIFF_NWELL_ENCLOSURE: i64 = 180;
pub const DIFF_SPACE: i64 = 270;
pub const DIFF_PSDM_ENCLOSURE: i64 = 125;
pub const DIFF_NSDM_ENCLOSURE: i64 = 125;
pub const POLY_DIFF_EXTENSION: i64 = 130;
pub const NPC_LICON_POLY_ENCLOSURE: i64 = 100;

const fn max(a: i64, b: i64) -> i64 {
    [a, b][(a < b) as usize]
}

pub const DIFF_EDGE_TO_GATE: i64 = max(
    DIFF_POLY_EXTENSION,
    GATE_LICON_SPACE + LICON_WIDTH + LICON_DIFF_ENCLOSURE,
);
pub const FINGER_SPACE: i64 = max(2 * GATE_LICON_SPACE + LI1_WIDTH, POLY_SPACE);
pub const DIFF_TO_OPPOSITE_DIFF: i64 = DIFF_NWELL_SPACE + DIFF_NWELL_ENCLOSURE;
