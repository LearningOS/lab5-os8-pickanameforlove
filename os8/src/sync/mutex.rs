use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{add_task, current_task};
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

pub trait Mutex: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
    fn get_allocate_tid(&self)-> Option<usize>;
    fn get_waiting_tids(&self)-> Option<Vec<usize>>;
    fn get_count(&self) -> isize;
}

pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    allocate_tid: UPSafeCell<usize>,
}

impl MutexSpin {
    pub fn new() -> Self {
        Self {
            allocate_tid: unsafe {
                UPSafeCell::new(0)
            },
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                let mut tid = self.allocate_tid.exclusive_access();

                let current_task = current_task().unwrap();
                let current_task_inner = current_task.inner_exclusive_access();
                let current_task_res = current_task_inner.res.as_ref().unwrap();
                let td = current_task_res.tid;

                *tid = td;
                return;
            }
        }
    }

    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
    fn get_waiting_tids(&self)-> Option<Vec<usize>> {
        return None;
    }
    fn get_allocate_tid(&self)-> Option<usize> {
        let mut locked = self.locked.exclusive_access();
        if *locked{
            let mut tid = self.allocate_tid.exclusive_access();
            return Some(*tid);
        }else{
            return None;
        }
    }
    fn get_count(&self) -> isize {
        let locked = self.locked.exclusive_access();
        if *locked{
            return 0;
        }else{
            return 1;
        }
    }
}

pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    allocate_tid: usize,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    allocate_tid: 0,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;

            let current_task = current_task().unwrap();
            let current_task_inner = current_task.inner_exclusive_access();
            let current_task_res = current_task_inner.res.as_ref().unwrap();
            let td = current_task_res.tid;

            mutex_inner.allocate_tid = td;
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {

            let current_task_inner = waking_task.inner_exclusive_access();
            let current_task_res = current_task_inner.res.as_ref().unwrap();
            let td = current_task_res.tid;
            mutex_inner.allocate_tid = td;
            drop(current_task_inner);

            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
    fn get_allocate_tid(&self)-> Option<usize> {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked{
            return Some(mutex_inner.allocate_tid);
        }else{
            return None;
        }
    }
    fn get_waiting_tids(&self)-> Option<Vec<usize>> {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked{
            let l = mutex_inner.wait_queue.len();
            if l == 0{
                return None;
            }else{
                let mut res = Vec::new();

                for i in 0..l{
                    let waiting_task = &mutex_inner.wait_queue[i];
                    let current_task_inner = waiting_task.inner_exclusive_access();
                    let current_task_res = current_task_inner.res.as_ref().unwrap();
                    let td = current_task_res.tid;

                    while res.len() < td + 1{
                        res.push(0)
                    }
                    res[td] += 1;

                }
                return Some(res);
            }
        }else{
            return None;
        }
    }
    fn get_count(&self) -> isize {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked{
            return 0;
        }else{
            return 1;
        }
    }
}
