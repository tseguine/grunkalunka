//! Conservative allocation effects shared by source member-kind proofs.
use super::*;

pub(super) enum Mutation {
    IndexDelete(NodeId),
}

pub(super) fn call(call: &CallExpression<'_>) -> Option<Mutation> {
    let (operation, receiver) = array_mutations::operation(call)?;
    match operation {
        array_mutations::Operation::Pop => Some(Mutation::IndexDelete(receiver.node_id())),
    }
}

pub(super) fn array_index(key: &[u16]) -> bool {
    if key.is_empty() || key.len() > 10 || (key.len() > 1 && key[0] == u16::from(b'0')) {
        return false;
    }
    let mut value = 0_u64;
    for unit in key {
        let Some(digit) = unit
            .checked_sub(u16::from(b'0'))
            .filter(|digit| *digit < 10)
        else {
            return false;
        };
        value = value * 10 + u64::from(digit);
    }
    value < u64::from(u32::MAX)
}
