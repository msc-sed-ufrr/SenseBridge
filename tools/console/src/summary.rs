//! Resumo acumulado dos ciclos desde o último boot.

use std::collections::BTreeMap;

use crate::record::Cycle;
use crate::view::{self, Style};

#[derive(Default)]
pub struct Summary {
    cycles: u64,
    states: BTreeMap<String, u64>,
    local_failures: BTreeMap<String, u64>,
    external_failures: BTreeMap<String, u64>,
    last: Option<Cycle>,
    stack_min_free: Option<u64>,
    cycle_max_us: u64,
}

impl Summary {
    pub fn add(&mut self, cycle: &Cycle) {
        self.cycles += 1;
        *self.states.entry(cycle.state.clone()).or_default() += 1;
        if let Err(failure) = &cycle.local {
            *self.local_failures.entry(failure.clone()).or_default() += 1;
        }
        if let Err(failure) = &cycle.external {
            *self.external_failures.entry(failure.clone()).or_default() += 1;
        }
        self.stack_min_free = Some(
            self.stack_min_free
                .map_or(cycle.stack_min_free, |min| min.min(cycle.stack_min_free)),
        );
        self.cycle_max_us = self.cycle_max_us.max(cycle.cycle_us);
        self.last = Some(cycle.clone());
    }

    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Uma linha: contagem por estado, falhas por tipo, memória e o ciclo mais longo.
    pub fn render(&self, style: Style) -> String {
        let mut parts = vec![format!("{} ciclos", self.cycles)];
        parts.extend(
            self.states
                .iter()
                .map(|(state, count)| style.state(state, &format!("{state} {count}"))),
        );
        for (label, failures) in [
            ("falhas locais", &self.local_failures),
            ("falhas externas", &self.external_failures),
        ] {
            if !failures.is_empty() {
                let list: Vec<String> = failures
                    .iter()
                    .map(|(failure, count)| format!("{failure} {count}"))
                    .collect();
                parts.push(style.red(&format!("{label}: {}", list.join(", "))));
            }
        }
        if let Some(last) = &self.last {
            parts.push(format!(
                "heap {} (mín {})",
                view::kilobytes(last.heap_free),
                view::kilobytes(last.heap_min_free)
            ));
        }
        if let Some(stack) = self.stack_min_free {
            parts.push(format!("pilha mín {}", view::kilobytes(stack)));
        }
        parts.push(format!("ciclo máx {}", view::duration(self.cycle_max_us)));
        style.cyan(&format!("── {}", parts.join(" · ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::Columns;

    #[test]
    fn counts_states_failures_and_extremes() {
        let columns = Columns::default();
        let mut summary = Summary::default();
        for row in [
            "0,21,Normal,27.0,41.0,,29.1,76.0,0,,,,21911,1460235,1482282,206832,201476,12436",
            "1,5005,DegradedLocal,,,NoResponse,29.1,76.0,4984,,Normal->DegradedLocal,false,\
             22132,,22254,206948,201476,12400",
        ] {
            summary.add(&columns.cycle(row).unwrap());
        }
        assert_eq!(summary.cycles(), 2);
        assert_eq!(
            summary.render(Style { color: false }),
            "── 2 ciclos · DegradedLocal 1 · Normal 1 · falhas locais: NoResponse 1 · \
             heap 202.1 KB (mín 196.8 KB) · pilha mín 12.1 KB · ciclo máx 1.48 s"
        );
    }
}
