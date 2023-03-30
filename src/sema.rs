// SPDX-License-Identifier:  MIT

use std::error::Error;
use std::ffi::CString;

#[allow(dead_code)]
pub struct Semaphore {
    raw_sema: *mut libc::sem_t,
    name: CString,
}

impl Semaphore {
    pub fn new_with_name(name: &str) -> Result<Semaphore, Box<dyn Error>> {
        let raw_sema_name = CString::new(name)?;

        let s;
        unsafe {
            s = libc::sem_open(
                raw_sema_name.as_ptr(),
                libc::O_CREAT,
                libc::S_IRUSR | libc::S_IWUSR,
                1,
            );
            if s.is_null() {
                return Err(From::from(
                    "Failed to allocate named semaphore, sem_open() failed",
                ));
            }
        }

        Ok(Semaphore {
            raw_sema: s,
            name: raw_sema_name,
        })
    }

    pub fn lock(&mut self) {
        unsafe {
            libc::sem_wait(self.raw_sema);
            debug!("lock taken by PID={}", libc::getpid());
        }
    }

    pub fn unlock(&mut self) {
        unsafe {
            debug!("lock released by PID={}", libc::getpid());
            libc::sem_post(self.raw_sema);
        }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            libc::sem_close(self.raw_sema);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use env_logger;
    use std::{thread, time};

    #[test]
    fn sema_sanity() {
        let _ = env_logger::try_init();
        let s = Semaphore::new_with_name(&"test").unwrap();
        unsafe {
            libc::sem_unlink(s.name.as_ptr());
        }
    }

    #[test]
    fn sema_concurent() {
        let _ = env_logger::try_init();
        let sema = Semaphore::new_with_name(&"test").expect("Failed to create semaphore");

        let t1 = thread::Builder::new()
            .spawn(|| {
                warn!("T1 spawned");
                let mut s = Semaphore::new_with_name(&"test").expect("Failed to create semaphore");

                s.lock();
                warn!("T1 in critical section");
                thread::sleep(time::Duration::from_millis(100));
                warn!("T1 leaving critical section");
                s.unlock();
            })
            .unwrap();

        thread::sleep(time::Duration::from_millis(50));

        let t2 = thread::Builder::new()
            .spawn(|| {
                warn!("T2 spawned");
                let mut s = Semaphore::new_with_name(&"test").expect("Failed to create semaphore");

                s.lock();
                warn!("T2 in critical section");
                thread::sleep(time::Duration::from_millis(3000));
                warn!("T2 leaving critical section");
                s.unlock();
            })
            .unwrap();

        thread::sleep(time::Duration::from_millis(200));

        let t3 = thread::Builder::new()
            .spawn(|| {
                warn!("T3 spawned");
                let mut s = Semaphore::new_with_name(&"test").expect("Failed to create semaphore");

                s.lock();
                warn!("T3 in critical section");
                warn!("T3 leaving critical section");
                s.unlock();
            })
            .unwrap();

        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();

        unsafe {
            libc::sem_unlink(sema.name.as_ptr());
        }

        // XXX: actually check that critical section where taken in right order
    }
}
