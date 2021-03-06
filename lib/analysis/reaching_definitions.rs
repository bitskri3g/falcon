use analysis::fixed_point;
use error::*;
use il;
use std::collections::{BTreeMap, BTreeSet};


struct ReachingDefinitions {}


#[allow(dead_code)]
/// Compute reaching definitions for the given function.
pub fn reaching_definitions<'r>(function: &'r il::Function)
-> Result<BTreeMap<il::RefProgramLocation<'r>, BTreeSet<il::RefProgramLocation<'r>>>> {
    fixed_point::fixed_point_forward(ReachingDefinitions{}, function)
}


impl<'r> fixed_point::FixedPointAnalysis<'r, BTreeSet<il::RefProgramLocation<'r>>> for ReachingDefinitions {
    fn trans(
        &self,
        location: il::RefProgramLocation<'r>,
        state: Option<BTreeSet<il::RefProgramLocation<'r>>>
    ) -> Result<BTreeSet<il::RefProgramLocation<'r>>> {

        let mut state = match state {
            Some(state) => state,
            None => BTreeSet::new()
        };

        match *location.function_location() {
            il::RefFunctionLocation::Instruction(_, ref instruction) => {
                let mut kill = Vec::new();
                if let Some(variable) = instruction.operation().variable_written() {
                    for location in &state {
                        if location.instruction()
                                   .unwrap()
                                   .operation()
                                   .variable_written()
                                   .unwrap()
                                   .multi_var_clone() == variable.multi_var_clone() {
                            kill.push(location.clone());
                        }
                    }
                    for k in kill {
                        state.remove(&k);
                    }
                    state.insert(location.clone());
                }
            },
            il::RefFunctionLocation::EmptyBlock(_) |
            il::RefFunctionLocation::Edge(_) => {}
        }

        Ok(state)
    }


    fn join(
        &self,
        mut state0: BTreeSet<il::RefProgramLocation<'r>>,
        state1: &BTreeSet<il::RefProgramLocation<'r>>
    ) -> Result<BTreeSet<il::RefProgramLocation<'r>>> {
        for state in state1 {
            state0.insert(state.clone());
        }
        Ok(state0)
    }
}


#[test]
fn reaching_definitions_test() {
    /*
    a = in
    b = 4
    if a < 10 {
        c = a
        [0xdeadbeef] = c
    }
    else {
        c = b
    }
    b = c
    c = [0xdeadbeef]
    */
    let mut control_flow_graph = il::ControlFlowGraph::new();

    let head_index = {
        let block = control_flow_graph.new_block().unwrap();

        block.assign(il::scalar("a", 32), il::expr_scalar("in", 32));
        block.assign(il::scalar("b", 32), il::expr_const(4, 32));

        block.index()
    };

    let gt_index = {
        let block = control_flow_graph.new_block().unwrap();

        block.assign(il::scalar("c", 32), il::expr_scalar("b", 32));

        block.index()
    };

    let lt_index = {
        let block = control_flow_graph.new_block().unwrap();

        block.assign(il::scalar("c", 32), il::expr_scalar("a", 32));
        block.store(il::array("mem", 1 << 32), il::expr_const(0xdeadbeef, 32), il::expr_scalar("c", 32));

        block.index()
    };

    let tail_index = {
        let block = control_flow_graph.new_block().unwrap();

        block.assign(il::scalar("b", 32), il::expr_scalar("c", 32));
        block.load(il::scalar("c", 32), il::expr_const(0xdeadbeef, 32), il::array("mem", 1 << 32));

        block.index()
    };

    let condition = il::Expression::cmpltu(
        il::expr_scalar("a", 32),
        il::expr_const(10, 32)
    ).unwrap();

    control_flow_graph.conditional_edge(head_index, lt_index, condition.clone()).unwrap();
    control_flow_graph.conditional_edge(head_index, gt_index, 
        il::Expression::cmpeq(condition, il::expr_const(0, 1)).unwrap()
    ).unwrap();

    control_flow_graph.unconditional_edge(lt_index, tail_index).unwrap();
    control_flow_graph.unconditional_edge(gt_index, tail_index).unwrap();

    let function = il::Function::new(0, control_flow_graph);

    let rd = reaching_definitions(&function).unwrap();

    // for r in rd.iter() {
    //     println!("{}", r.0);
    //     for d in r.1 {
    //         println!("  {}", d);
    //     }
    // }

    let block = function.control_flow_graph().block(3).unwrap();
    let instruction = block.instruction(0).unwrap();

    let function_location = il::RefFunctionLocation::Instruction(block, instruction);
    let program_location = il::RefProgramLocation::new(&function, function_location);

    let r = &rd[&program_location];

    let block = function.control_flow_graph().block(0).unwrap();
    assert!(r.contains(&il::RefProgramLocation::new(&function,
        il::RefFunctionLocation::Instruction(
            block,
            block.instruction(0).unwrap()
        )
    )));

    let block = function.control_flow_graph().block(1).unwrap();
    assert!(r.contains(&il::RefProgramLocation::new(&function,
        il::RefFunctionLocation::Instruction(
            block,
            block.instruction(0).unwrap()
        )
    )));

    let block = function.control_flow_graph().block(2).unwrap();
    assert!(r.contains(&il::RefProgramLocation::new(&function,
        il::RefFunctionLocation::Instruction(
            block,
            block.instruction(0).unwrap()
        )
    )));

    let block = function.control_flow_graph().block(2).unwrap();
    assert!(r.contains(&il::RefProgramLocation::new(&function,
        il::RefFunctionLocation::Instruction(
            block,
            block.instruction(1).unwrap()
        )
    )));

    let block = function.control_flow_graph().block(3).unwrap();
    assert!(r.contains(&il::RefProgramLocation::new(&function,
        il::RefFunctionLocation::Instruction(
            block,
            block.instruction(0).unwrap()
        )
    )));

    let block = function.control_flow_graph().block(0).unwrap();
    assert!(!r.contains(&il::RefProgramLocation::new(&function,
        il::RefFunctionLocation::Instruction(
            block,
            block.instruction(1).unwrap()
        )
    )));
}