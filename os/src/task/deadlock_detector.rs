use alloc::vec::Vec;

/// 死锁检测器
/// 维护在进程中
pub struct DeadlockDetector {
    available_vec: Vec<u32>,
    request_mat: Vec<Vec<u32>>,
    allocation_mat: Vec<Vec<u32>>,
    thread_count: usize,
    resource_count: usize,
}

impl DeadlockDetector {
    /// 创建一个新的死锁检测器
    pub fn new() -> Self {
        let mut request_mat = Vec::new();
        request_mat.push(Vec::new());

        let mut allocation_mat = Vec::new();
        allocation_mat.push(Vec::new());

        DeadlockDetector {
            available_vec: Vec::new(),
            request_mat,
            allocation_mat,
            thread_count: 1,
            resource_count: 0,
        }
    }

    /// 添加一个线程
    pub fn add_thread(&mut self) {
        self.thread_count += 1;

        let mut request_vec = Vec::new();
        request_vec.resize(self.resource_count, 0);
        self.request_mat.push(request_vec);

        let mut allocation_vec = Vec::new();
        allocation_vec.resize(self.resource_count, 0);
        self.allocation_mat.push(allocation_vec);
    }

    /// 添加一个资源
    pub fn add_resource(&mut self, resource_count: u32) {
        self.available_vec.push(resource_count);
        self.resource_count += 1;
        // 每个线程可用资源 +1
        for vec in &mut self.request_mat {
            vec.push(0);
        }
        for vec in &mut self.allocation_mat {
            vec.push(0);
        }
    }

    /// 请求资源
    /// 成功返回 true, 否则返回 false
    pub fn request_resources(&mut self, tid: usize, resource_id: usize) -> bool {
        // 若资源足够，则直接分配
        // 否则加入请求矩阵
        if self.available_vec[resource_id] > 0 {
            self.available_vec[resource_id] -= 1;
            self.allocation_mat[tid][resource_id] += 1;
        } else {
            self.request_mat[tid][resource_id] += 1;
        }

        !self.is_deadlock()
    }

    /// 释放资源
    pub fn release_resources(&mut self, tid: usize, resource_id: usize) {
        if self.allocation_mat[tid][resource_id] > 0 {
            self.allocation_mat[tid][resource_id] -= 1;
            self.available_vec[resource_id] += 1;
        } else {
            self.available_vec[resource_id] += 1;
        }
    }

    /// 判断是否发生死锁
    fn is_deadlock(&mut self) -> bool {
        // 资源向量
        let mut work = self.available_vec.clone();
        let mut finish: Vec<bool> = Vec::new();
        for item in &self.allocation_mat {
            if item.iter().all(|&x| x == 0) {
                finish.push(true);
            } else {
                finish.push(false);
            }
        }

        // 核心算法
        for _ in 0..self.thread_count {
            let mut found = false;
            for i in 0..self.thread_count {
                if !finish[i] && self.request_mat[i].iter().zip(&work).all(|(&r, &a)| r <= a) {
                    for j in 0..self.resource_count {
                        work[j] += self.allocation_mat[i][j];
                    }
                    finish[i] = true;
                    found = true;
                }
            }
            if finish.iter().all(|&x| x) {
                return false;
            }

            // 如果没有找到可以释放资源的线程，则说明发生了死锁
            if !found {
                return true;
            }
        }
        false
    }
}
