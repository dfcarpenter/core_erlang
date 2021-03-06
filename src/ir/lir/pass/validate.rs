use ::std::collections::HashSet;
use ::ir::lir::{ FunctionCfg, Source };

pub fn validate(cfg: &FunctionCfg) {

    validate_proper_ssa(cfg);

}

fn validate_proper_ssa(cfg: &FunctionCfg) {

    let mut assigns = HashSet::new();

    for block in cfg.blocks_iter() {
        for phi in block.phi_nodes.iter() {
            if assigns.contains(&phi.ssa) {
                println!("Double assign of {:?}", phi.ssa);
            }
            assigns.insert(phi.ssa);
        }

        for op in block.ops.iter() {
            for write in op.writes.iter() {
                if assigns.contains(write) {
                    println!("Double assign of {:?}", write);
                }
                assigns.insert(*write);
            }
        }
    }

    for block in cfg.blocks_iter() {
        for phi in block.phi_nodes.iter() {
            for &(_label, ssa) in phi.entries.iter() {
                if !assigns.contains(&ssa) {
                    println!("Use of unassigned {:?}", ssa);
                }
            }
        }

        for op in block.ops.iter() {
            for read in op.reads.iter() {
                if let Source::Variable(ref ssa) = *read {
                    if !assigns.contains(&ssa) {
                        println!("Use of unassigned {:?}", ssa);
                    }
                }
            }
        }
    }

}
