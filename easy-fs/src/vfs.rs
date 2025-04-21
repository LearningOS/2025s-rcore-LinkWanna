//！第五层：索引节点层
use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use log::debug;
use spin::{Mutex, MutexGuard};

/// Virtual filesystem layer over easy-fs
/// 描述 DiskInode 在磁盘中的位置，并封装功能
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    /// Find inode under a disk inode by name
    /// 在指定目录(disk_node)下寻找指定文件(name)的 inode_id
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        debug!("There are {} files in this directory", file_count);

        // 遍历当前目录下的所有文件
        for i in 0..file_count {
            // 确保读取成功，没有 early stop
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            // 可以看出这里并没有递归寻找目录的方案
            // 所以当前的文件视图是扁平的
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                // 获取文件 inode 的位置
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// Create inode under current inode by name
    /// 在当前inode下创建一个新文件
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }

        // create a new file
        // alloc a inode with an indirect block
        // 分配一个 inode
        let new_inode_id = fs.alloc_inode();

        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });

        // 向根目录添加新文件的目录项
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);

            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    /// 进行链接
    pub fn do_linkat(&self, old_name: &str, new_name: &str) {
        let mut fs = self.fs.lock();
        // 向根目录添加新文件的目录项
        debug!("getting disk_inode");
        self.modify_disk_inode(|disk_inode| {
            // append file in the dirent
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, disk_inode, &mut fs);

            // write dirent
            debug!("getting dirent");
            let dirent = self
                .find_inode_id(old_name, disk_inode)
                .map(|inode_id| {
                    // 增加硬链接数量
                    let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                    get_block_cache(block_id as usize, self.block_device.clone())
                        .lock()
                        .modify(block_offset, |disk_inode: &mut DiskInode| {
                            disk_inode.links += 1
                        });
                    // 构建目录项
                    DirEntry::new(new_name, inode_id)
                })
                .unwrap();

            debug!("writing dirent");
            disk_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
        // 同步
        debug!("syncing");
        block_cache_sync_all();
    }

    /// 取消链接
    pub fn do_unlinkat(&self, name: &str) -> bool {
        let mut fs = self.fs.lock();
        // 寻找文件对应的 inode
        let old_inode_id = self.modify_disk_inode(|disk_inode| {
            // 1. 遍历所有文件，删除目录项
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut old_inode_id = 0;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );

                // 如果 inode_id 为 0，则跳过
                if dirent.inode_id() == 0 {
                    continue;
                }
                // 如果文件名匹配，则删除该目录项
                if dirent.name() == name {
                    old_inode_id = dirent.inode_id();
                    disk_inode.write_at(
                        i * DIRENT_SZ,
                        DirEntry::empty().as_bytes(),
                        &self.block_device,
                    );
                    break;
                }
            }
            old_inode_id
        });

        // 如果 old_inode_id 为 0，则直接返回
        if old_inode_id == 0 {
            return false;
        }

        // 获取对应的 disk_inode
        let (old_inode_block_id, old_inode_block_offset) = fs.get_disk_inode_pos(old_inode_id);
        get_block_cache(old_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(old_inode_block_offset, |old_disk_inode: &mut DiskInode| {
                // 2. 减少硬链接数
                old_disk_inode.links -= 1;
                // 3. 如果硬链接数为 0，则释放 inode
                if old_disk_inode.links == 0 {
                    // 释放数据
                    let block_vec = old_disk_inode.clear_size(&Arc::clone(&self.block_device));
                    for block_id in block_vec {
                        fs.dealloc_data(block_id);
                    }
                    // 释放 inode
                    fs.dealloc_inode(old_inode_id);
                }
            });
        // 同步
        block_cache_sync_all();
        true
    }

    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        // 锁住该 fs 实例避免其他核在同时间的访问造成并发冲突
        let _fs = self.fs.lock();

        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            // 遍历所有文件，获取文件名
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );

                // 如果 inode_id 为 0，则跳过
                if dirent.inode_id() == 0 {
                    continue;
                }
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    /// Clear the data in current inode
    /// 清除当前的 Inode 数据
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}

// 重新导出 DiskInode 的函数
impl Inode {
    /// 获取当前 Inode 的 ID
    pub fn inode_id(&self) -> u32 {
        let fs = self.fs.lock();
        fs.get_inode_id(self.block_id, self.block_offset)
    }

    /// 获取硬链接数量
    pub fn link_count(&self) -> u16 {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.links)
    }

    /// 判断当前 Inode 是否为目录
    pub fn is_dir(&self) -> bool {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.is_dir())
    }

    /// 判断当前 Inode 是否为文件
    pub fn is_file(&self) -> bool {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.is_file())
    }
}
