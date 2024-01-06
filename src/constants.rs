/// Page Table Addresses
#[allow(non_snake_case)]
pub mod PageTables {
    /// Page Map Level 4 Table
    pub const PML4: usize = 0x1000;
    /// Page Directory Pointer Table
    pub const PDPT: usize = 0x2000;
    /// Page Directory
    pub const PD: usize = 0x3000;
}

/// Paging
#[allow(non_snake_case)]
pub mod PageFlags {
    /// The page is present in physical memory
    pub const PRESENT: u64 = 1 << 0;
    /// The page is read/write
    pub const READ_WRITE: u64 = 1 << 1;
    /// Make PDE map to a 4MiB page, Page Size Extension must be enabled
    pub const PAGE_SIZE: u64 = 1 << 7;
}

/// Control Register 0
#[allow(non_snake_case)]
pub mod Cr0Flags {
    /// Enable protected mode
    pub const PE: u64 = 1 << 0;
    /// Enable paging
    pub const PG: u64 = 1 << 31;
}

/// Control Register 4
#[allow(non_snake_case)]
pub mod Cr4Flags {
    /// Page Size Extension
    pub const PSE: u64 = 1 << 4;
    /// Physical Address Extension, size of large pages is reduced from
    /// 4MiB to 2MiB and PSE is enabled regardless of the PSE bit
    pub const PAE: u64 = 1 << 5;
}

/// Extended Feature Enable Register
#[allow(non_snake_case)]
pub mod EferFlags {
    /// Long Mode Enable
    pub const LME: u64 = 1 << 8;
    /// Long Mode Active
    pub const LMA: u64 = 1 << 10;
}

/// Code/Data Segment flags
/// Read permission & Data Segment is implied
/// Code Segment is implicitly executable
#[allow(non_snake_case)]
pub mod SegmentFlags {
    /// Read permissions for Code Segment
    pub const CODE_READ: u8 = 1 << 1;
    /// Indicate that this is a Code Segment
    pub const CODE_SEGMENT: u8 = 1 << 3;
    /// Write permissions for Data Segment
    pub const DATA_WRITE: u8 = 1 << 1;
}
