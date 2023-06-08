use substrate::pdk::stdcell::{Function, StdCellData, StdCellDb, StdCellLibData};

use crate::Sky130Pdk;

impl Sky130Pdk {
    pub fn std_cells(&self) -> substrate::error::Result<StdCellDb> {
        let lib = "sky130_fd_sc_hd";
        let mut hd = StdCellLibData::new(lib);
        let cells = vec![
            ("and2", Function::And2, vec![0, 1, 2, 4]),
            ("and3", Function::And3, vec![1, 2, 4]),
            ("buf", Function::Buf, vec![1, 2, 4, 6, 8, 12, 16]),
            ("bufbuf", Function::Buf, vec![8, 16]),
            ("inv", Function::Inv, vec![1, 2, 4, 6, 8]),
            ("tap", Function::Tap, vec![1, 2]),
            ("mux2", Function::Mux2, vec![1, 2, 4, 8]),
            ("mux4", Function::Mux4, vec![1, 2, 4]),
            ("nand2", Function::Nand2, vec![1, 2, 4, 8]),
            ("nand3", Function::Nand3, vec![1, 2, 4]),
            ("nor2", Function::Nor2, vec![1, 2, 4, 8]),
            ("nor3", Function::Nor3, vec![1, 2, 4]),
            ("or2", Function::Or2, vec![0, 1, 2, 4]),
            ("or3", Function::Or3, vec![1, 2, 4]),
            ("xnor2", Function::Xnor2, vec![1, 2, 4]),
            ("xnor3", Function::Xnor3, vec![1, 2, 4]),
            ("xor2", Function::Xor2, vec![1, 2, 4]),
            ("xor3", Function::Xor3, vec![1, 2, 4]),
            ("diode", Function::Other("diode".to_string()), vec![2]),
            (
                "dfxtp",
                Function::Other("pos_ff".to_string()),
                vec![1, 2, 4],
            ),
            (
                "dfrtp",
                Function::Other("pos_ff".to_string()),
                vec![1, 2, 4],
            ),
        ];
        for (name, function, strengths) in cells {
            for strength in strengths {
                let cell = StdCellData::builder()
                    .name(arcstr::format!("{lib}__{name}_{strength}"))
                    .layout_source(self.pdk_root.join(format!(
                        "libraries/{lib}/latest/cells/{name}/{lib}__{name}_{strength}.gds"
                    )))
                    .schematic_source(self.pdk_root.join(format!(
                        "libraries/{lib}/latest/cells/{name}/{lib}__{name}_{strength}.spice"
                    )))
                    .function(function.clone())
                    .strength(strength)
                    .build()
                    .unwrap();
                hd.add_cell(cell);
            }
        }
        let mut db = StdCellDb::new();
        let key = db.add_lib(hd);
        db.set_default_lib(key);
        Ok(db)
    }
}
