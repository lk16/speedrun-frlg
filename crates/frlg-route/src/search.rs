//! Try several ways of doing the same thing and keep the best one.
//!
//! This is the difference between a route that happens to work and a route that
//! is *manipulated*. The rival battle is the case that forced it into
//! existence: mashing A through it wins or loses depending on nothing but which
//! frame the mash starts on, so the route searches for a start that wins
//! instead of accepting the roll it is handed (`docs/rival-1/route.md`).

use crate::record::{Recorder, RouteError, Trial};

/// What a candidate cost, and whether it did what was asked. Deliberately not
/// the masks themselves: reporting is per-variant and the masks run to
/// thousands of frames, so handing them out means copying every one of them to
/// print a count.
pub struct Attempt {
    pub frames: usize,
    pub ok: bool,
}

/// Run `attempt` for each variant, from the recorder's current state, and
/// commit the shortest one that succeeded.
///
/// Every trial starts from the same savestate, so the variants are genuinely
/// comparable, and the search costs the route nothing: only the winning masks
/// end up in the log. Returns the winning variant and its frame cost.
///
/// `report` sees every trial, which is how a search stops being a black box --
/// "12 tried, 6 won, cheapest was 3451" is the sort of thing that belongs in
/// the build output rather than in a comment.
pub fn best_of<V: Copy>(
    rec: &mut Recorder,
    variants: impl IntoIterator<Item = V>,
    mut attempt: impl FnMut(&mut Trial, V) -> Result<bool, RouteError>,
    mut report: impl FnMut(V, &Attempt),
) -> Result<(V, usize), RouteError> {
    let start = rec.save_state()?;
    let mut best: Option<(V, Vec<u16>)> = None;

    for variant in variants {
        rec.emu().load_state(&start)?;
        let mut trial = Trial::new(rec.emu());
        // A trial that times out is an answer -- this variant does not work --
        // not a reason to abandon the search.
        let ok = match attempt(&mut trial, variant) {
            Ok(ok) => ok,
            Err(RouteError::Timeout { .. }) => false,
            Err(other) => return Err(other),
        };
        let inputs = trial.into_inputs();
        report(
            variant,
            &Attempt {
                frames: inputs.len(),
                ok,
            },
        );
        if ok
            && best
                .as_ref()
                .is_none_or(|(_, seen)| inputs.len() < seen.len())
        {
            best = Some((variant, inputs));
        }
    }

    let (variant, inputs) = best.ok_or_else(|| RouteError::Timeout {
        what: "any variant to succeed".to_string(),
        budget: 0,
        frames: rec.frames(),
    })?;
    rec.emu().load_state(&start)?;
    let frames = inputs.len();
    rec.play(&inputs)?;
    Ok((variant, frames))
}
