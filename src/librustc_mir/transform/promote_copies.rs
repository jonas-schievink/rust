//! Promotes `Operand::Copy` to `Operand::Move` when the copied value is no longer alive after the
//! copy.

use crate::dataflow::{Analysis, MaybeBorrowedLocals, Results};
use crate::transform::{MirPass, MirSource};
use crate::util::liveness::{liveness_of_locals, LivenessResult};
use rustc_middle::mir::{BodyAndCache, Location, Operand, visit::MutVisitor};
use rustc_middle::ty::TyCtxt;

pub struct PromoteCopies;

impl<'tcx> MirPass<'tcx> for PromoteCopies {
    fn run_pass(&self, tcx: TyCtxt<'tcx>, source: MirSource<'tcx>, body: &mut BodyAndCache<'tcx>) {
        // Since moving from a place invalidates all references to it, we need to ensure it is not
        // borrowed when doing the promotion.
        // Of course, the place must also not be live after the use, so we need to run liveness.

        let liveness = liveness_of_locals(read_only!(body));
        let borrow_result = MaybeBorrowedLocals::all_borrows()
            .into_engine(tcx, body, source.def_id())
            .iterate_to_fixpoint();
        unimplemented!()
    }
}

struct Visitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    liveness: LivenessResult,
    borrows: Results<'tcx, MaybeBorrowedLocals>,
}

impl<'tcx> MutVisitor<'tcx> for Visitor<'tcx> {
    fn tcx<'a>(&'a self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn visit_operand(&mut self, operand: &mut Operand<'tcx>, location: Location) {
        if let Operand::Copy(place) = operand {
            if place.is_indirect() {
                return;
            }

            let local = place.local;

            // FIXME: There's no way to access liveness results inside basic blocks until the
            // liveness pass uses the dataflow framework (and that has backwards dataflow support)
        }
    }
}
