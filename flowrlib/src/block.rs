use std::fmt;

use serde_derive::{Deserialize, Serialize};

/// blocks: (blocking_id, blocking_io_number, blocked_id, blocked_flow_id) a blocks between functions
#[derive(PartialEq, Clone, Hash, Eq, Serialize, Deserialize)]
pub struct Block {
    /// The id of the flow where the blocking function reside
    pub blocking_flow_id: usize,
    /// The id of the blocking function (destination with input unable to be sent to)
    pub blocking_id: usize,
    /// The number of the io in the blocking function that is full and causing the block
    pub blocking_io_number: usize,
    /// The id of the function that would like to send to the blocking function but cannot because
    /// the input is full
    pub blocked_id: usize,
    /// The id of the flow where the blocked function resides
    pub blocked_flow_id: usize,
}

impl Block {
    /// Create a new `Block`
    pub fn new(
        blocking_flow_id: usize,
        blocking_id: usize,
        blocking_io_number: usize,
        blocked_id: usize,
        blocked_flow_id: usize,
    ) -> Self {
        Block {
            blocking_flow_id,
            blocking_id,
            blocking_io_number,
            blocked_id,
            blocked_flow_id,
        }
    }
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "#{}({}) -> #{}({}):{}",
            self.blocked_id,
            self.blocked_flow_id,
            self.blocking_id,
            self.blocking_flow_id,
            self.blocking_io_number
        )
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "#{}({}) -> #{}({}):{}",
            self.blocked_id,
            self.blocked_flow_id,
            self.blocking_id,
            self.blocking_flow_id,
            self.blocking_io_number
        )
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn display_block_test() {
        let block = super::Block::new(1, 2, 0, 1, 0);
        println!("Block: {}", block);
    }

    #[test]
    fn debug_block_test() {
        let block = super::Block::new(1, 2, 0, 1, 0);
        println!("Block: {:?}", block);
    }

    #[test]
    fn block_new_test() {
        let block = super::Block::new(1, 2, 0, 1, 0);
        assert_eq!(block.blocking_flow_id, 1);
        assert_eq!(block.blocking_id, 2);
        assert_eq!(block.blocking_io_number, 0);
        assert_eq!(block.blocked_id, 1);
        assert_eq!(block.blocked_flow_id, 0);
    }
}
