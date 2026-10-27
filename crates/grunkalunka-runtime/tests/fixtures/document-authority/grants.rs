//! Private in-memory policy fixture; not a public runtime grant API.
use std::collections::BTreeMap;
use std::sync::Arc;

use super::ReadError;

#[derive(Clone)]
pub struct Grant {
    owner: Arc<()>,
    rep: u32,
}

pub struct Documents {
    owner: Arc<()>,
    texts: Vec<String>,
    live: BTreeMap<u32, usize>,
    next: Option<u32>,
    pub allowed: bool,
    pub attempts: usize,
    pub reads: Vec<usize>,
}

impl Documents {
    pub fn new(texts: Vec<String>) -> Self {
        Self {
            owner: Arc::new(()),
            texts,
            live: BTreeMap::new(),
            next: Some(1024),
            allowed: false,
            attempts: 0,
            reads: Vec::new(),
        }
    }

    pub fn issue(&mut self, index: usize) -> Option<Grant> {
        self.texts.get(index)?;
        let rep = self.next?;
        self.next = rep.checked_add(1);
        assert!(self.live.insert(rep, index).is_none());
        Some(Grant {
            owner: self.owner.clone(),
            rep,
        })
    }

    // Authenticate the host token before constructing any canonical borrow.
    pub fn representation(&self, grant: &Grant) -> Option<u32> {
        Arc::ptr_eq(&self.owner, &grant.owner).then_some(grant.rep)
    }

    pub fn revoke(&mut self, grant: &Grant) -> bool {
        self.representation(grant)
            .and_then(|rep| self.live.remove(&rep))
            .is_some()
    }

    pub fn read(&mut self, rep: u32) -> Result<String, ReadError> {
        self.attempts += 1;
        if !self.allowed {
            return Err(ReadError::Denied);
        }
        let index = *self.live.get(&rep).ok_or(ReadError::UnknownGrant)?;
        let text = &self.texts[index];
        if text.len() > 4096 {
            return Err(ReadError::TooLarge);
        }
        self.reads.push(index);
        Ok(text.clone())
    }
}

#[test]
fn exhausted_grant_ids_never_wrap_and_invalid_issuance_preserves_capacity() {
    let mut documents = Documents::new(vec!["last".into()]);
    documents.allowed = true;
    documents.next = Some(u32::MAX);
    assert!(documents.issue(1).is_none());
    let last = documents.issue(0).unwrap();
    assert_eq!(documents.representation(&last), Some(u32::MAX));
    assert!(documents.issue(0).is_none());
    assert_eq!(documents.read(u32::MAX), Ok("last".into()));
    assert!(documents.revoke(&last));
    assert!(!documents.revoke(&last));
    assert!(documents.issue(0).is_none());
    assert_eq!(documents.read(u32::MAX), Err(ReadError::UnknownGrant));
    assert_eq!(documents.reads, vec![0]);
}
