//! Infers the required storage duration of `Local`s.
//!
//! This uses a combination of liveness and borrow dataflow analyses to compute the earliest and
//! latest point in the CFG where a local's storage may be required.

use super::{MirPass, MirSource};
use rustc_data_structures::graph::dominators::dominators;
use rustc_index::vec::IndexVec;
use rustc_middle::{
    mir::visit::Visitor,
    mir::{Body, BodyAndCache, Local, Location, Statement, StatementKind},
    ty::TyCtxt,
};
use smallvec::SmallVec;
use rustc_middle::mir::visit::PlaceContext;
use rustc_middle::mir::{PlaceElem, BasicBlockData};

pub struct InferStorage;

impl<'tcx> MirPass<'tcx> for InferStorage {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, _: MirSource<'tcx>, body: &mut BodyAndCache<'tcx>) {
        if tcx.sess.opts.debugging_opts.mir_opt_level <= 1 {
            return;
        }

        let old_storage = old_storage(body);

        /*

        Problem: The cleanup path can use locals, but those uses do not dominate the StorageDead
        (which will never be reached). The StorageDead must not be hoisted before the unwinding
        terminator though!

        Should we count the terminator as a use of any locals that may be access by the cleanup
        path?

        TODO:
        - ensure soundness: the inferred storage live location must dominate all uses of the local
        - inferred live location should be dominated by explicit location (if any)

        */

        let domtree = dominators(&read_only!(body));

        UseCollector {
            callback: |local, location: Location| {
                // Not really sure what to do when there's multiple live/dead statements, so I guess
                // we'll only check if there is only 1?
                // Note sure in which cases there can be multiple, but maybe if one dominates the
                // other we can deduplicate?
                let live = &old_storage[local].0;
                let dead = &old_storage[local].1;
                if live.len() == 1 {
                    assert!(live[0].dominates(location, &domtree), "StorageLive at {:?} does not dominate use at {:?}", live[0], location);
                }
                if dead.len() == 1 {
                    assert!(location.dominates(dead[0], &domtree), "use at {:?} does not dominate StorageDead at {:?}", location, dead[0]);
                }
            },
        }.visit_body(body);
    }
}

/// A visitor that invokes a callback when any local is used.
struct UseCollector<F> {
    callback: F,
}

impl<'tcx, F> Visitor<'tcx> for UseCollector<F>
where
    F: FnMut(Local, Location),
{
    fn visit_local(&mut self, local: &Local, context: PlaceContext, location: Location) {
        // This gets called on debuginfo, so check that the context is actually a use.
        if context.is_use() {
            (self.callback)(*local, location);
        }
    }

    fn visit_projection_elem(
        &mut self,
        _local: Local,
        _proj_base: &[PlaceElem<'tcx>],
        elem: &PlaceElem<'tcx>,
        _context: PlaceContext,
        location: Location,
    ) {
        if let PlaceElem::Index(local) = elem {
            (self.callback)(*local, location);
        }
    }

    fn visit_statement(&mut self, statement: &Statement<'tcx>, location: Location) {
        // Do not visit storage statements as those will be removed/inferred after merging.
        match &statement.kind {
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) => {}
            _ => self.super_statement(statement, location),
        }
    }
}

type StorageMap = IndexVec<Local, (SmallVec<[Location; 1]>, SmallVec<[Location; 1]>)>;

fn old_storage(body: &Body<'_>) -> StorageMap {
    struct OldStorage {
        storage: StorageMap,
    }

    impl Visitor<'_> for OldStorage {
        fn visit_statement(&mut self, statement: &Statement<'tcx>, location: Location) {
            match &statement.kind {
                StatementKind::StorageLive(local) => {
                    self.storage[*local].0.push(location);
                }
                StatementKind::StorageDead(local) => {
                    self.storage[*local].1.push(location);
                }
                _ => {}
            }
        }
    }

    let mut v = OldStorage {
        storage: IndexVec::from_elem_n((SmallVec::new(), SmallVec::new()), body.local_decls.len()),
    };
    v.visit_body(body);
    v.storage
}
