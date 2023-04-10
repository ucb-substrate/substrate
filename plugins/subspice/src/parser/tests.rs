use std::path::PathBuf;

use crate::parse;
use crate::parser::SubcktLine;

pub(crate) const EXAMPLES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

const SPICE_RESISTOR: &str = r#"
.subckt my_resistor p n
R1 p n 100
.ends
"#;

#[test]
fn test_spice_resistor() {
    let parsed = parse(&SPICE_RESISTOR).unwrap();
    assert_eq!(parsed.subcircuits().count(), 1);
    assert_eq!(
        parsed.subcircuits().next().unwrap(),
        &SubcktLine {
            name: "my_resistor",
            ports: vec!["p", "n"],
        }
    );
}

#[test]
fn test_dff() {
    let path = PathBuf::from(EXAMPLES_PATH).join("dff.spice");
    let data = std::fs::read_to_string(path).unwrap();
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.subcircuits().count(), 1);
    assert_eq!(
        parsed.subcircuits().next().unwrap(),
        &SubcktLine {
            name: "openram_dff",
            ports: vec!["VDD", "GND", "CLK", "D", "Q", "Q_N"],
        }
    );
}

#[test]
fn test_sense_amp() {
    let path = PathBuf::from(EXAMPLES_PATH).join("sense_amp.spice");
    let data = std::fs::read_to_string(path).unwrap();
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.subcircuits().count(), 1);
    assert_eq!(
        parsed.subcircuits().next().unwrap(),
        &SubcktLine {
            name: "AAA_Comp_SA_sense",
            ports: vec!["clk", "inn", "inp", "midn", "midp", "outn", "outp", "VDD", "VSS"],
        }
    );
}

#[test]
fn test_tmc() {
    let path = PathBuf::from(EXAMPLES_PATH).join("tmc.spice");
    let data = std::fs::read_to_string(path).unwrap();
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.subcircuits().count(), 2);
    assert_eq!(
        parsed.subcircuits().next().unwrap(),
        &SubcktLine {
            name: "dbdr_delay_unit_3",
            ports: vec!["clk_in", "clk_out", "sae_in", "sae_out", "clk_rev", "vdd", "vss"],
        }
    );
    assert_eq!(
        parsed.subcircuits().nth(1).unwrap(),
        &SubcktLine {
            name: "timing_multiplier_3",
            ports: vec!["clk", "sae_in", "sae_out", "vdd", "vss"],
        }
    );
}

#[test]
fn test_sram() {
    let path = PathBuf::from(EXAMPLES_PATH).join("sram.spice");
    let data = std::fs::read_to_string(path).unwrap();
    let parsed = parse(&data).unwrap();
    assert_eq!(parsed.subcircuits().count(), 29);
    assert_eq!(
        parsed
            .subcircuit_named("hierarchical_decoder_inv_3")
            .unwrap(),
        &SubcktLine {
            name: "hierarchical_decoder_inv_3",
            ports: vec!["gnd", "vdd", "din", "din_b"],
        }
    );
    assert_eq!(
        parsed
            .subcircuit_named("sramgen_sram_128x64m2w64_replica_v1")
            .unwrap(),
        &SubcktLine {
            name: "sramgen_sram_128x64m2w64_replica_v1",
            ports: vec![
                "vdd", "vss", "clk", "din[63]", "din[62]", "din[61]", "din[60]", "din[59]",
                "din[58]", "din[57]", "din[56]", "din[55]", "din[54]", "din[53]", "din[52]",
                "din[51]", "din[50]", "din[49]", "din[48]", "din[47]", "din[46]", "din[45]",
                "din[44]", "din[43]", "din[42]", "din[41]", "din[40]", "din[39]", "din[38]",
                "din[37]", "din[36]", "din[35]", "din[34]", "din[33]", "din[32]", "din[31]",
                "din[30]", "din[29]", "din[28]", "din[27]", "din[26]", "din[25]", "din[24]",
                "din[23]", "din[22]", "din[21]", "din[20]", "din[19]", "din[18]", "din[17]",
                "din[16]", "din[15]", "din[14]", "din[13]", "din[12]", "din[11]", "din[10]",
                "din[9]", "din[8]", "din[7]", "din[6]", "din[5]", "din[4]", "din[3]", "din[2]",
                "din[1]", "din[0]", "dout[63]", "dout[62]", "dout[61]", "dout[60]", "dout[59]",
                "dout[58]", "dout[57]", "dout[56]", "dout[55]", "dout[54]", "dout[53]", "dout[52]",
                "dout[51]", "dout[50]", "dout[49]", "dout[48]", "dout[47]", "dout[46]", "dout[45]",
                "dout[44]", "dout[43]", "dout[42]", "dout[41]", "dout[40]", "dout[39]", "dout[38]",
                "dout[37]", "dout[36]", "dout[35]", "dout[34]", "dout[33]", "dout[32]", "dout[31]",
                "dout[30]", "dout[29]", "dout[28]", "dout[27]", "dout[26]", "dout[25]", "dout[24]",
                "dout[23]", "dout[22]", "dout[21]", "dout[20]", "dout[19]", "dout[18]", "dout[17]",
                "dout[16]", "dout[15]", "dout[14]", "dout[13]", "dout[12]", "dout[11]", "dout[10]",
                "dout[9]", "dout[8]", "dout[7]", "dout[6]", "dout[5]", "dout[4]", "dout[3]",
                "dout[2]", "dout[1]", "dout[0]", "we", "addr[6]", "addr[5]", "addr[4]", "addr[3]",
                "addr[2]", "addr[1]", "addr[0]",
            ],
        }
    );
}
