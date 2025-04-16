use winapi::ctypes::c_void;
use winapi::um::psapi::{EnumProcesses, EnumProcessModules, GetModuleBaseNameA};
use winapi::um::processthreadsapi::OpenProcess;
use winapi::shared::minwindef::{HMODULE, FALSE, DWORD};
use winapi::um::winnt;

use std::{io, fmt, ptr};
use std::ptr::NonNull;
use std::mem::{self, size_of_val, MaybeUninit, size_of};
use std::ffi::CString;
use std::ptr::null_mut as NULL;

const MAX_PROC_NAME_LEN: usize = 64;
const MAX_PIDS: usize = 1024;
static PROGRAM_PID: Option<&str> = option_env!("PID");

#[derive(Debug)]
struct Process {
    pid: u32,
    handle: NonNull<c_void>
}
struct ProcessItem {
    pid: u32,
    name: String
}

impl fmt::Display for ProcessItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (pid={})", self.name, self.pid)
    }
}

pub fn enumProcs() -> io::Result<Vec<u32>> {
    let mut size: u32 = 0;
    let mut pids: Vec<u32> = Vec::<DWORD>::with_capacity(MAX_PIDS);

    if unsafe {
        EnumProcesses(
            pids.as_mut_ptr(),
            (pids.capacity() * size_of::<DWORD>()) as u32, 
            &mut size
        )
    } == FALSE {
        return Err(io::Error::last_os_error());
    }

    let count = size as usize / mem::size_of::<DWORD>();
    unsafe { pids.set_len(count) };
    Ok(pids)
}

impl Process {
    pub fn open(pid: u32) -> io::Result<Self> {
        NonNull::new(unsafe {
            OpenProcess(
                winnt::PROCESS_QUERY_INFORMATION | winnt::PROCESS_VM_READ | winnt::PROCESS_VM_WRITE | winnt::PROCESS_VM_OPERATION,
                FALSE, 
                pid
            )
        }).map(|handle| Self {pid, handle})
        .ok_or_else(io::Error::last_os_error)
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn name(&self) -> io::Result<String> {
        let mut module = MaybeUninit::<HMODULE>::uninit();
        let mut size: u32 = 0;

        if unsafe {
            EnumProcessModules(
                self.handle.as_ptr(),
                module.as_mut_ptr(), 
                mem::size_of::<HMODULE>() as u32, 
                &mut size
            )
        } == FALSE {
            return Err(io::Error::last_os_error());
        }

        let module = unsafe { module.assume_init() };
        let mut buffer = Vec::<u8>::with_capacity(MAX_PROC_NAME_LEN);
        let length: u32 = unsafe {
            GetModuleBaseNameA(
                self.handle.as_ptr(),
                module,
                buffer.as_mut_ptr().cast(),
                buffer.capacity() as u32
            )
        };

        if length == 0 {
            return Err(io::Error::last_os_error());
        }

        unsafe { buffer.set_len(length as usize) };
        Ok(String::from_utf8(buffer).unwrap())
    }

    pub fn enum_module(&self) -> io::Result<Vec<HMODULE>> {
        let mut size: u32 = 0;
        if unsafe {
            EnumProcessModules(
                self.handle.as_ptr(), 
                NULL(), 
                0, 
                &mut size
            )
        } == FALSE {
            return  Err(io::Error::last_os_error());
        }

        let mut modules = Vec::with_capacity(size as usize / mem::size_of::<HMODULE>());
        if unsafe {
            EnumProcessModules(self.handle.as_ptr(), 
                modules.as_mut_ptr(), 
                (modules.capacity() * mem::size_of::<HMODULE>()) as u32,
                &mut size
            )
        } == FALSE {
            return  Err(io::Error::last_os_error());
        }

        unsafe { modules.set_len(size as usize / mem::size_of::<HMODULE>() )};
        Ok(modules)
    }
}

fn printProc(p: &ProcessItem) {
    println!("[+] {} ({})", p.name, p.pid);
}


fn main() {
    let _processes = enumProcs()
        .unwrap()
        .iter()
        .flat_map(|&pid| Process::open(pid))
        .filter_map(|proc| {
            match proc.name() {
                Ok(name) => {
                    let p = ProcessItem {
                        pid: proc.pid(),
                        name,
                    };
                    printProc(&p);
                    Some(p)
                }
                Err(_) => None,
            }
        })
        .collect::<Vec<_>>();
}
