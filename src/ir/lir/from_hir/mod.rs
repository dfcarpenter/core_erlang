use ::ir::{ Module, FunctionDefinition };
use ::ir::SSAVariable;
use ::ir::hir;
use ::ir::lir;
use ::ir::lir::Source;
use ::ir::hir::pass::ssa::ScopeTracker;

mod pattern;

pub fn do_lower(module: &mut Module, env: &mut ScopeTracker) {
    module.lower(env)
}

impl Module {
    fn lower(&mut self, env: &mut ScopeTracker) {
        for fun in &mut self.functions {
            fun.lower(env);
        }
    }
}

impl FunctionDefinition {
    fn lower(&mut self, env: &mut ScopeTracker) {
        let mut cfg = lir::FunctionCfg::new();

        {
            let mut builder = lir::cfg::FunctionCfgBuilder::new(&mut cfg);
            builder.basic_op(
                lir::OpKind::Arguments, vec![],
                self.hir_fun.args.iter().map(|a| a.ssa).collect());

            let ret = self.hir_fun.body.lower(&mut builder, env);
            builder.basic_op(
                lir::OpKind::ReturnOk,
                vec![lir::Source::Variable(ret)], vec![]);
        }

        self.lir_function = Some(cfg);
    }
}

//use ::pattern::cfg as pat_cfg;
//fn lower_pattern_cfg(
//    graph: &pat_cfg::PatternCfg, node_id: pat_cfg::CfgNodeIndex,
//    leaves: &Vec<lir::cfg::LabelN>,
//    lir: &mut lir::cfg::FunctionCfgBuilder) -> lir::LabelN
//{
//    let mut children = graph.graph.neighbors_directed(
//        node_id, ::petgraph::Direction::Outgoing);
//    let node = &graph.graph[node_id];
//    match *node {
//        pat_cfg::CfgNodeKind::Root => {
//            let child_idx = children.next().unwrap();
//            assert!(children.next().is_none());
//            lower_pattern_cfg(graph, child_idx, leaves, lir)
//        },
//        pat_cfg::CfgNodeKind::Match(ref variable) => {
//            let curr = lir.add_block();
//            lir.set_block(curr);
//            lir.basic_op(lir::OpKind::Comment(format!("match on: {:?}", variable)),
//                         vec![], vec![]);
//
//            let mut types = Vec::new();
//            for edge in graph.graph.edges_directed(
//                node_id, ::petgraph::Direction::Outgoing) {
//                let edge_weight = edge.weight();
//                println!("{:?}", edge_weight);
//                types.push(edge_weight.kind.clone());
//
//                use ::petgraph::visit::EdgeRef;
//                let child_idx = lower_pattern_cfg(graph, edge.target(), leaves, lir);
//
//                lir.add_jump(curr, child_idx);
//            }
//            // Nodes are traversed in reverse added order
//            types.reverse();
//
//            lir.set_block(curr);
//            lir.basic_op(lir::OpKind::Match {
//                types: types,
//            }, vec![], vec![]);
//
//            curr
//        },
//        pat_cfg::CfgNodeKind::Fail => {
//            assert!(children.count() == 0);
//            unimplemented!()
//        },
//        pat_cfg::CfgNodeKind::Leaf(num) => {
//            assert!(children.count() == 0);
//            leaves[num]
//        },
//    }
//
//    //for child_idx in children {
//    //}
//}

