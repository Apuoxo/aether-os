//! Capability handles and rights primitives.
//!
//! The process bootstrap ABI still uses fixed capability slots 0/1/2.
//! External capability handles use a generation tag so a revoked handle
//! cannot become valid again merely because its slot is reused.

pub const CAP_READ: u32 = 1 << 0;
pub const CAP_WRITE: u32 = 1 << 1;
pub const CAP_EXEC: u32 = 1 << 2;
pub const CAP_MAP: u32 = 1 << 3;
pub const CAP_GRANT: u32 = 1 << 4;
pub const CAP_ADMIN: u32 = 1 << 5;

const SLOT_BITS: usize = 4;
const SLOT_MASK: usize = (1usize << SLOT_BITS) - 1;

#[derive(Clone, Copy)]
pub struct Cap {
    pub id: u64,
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
    generations: [u32; 16],
}

impl CapTable {
    pub const fn new() -> Self {
        CapTable {
            entries: [Cap::null(); 16],
            used: 0,
            generations: [1; 16],
        }
    }

    fn encode(slot: usize, generation: u32) -> usize {
        ((generation as usize) << SLOT_BITS) | slot
    }

    fn decode(handle: usize) -> (usize, u32) {
        (handle & SLOT_MASK, (handle >> SLOT_BITS) as u32)
    }

    /// Install a capability and return a generation-checked external handle.
    pub fn insert(&mut self, cap: Cap) -> Option<usize> {
        if !cap.is_valid() {
            return None;
        }
        let mut i = 0usize;
        while i < self.entries.len() {
            if !self.entries[i].is_valid() {
                let generation = self.generations[i];
                self.entries[i] = cap;
                self.used += 1;
                return Some(Self::encode(i, generation));
            }
            i += 1;
        }
        None
    }

    /// Resolve an external handle. A stale generation is rejected.
    pub fn get(&self, handle: usize) -> Option<Cap> {
        let (slot, generation) = Self::decode(handle);
        if slot >= self.entries.len() || generation == 0 {
            return None;
        }
        if self.generations[slot] != generation {
            return None;
        }
        let cap = self.entries[slot];
        if cap.is_valid() { Some(cap) } else { None }
    }

    pub fn check(&self, handle: usize, required: u32) -> bool {
        self.get(handle).map(|c| c.permits(required)).unwrap_or(false)
    }

    /// Bootstrap-only fixed-slot lookup. This is intentionally separate from
    /// external handles and is used by the current process ABI (0/1/2).
    pub fn check_slot(&self, slot: usize, required: u32) -> bool {
        if slot >= self.entries.len() {
            return false;
        }
        self.entries[slot].permits(required)
    }

    /// Revoke an external handle and advance its generation.
    pub fn revoke(&mut self, handle: usize) -> bool {
        let (slot, generation) = Self::decode(handle);
        if slot >= self.entries.len() || generation == 0
            || self.generations[slot] != generation
            || !self.entries[slot].is_valid() {
            return false;
        }
        self.entries[slot] = Cap::null();
        self.generations[slot] = self.generations[slot].wrapping_add(1);
        if self.generations[slot] == 0 {
            self.generations[slot] = 1;
        }
        if self.used > 0 {
            self.used -= 1;
        }
        true
    }
}

/// Deterministic regression for rights, derivation, revocation and stale
/// generation rejection.
pub fn self_test() -> bool {
    let mut t = CapTable::new();
    let h1 = match t.insert(Cap::new(0x100, CAP_READ | CAP_WRITE)) {
        Some(h) => h,
        None => return false,
    };
    if !t.check(h1, CAP_READ) || !t.check(h1, CAP_WRITE) {
        return false;
    }
    if t.check(h1, CAP_MAP) {
        return false;
    }
    if t.get(h1).and_then(|c| c.derive(CAP_READ | CAP_MAP)).is_some() {
        return false;
    }
    if t.get(h1).and_then(|c| c.derive(CAP_READ)).is_none() {
        return false;
    }
    if !t.revoke(h1) || t.check(h1, CAP_READ) || t.get(h1).is_some() {
        return false;
    }

    // The slot is reused, but the old handle must remain permanently stale.
    let h2 = match t.insert(Cap::new(0x200, CAP_READ)) {
        Some(h) => h,
        None => return false,
    };
    if h1 == h2 || t.check(h1, CAP_READ) || !t.check(h2, CAP_READ) {
        return false;
    }
    true
}
