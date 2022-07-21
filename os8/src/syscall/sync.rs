use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;

pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

// LAB5 HINT: you might need to maintain data structures used for deadlock detection
// during sys_mutex_* and sys_semaphore_* syscalls
pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

// LAB5 HINT: Return -0xDEAD if deadlock is detected
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();

    let is_detection = process.inner_exclusive_access().is_enable_deadlock_detection;
    if is_detection{
        let process_inner = process.inner_exclusive_access();
        let res = process_inner.deadlock_detection(mutex_id);
        if !res{
            return -0xDEAD;
        }
    }
    
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    // while process_inner.sem_work.len() < id + 1{
    //     process_inner.sem_work.push(0);
    // }
    // process_inner.sem_work[id] = res_count;
    id as isize
}

pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    // let current_task = current_task().unwrap();
    // let current_task_inner = current_task.inner_exclusive_access();
    // let current_task_res = current_task_inner.res.as_ref().unwrap();
    // let tid = current_task_res.tid;
    // while process_inner.sem_alloc.len() < sem_id + 1{
    //     process_inner.sem_alloc.push(Vec::new());
    // }
    // while process_inner.sem_alloc[sem_id].len()<tid +1 {
    //     process_inner.sem_alloc[sem_id].push(0);
    // }

    // if process_inner.sem_alloc[sem_id][tid] > 0{
    //     process_inner.sem_alloc[sem_id][tid] -= 1;
    // }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);

    
    sem.up();
    0
}

// LAB5 HINT: Return -0xDEAD if deadlock is detected
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();

    let is_detection = process.inner_exclusive_access().is_enable_deadlock_detection;
    if is_detection{
        let process_inner = process.inner_exclusive_access();
        let res = process_inner.deadlock_detection(sem_id);
        if !res{
            drop(process_inner);
            drop(process);
            return -0xDEAD;
        }
    }

    let mut process_inner = process.inner_exclusive_access();

    // let current_task = current_task().unwrap();
    // let current_task_inner = current_task.inner_exclusive_access();
    // let current_task_res = current_task_inner.res.as_ref().unwrap();
    // let tid = current_task_res.tid;
    // while process_inner.sem_need.len() < sem_id + 1{
    //     process_inner.sem_need.push(Vec::new());
    // }
    // while process_inner.sem_need[sem_id].len()<tid +1 {
    //     process_inner.sem_need[sem_id].push(0);
    // }

    // process_inner.sem_need[sem_id][tid] += 1;
    

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    0
}

pub fn sys_condvar_create(_arg: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}

pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}

pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}

// LAB5 YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    if  _enabled < 0 || _enabled > 1 {
        return -1;
    }else if _enabled==0 {
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.is_enable_deadlock_detection = false;
        return 0;
    }else {
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.is_enable_deadlock_detection = true;
        return 0;
    }
}
