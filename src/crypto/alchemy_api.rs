use chrono::{ DateTime, Datelike };
use std::{sync::{Arc, Mutex}, thread};

/* 

TODO:
    - Handle mutex poisoning
        https://blog.logrocket.com/understanding-handling-rust-mutex-poisoning/
    - Handle possible thread panicking

*/


impl CUInner {
    fn default() -> Self {
        CUInner {
            used_cu: Mutex::new(0),
            max_cu: 300_000_000,
            days_since_reset: Mutex::new(0),
        }
    }
    
    fn add_cu(&self, cu: u32) {
        *self.used_cu.lock().unwrap() += cu;
    }

    fn start_of_month_reset_cu(&self) {
        let utc_date: DateTime<chrono::Utc> = chrono::Utc::now();
        let mut days_since_reset = self.days_since_reset.lock().unwrap();

        if utc_date.day() == 1 || ( *days_since_reset >= 28 && utc_date.day() == 2 ) {
            let mut used_cu = self.used_cu.lock().unwrap();
            *used_cu = 0;
            *days_since_reset = 0;
        } else {
            *days_since_reset += 1;
        };
    }


}

impl CU {
    fn default() -> Self {
        CU {
            inner: Arc::new(CUInner::default()),
        }
    }

    // calls the 'start_of_month_reset_cu' function once a day
    fn start(&mut self) {
        let local_self = self.inner.clone();
        
        thread::spawn(move || {
            loop {
                thread::sleep(chrono::Duration::days(1).to_std().unwrap());
                
                local_self.start_of_month_reset_cu();
            }
        });
    }
}

#[derive(Debug, Default)]
struct CUInner {
    used_cu: Mutex<u32>,
    max_cu: u32,
    days_since_reset: Mutex<u8>,
}

#[derive(Debug, Default)]
pub struct CU {
    inner: Arc<CUInner>,
}

pub fn start_cu_instance() -> CU {
    let mut compute_unit: CU = CU::default();
    compute_unit.start();

    compute_unit
}