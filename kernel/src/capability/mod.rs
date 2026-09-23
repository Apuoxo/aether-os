//! Capability handles and rights primitives.
//!
//! This layer deliberately contains no global object table yet.  It provides
//! the kernel-side handle/rights invariants that syscall code can use without
//! treating a numeric capability as proof of authority.

pub const CAP_READ: u32 = 1 << 0;
pub const CAP_WRITE: u32 = 1 << 1;
pub const CAP_EXEC: u32 = 1 << 2;
pub const CAP_MAP: u32 = 1 << 3;
pub const CAP_GRANT: u32 = 1 << 4;
pub const CAP_ADMIN: u32 = 1 << 5;

#[derive(Clone, Copy)]
pub struct Cap {
    /// Stable object/handle identifier. Zero is always the null capability.
    pub id: u64,
    /// Rights are a subset of the rights originally granted to this handle.
    pub rights: u32,
}

impl Cap {
    pub const fn null() -> Self {
        Cap { id: 0, rights: 0 }
    }

    pub const fn new(id: u64, rights: u32) -> Self {
        Cap { id, rights }
    }

    pub fn is_valid(&self) -> bool {
        self.id != 0
    }

    pub fn permits(&self, required: u32) -> bool {
        self.is_valid() && (self.rights & required) == required
    }

    /// Derive a weaker handle; rights can never be increased by derivation.
    pub fn derive(&self, requested: u32) -> Option<Cap> {
        if !self.is_valid() || (requested & !self.rights) != 0 {
            None
        } else {
            Some(Cap::new(self.id, requested))
        }
    }
}

#[derive(Clone, Copy)]
pub struct CapTable {
    pub entries: [Cap; 16],
    pub used: usize,
}

impl CapTable {
    pub const fn new() -> Self {
        CapTable {
            entries: [Cap::null(); 16],
            used: 0,
        }
    }

    /// Install a capability and return its local slot/handle.
    pub fn insert(&mut self, cap: Cap) -> Option<usize> {
        if !cap.is_valid() { return None; }
        let mut i = 0usize;
        while i < self.entries.len() {
            if !self.entries[i].is_valid() {
                self.entries[i] = cap;
                self.used += 1;
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn get(&self, slot: usize) -> Option<Cap> {
        if slot >= self.entries.len() { return None; }
        let cap = self.entries[slot];
        if cap.is_valid() { Some(cap) } else { None }
    }

    pub fn check(&self, slot: usize, required: u32) -> bool {
        self.get(slot).map(|c| c.permits(required)).unwrap_or(false)
    }

    /// Revoke the local handle. Future lookups of this slot fail.
    pub fn revoke(&mut self, slot: usize) -> bool {
        if slot >= self.entries.len() || !self.entries[slot].is_valid() {
            return false;
        }
        self.entries[slot] = Cap::null();
        if self.used > 0 { self.used -= 1; }
        true
    }
}


/// Deterministic boot-time regression for capability rights invariants.
/// This validates the handle layer itself; syscall allow/deny runtime proof
/// remains a separate test because it must execute through the Ring3 path.
pub fn self_test() -> bool {
    let mut t = CapTable::new();
    let slot = match t.insert(Cap::new(0x100, CAP_READ | CAP_WRITE)) { Some(s) => s, None => return false };
    if !t.check(slot, CAP_READ) || !t.check(slot, CAP_WRITE) { return false; }
    if t.check(slot, CAP_MAP) { return false; }
    if t.get(slot).and_then(|c| c.derive(CAP_READ | CAP_MAP)).is_some() { return false; }
    if t.get(slot).and_then(|c| c.derive(CAP_READ)).is_none() { return false; }
    if !t.revoke(slot) || t.check(slot, CAP_READ) || t.get(slot).is_some() { return false; }
    true
}