use self::hir::SingleExpressionKind as HSEK;
impl hir::SingleExpression {
    fn lower(&self, b: &mut lir::cfg::FunctionCfgBuilder,
             env: &mut ScopeTracker) -> ::ir::SSAVariable {
        match self.kind {
            HSEK::InterModuleCall { ref module, ref name, ref args } => {
                let mut reads_r = vec![module.lower(b, env), name.lower(b, env)];
                reads_r.extend(args.iter().map(|a| a.lower(b, env)));
                let reads: Vec<_> = reads_r.iter()
                    .map(|r| lir::Source::Variable(*r))
                    .collect();

                b.basic_op(lir::OpKind::Call, reads, vec![self.ssa]);
                let prev_block = b.get_block();

                let throw_block = b.add_block();
                b.set_block(throw_block);
                b.basic_op(lir::OpKind::ReturnThrow, vec![], vec![]);

                let resume_block = b.add_block();
                b.set_block(resume_block);

                b.add_jump(prev_block, resume_block);
                b.add_jump(prev_block, throw_block);

                self.ssa
            },
            HSEK::ApplyCall { ref fun, ref args } => {
                let mut reads_r = vec![fun.lower(b, env)];
                reads_r.extend(args.iter().map(|a| a.lower(b, env)));
                let reads: Vec<_> = reads_r.iter()
                    .map(|r| lir::Source::Variable(*r))
                    .collect();

                b.basic_op(lir::OpKind::Apply, reads, vec![self.ssa]);
                let prev_block = b.get_block();

                let throw_block = b.add_block();
                b.set_block(throw_block);
                b.basic_op(lir::OpKind::ReturnThrow, vec![], vec![]);

                let resume_block = b.add_block();
                b.set_block(resume_block);

                b.add_jump(prev_block, resume_block);
                b.add_jump(prev_block, throw_block);

                self.ssa
            },
            HSEK::Atomic(ref atomic) => {
                b.basic_op(
                    lir::OpKind::Move,
                    vec![lir::Source::Constant(atomic.clone())], vec![self.ssa]);
                self.ssa
            },
            HSEK::NamedFunction { ref name, is_lambda } => {
                if is_lambda {
                    self.ssa
                } else {
                    let n = ::ir::FunctionIdent {
                        name: name.var.name.clone(),
                        arity: name.var.arity,
                        lambda: None,
                    };
                    b.basic_op(
                        lir::OpKind::CaptureNamedFunction(n),
                        vec![], vec![self.ssa]);
                    self.ssa
                }
            }
            HSEK::Variable(_) => {
                self.ssa
            },
            HSEK::Let { ref val, ref body, .. } => {
                for v in val.values.iter() {
                    v.lower(b, env);
                }
                body.lower(b, env);
                self.ssa
            },
            HSEK::Try { ref body, ref then,
                        ref catch_vars, ref catch, .. } => {
                for expr in body.values.iter() {
                    expr.lower(b, env);
                }
                then.lower(b, env);
                then.ssa
            },
            // TODO
            HSEK::Case { ref val, ref clauses, ref values } => {

                //println!("aaaa: {:#?}", values);

                //::pattern::to_decision_tree(clauses);

                for v in &val.values {
                    v.lower(b, env);
                }

                for value in values {
                    value.lower(b, env);
                }
                let value_vars: Vec<_> = values.iter().map(|v| v.ssa).collect();

                let a: Vec<_> = val.values.iter().map(|v| v.ssa).collect();

                // TODO: This is currently called when lowering HIR to LIR. This means we 
                // completely ignore any optimizations that can be done due to present type 
                // information on inputs. This should be moved to a later pass on the LIR.
                //pattern::compile(&a, clauses, &value_vars, env);

                let from_label = b.get_block();
                let done_label = b.add_block();

                let leaves: Vec<_> = clauses.iter()
                    .map(|clause| {
                        let clause_label = b.add_block();
                        b.set_block(clause_label);

                        let clause_ret = clause.body.lower(b, env);
                        b.basic_op(lir::OpKind::Jump, vec![], vec![]);
                        let clause_done_label = b.get_block();
                        b.add_jump(clause_done_label, done_label);
                        b.add_phi(clause_done_label, clause_ret,
                                  done_label, self.ssa);
                        clause_label
                    }).collect();

                //let entry = lower_pattern_cfg(&cfg, cfg.get_entry(), &leaves, b);
                //b.add_jump(from_label, entry);

                b.set_block(from_label);
                b.basic_op(lir::OpKind::Case {
                    vars: val.values.iter().map(|v| v.ssa).collect(),
                    clauses: clauses.iter().map(|c| {
                        lir::Clause {
                            patterns: c.patterns.clone(),
                        }
                    }).collect(),
                    value_vars: value_vars,
                }, vec![], vec![]);
                for leaf in leaves.iter() {
                    b.add_jump(from_label, *leaf);
                }

                b.set_block(done_label);

                self.ssa
            },
            HSEK::Tuple(ref elems) => {
                for elem in elems.iter() {
                    elem.lower(b, env);
                }

                b.basic_op(lir::OpKind::MakeTuple,
                           elems.iter()
                           .map(|e| lir::Source::Variable(e.ssa))
                           .collect(),
                           vec![self.ssa]);

                self.ssa
            },
            HSEK::List{ ref head, ref tail } => {
                tail.lower(b, env);
                for elem in head.iter() {
                    elem.lower(b, env);
                }

                let mut reads = vec![lir::Source::Variable(tail.ssa)];
                reads.extend(head.iter()
                             .map(|v| lir::Source::Variable(v.ssa)));

                b.basic_op(lir::OpKind::MakeList,
                           reads, vec![self.ssa]);

                self.ssa
            },
            HSEK::Map(ref kv) => {
                let mut reads = Vec::new();
                for &(ref key, ref value) in kv.iter() {
                    key.lower(b, env);
                    value.lower(b, env);

                    reads.push(lir::Source::Variable(key.ssa));
                    reads.push(lir::Source::Variable(value.ssa));
                }

                b.basic_op(lir::OpKind::MakeMap,
                           reads, vec![self.ssa]);

                self.ssa
            },
            HSEK::PrimOp { ref name, ref args } => {
                for arg in args.iter() {
                    arg.lower(b, env);
                }
                b.basic_op(
                    lir::OpKind::PrimOp(name.clone()),
                    args.iter().map(|a| lir::Source::Variable(a.ssa)).collect(),
                    vec![]);
                self.ssa
            },
            HSEK::Do(ref d1, ref d2) => {
                for v in d1.values.iter() {
                    v.lower(b, env);
                }
                d2.lower(b, env);
                self.ssa
            },
            HSEK::Receive { .. } => {
                // TODO
                self.ssa
            },
            HSEK::BindClosure { ref closure, lambda_env, env_ssa } => {
                // TODO
                b.basic_op(
                    lir::OpKind::MakeClosureEnv {
                        env_idx: lambda_env.unwrap()
                    },
                    vec![], vec![env_ssa]);
                b.basic_op(
                    lir::OpKind::BindClosure {
                        ident: closure.ident.clone().unwrap(),
                    },
                    vec![Source::Variable(env_ssa)],
                    vec![self.ssa]);
                self.ssa
            },
            HSEK::BindClosures { ref closures, lambda_env, ref body, env_ssa } => {
                // TODO
                b.basic_op(
                    lir::OpKind::MakeClosureEnv {
                        env_idx: lambda_env.unwrap()
                    },
                    vec![], vec![env_ssa]);
                body.lower(b, env);
                self.ssa
            },
            ref s => panic!("Unhandled: {:?}", s),
        }
    }
}