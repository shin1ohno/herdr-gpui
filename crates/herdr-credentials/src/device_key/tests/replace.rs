use super::*;
use crate::device_key::replace::{Slot, recover, replace};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    // keep-sorted start
    Create,
    Delete(Slot),
    Find(Slot),
    Retag(Slot, Slot),
    // keep-sorted end
}

#[derive(Default)]
struct State {
    active: Option<&'static str>,
    retired: Option<&'static str>,
    staged: Option<&'static str>,
    omit_create: bool,
    operations: Vec<Operation>,
    failures: Vec<(Operation, usize, Error)>,
}

impl State {
    fn slot(&mut self, slot: Slot) -> &mut Option<&'static str> {
        match slot {
            Slot::Active => &mut self.active,
            Slot::Retired => &mut self.retired,
            Slot::Staged => &mut self.staged,
        }
    }

    fn visit(&mut self, operation: Operation) -> Result<(), Error> {
        self.operations.push(operation);
        if let Some(index) = self.failures.iter().position(|(op, _, _)| *op == operation) {
            if self.failures[index].1 > 0 {
                self.failures[index].1 -= 1;
            } else {
                return Err(self.failures.remove(index).2);
            }
        }
        Ok(())
    }
}

struct Keychain(RefCell<State>);

impl Keychain {
    fn new(failures: Vec<(Operation, Error)>) -> Self {
        Self(RefCell::new(State {
            active: Some("old"),
            failures: failures
                .into_iter()
                .map(|(op, error)| (op, 0, error))
                .collect(),
            ..State::default()
        }))
    }

    fn find(&self, slot: Slot) -> Result<Option<&'static str>, Error> {
        let mut state = self.0.borrow_mut();
        state.visit(Operation::Find(slot))?;
        Ok(*state.slot(slot))
    }

    fn retag(&self, from: Slot, to: Slot) -> Result<(), Error> {
        let mut state = self.0.borrow_mut();
        state.visit(Operation::Retag(from, to))?;
        assert!(state.slot(to).is_none(), "destination tag must be empty");
        let key = state.slot(from).take();
        assert!(key.is_some(), "source tag must exist");
        *state.slot(to) = key;
        Ok(())
    }

    fn replace(&self) -> Result<&'static str, Error> {
        replace(
            |slot| self.find(slot),
            || {
                let mut state = self.0.borrow_mut();
                state.visit(Operation::Create)?;
                if !state.omit_create {
                    assert!(state.staged.replace("new").is_none());
                }
                Ok(())
            },
            |from, to| self.retag(from, to),
            |slot| {
                let mut state = self.0.borrow_mut();
                state.visit(Operation::Delete(slot))?;
                *state.slot(slot) = None;
                Ok(())
            },
        )
    }

    fn recover(&self) -> Result<Option<&'static str>, Error> {
        recover(|slot| self.find(slot), |from, to| self.retag(from, to))
    }
}

#[test]
fn replacement_is_read_back_before_the_old_key_moves_and_is_removed() {
    let keychain = Keychain::new(vec![]);
    assert_eq!(keychain.replace().unwrap(), "new");
    let state = keychain.0.borrow();
    assert_eq!(state.active, Some("new"));
    assert_eq!(state.retired, None);
    assert_eq!(state.staged, None);
    assert_eq!(
        state.operations,
        [
            Operation::Find(Slot::Active),
            Operation::Delete(Slot::Staged),
            Operation::Delete(Slot::Retired),
            Operation::Create,
            Operation::Find(Slot::Staged),
            Operation::Retag(Slot::Active, Slot::Retired),
            Operation::Retag(Slot::Staged, Slot::Active),
            Operation::Delete(Slot::Retired),
        ]
    );
}

#[test]
fn replacement_before_first_use_stores_one_new_key() {
    let keychain = Keychain::new(vec![]);
    keychain.0.borrow_mut().active = None;
    assert_eq!(keychain.replace().unwrap(), "new");
    let state = keychain.0.borrow();
    assert_eq!(state.active, Some("new"));
    assert_eq!(state.staged, None);
    assert_eq!(state.retired, None);
}

#[test]
fn failures_before_promotion_keep_the_old_key_and_return_the_typed_error() {
    for operation in [
        Operation::Find(Slot::Active),
        Operation::Delete(Slot::Staged),
        Operation::Delete(Slot::Retired),
        Operation::Create,
        Operation::Find(Slot::Staged),
        Operation::Retag(Slot::Active, Slot::Retired),
        Operation::Retag(Slot::Staged, Slot::Active),
    ] {
        let keychain = Keychain::new(vec![(operation, Error::Locked)]);
        assert!(
            matches!(keychain.replace(), Err(Error::Locked)),
            "{operation:?}"
        );
        assert_eq!(keychain.recover().unwrap(), Some("old"), "{operation:?}");
        assert_eq!(keychain.0.borrow().active, Some("old"));
    }
}

