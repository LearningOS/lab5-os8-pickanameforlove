use crate::sync::UPSafeCell;
use crate::task::{add_task, block_current_and_run_next, current_task, TaskControlBlock};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

pub struct Semaphore {
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
    pub allocated_queue: Vec<usize>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                    allocated_queue: Vec::new(),
                })
            },
        }
    }

    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;

        let current_task = current_task().unwrap();
        let current_task_inner = current_task.inner_exclusive_access();
        let current_task_res = current_task_inner.res.as_ref().unwrap();
        let tid = current_task_res.tid;
        if tid < inner.allocated_queue.len() && inner.allocated_queue[tid] > 0{
            inner.allocated_queue[tid] -= 1;
        }
        drop(current_task_inner);

        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let task_inner = task.inner_exclusive_access();
                let task_res = task_inner.res.as_ref().unwrap();
                let task_tid = task_res.tid;
                while inner.allocated_queue.len() < task_tid + 1{
                    inner.allocated_queue.push(0);
                }
                inner.allocated_queue[task_tid] += 1;

                drop(task_inner);

                add_task(task);
            }
        }
    }

    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
        else{
            let current_task = current_task().unwrap();
            let current_task_inner = current_task.inner_exclusive_access();
            let current_task_res = current_task_inner.res.as_ref().unwrap();
            let tid = current_task_res.tid;

            while inner.allocated_queue.len() < tid + 1{
                inner.allocated_queue.push(0);
            }
            inner.allocated_queue[tid] += 1;
        }
    }
    pub fn get_count(&self)-> isize{
        let mut inner = self.inner.exclusive_access();
        return inner.count;
    }

    pub fn get_allocated_tids(&self)-> Option<Vec<usize>> {
        let mut inner = self.inner.exclusive_access();
        if inner.allocated_queue.len() > 0 {
            return Some(inner.allocated_queue.clone())
        }else{
            return None;
        }
    }
    pub fn get_waiting_tids(&self) -> Option<Vec<usize>>{
        let mut inner = self.inner.exclusive_access();
        let l = inner.wait_queue.len();
        if l > 0{
            let mut res = Vec::new();
            for i in 0..l{
                let waiting_task = &inner.wait_queue[i];
                let current_task_inner = waiting_task.inner_exclusive_access();
                let current_task_res = current_task_inner.res.as_ref().unwrap();
                let td = current_task_res.tid;

                while res.len() < td + 1{
                    res.push(0);
                }
                res[td] += 1;

            }
            return Some(res);
        }else{
            return None;
        }
    }

}
