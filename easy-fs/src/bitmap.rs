use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;

/// A bitmap block
type BitmapBlock = [u64; 64]; // 使用 u64 加速搜索
/// Number of bits in a block
const BLOCK_BITS: usize = BLOCK_SZ * 8;

/// A bitmap
pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

/// Decompose bits into (block_pos, bits64_pos, inner_pos)
/// 将 bit 分解为区域中的块编号 block_pos、块内的组编号 bits64_pos 以及组内编号 inner_pos 的三元组
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    /// A new bitmap from start block id and number of blocks
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// Allocate a new block from a block device
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        // 遍历所有的位图块
        for block_id in 0..self.blocks {
            let pos = get_block_cache(
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                // 寻找空闲位
                // bits64_pos: 位图块中的第几个 u64
                // inner_pos: u64 中的第几个位(从最低位开始)
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX)
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                {
                    // trailing_ones：返回从最低位开始连续的 1 的数量
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                } else {
                    None
                }
            });

            if pos.is_some() {
                return pos;
            }
        }
        None
    }

    /// Deallocate a block
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }

    /// Get the max number of allocatable blocks
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