#[test]
fn an_unreadable_replacement_does_not_move_the_old_key() {
    let keychain = Keychain::new(vec![(Operation::Find(Slot::Staged), Error::PublicKey)]);
    assert!(matches!(keychain.replace(), Err(Error::PublicKey)));
    let state = keychain.0.borrow();
    assert_eq!(state.active, Some("old"));
    assert!(
        !state
            .operations
            .iter()
            .any(|op| matches!(op, Operation::Retag(..)))
    );
}

#[test]
fn a_missing_replacement_does_not_remove_the_old_key() {
    let keychain = Keychain::new(vec![]);
    keychain.0.borrow_mut().omit_create = true;
    assert!(matches!(keychain.replace(), Err(Error::Missing)));
    assert_eq!(keychain.0.borrow().active, Some("old"));
    assert_eq!(keychain.0.borrow().retired, None);
}

#[test]
fn a_locked_rollback_recovers_the_old_key_before_any_creation() {
    let keychain = Keychain::new(vec![
        (Operation::Retag(Slot::Staged, Slot::Active), Error::Locked),
        (Operation::Retag(Slot::Retired, Slot::Active), Error::Locked),
    ]);
    assert!(matches!(keychain.replace(), Err(Error::Locked)));
    {
        let mut state = keychain.0.borrow_mut();
        assert_eq!(state.active, None);
        assert_eq!(state.retired, Some("old"));
        assert_eq!(state.staged, Some("new"));
        state.operations.clear();
    }
    assert_eq!(keychain.recover().unwrap(), Some("old"));
    assert_eq!(
        keychain.0.borrow().operations,
        [
            Operation::Find(Slot::Active),
            Operation::Find(Slot::Retired),
            Operation::Retag(Slot::Retired, Slot::Active),
        ]
    );
}

#[test]
fn an_interrupted_first_replacement_recovers_the_stored_new_key() {
    let keychain = Keychain::new(vec![]);
    {
        let mut state = keychain.0.borrow_mut();
        state.active = None;
        state.staged = Some("new");
    }
    assert_eq!(keychain.recover().unwrap(), Some("new"));
    assert_eq!(keychain.0.borrow().active, Some("new"));
}

#[test]
fn an_interrupted_committed_replacement_keeps_the_new_key() {
    let keychain = Keychain::new(vec![]);
    {
        let mut state = keychain.0.borrow_mut();
        state.active = Some("new");
        state.retired = Some("old");
    }
    assert_eq!(keychain.recover().unwrap(), Some("new"));
    assert_eq!(
        keychain.0.borrow().operations,
        [Operation::Find(Slot::Active)]
    );
}

#[test]
fn a_lock_while_recovering_never_falls_through_to_another_key() {
    let keychain = Keychain::new(vec![(Operation::Find(Slot::Active), Error::Locked)]);
    keychain.0.borrow_mut().retired = Some("old");
    assert!(matches!(keychain.recover(), Err(Error::Locked)));
    assert_eq!(
        keychain.0.borrow().operations,
        [Operation::Find(Slot::Active)]
    );
}

#[test]
fn cleanup_failure_reports_the_committed_new_identity_and_is_retried() {
    let keychain = Keychain::new(vec![]);
    keychain
        .0
        .borrow_mut()
        .failures
        .push((Operation::Delete(Slot::Retired), 1, Error::Locked));
    assert_eq!(keychain.replace().unwrap(), "new");
    {
        let mut state = keychain.0.borrow_mut();
        assert_eq!(state.active, Some("new"));
        assert_eq!(state.retired, Some("old"));
        assert_eq!(state.staged, None);
        state.operations.clear();
    }
    assert_eq!(keychain.recover().unwrap(), Some("new"));
    assert_eq!(keychain.replace().unwrap(), "new");
    let state = keychain.0.borrow();
    assert_eq!(state.retired, None);
    let cleanup = state
        .operations
        .iter()
        .position(|op| *op == Operation::Delete(Slot::Retired))
        .unwrap();
    let create = state
        .operations
        .iter()
        .position(|op| *op == Operation::Create)
        .unwrap();
    assert!(cleanup < create);
}
