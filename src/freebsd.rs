#![allow(bad_style)]

use crate::Result;
use std::mem::{MaybeUninit, size_of};
use ::libc::*;

pub fn set_thread_affinity(core_ids: &[usize]) -> Result<()> {
    let mut set: cpuset_t = unsafe{MaybeUninit::uninit().assume_init()};
    CPU_ZERO(&mut set);
    for core_id in core_ids {
        CPU_SET(*core_id, &mut set);
    }

    if let Err(e) = _sched_setaffinity(_pthread_self(), size_of::<cpuset_t>(), &set) {
        return Err(From::from(format!("sched_setaffinity failed with errno {}", e)));
    }
    Ok(())
}

pub fn get_thread_affinity() -> Result<Vec<usize>> {
    let mut affinity = Vec::new();
    let mut set: cpuset_t = unsafe{MaybeUninit::uninit().assume_init()};
    CPU_ZERO(&mut set);

    if let Err(e) = _sched_getaffinity(_pthread_self(), size_of::<cpuset_t>(), &mut set) {
        return Err(From::from(format!("sched_getaffinity failed with errno {}", e)));
    }

    for i in 0..CPU_SETSIZE as usize {
        if CPU_ISSET(i, &set) {
            affinity.push(i);
        }
    }

    Ok(affinity)
}

/* Wrappers around unsafe OS calls */
fn _sched_setaffinity(pid: pthread_t, cpusetsize: usize, mask: *const cpuset_t) -> std::result::Result<(), i32> {
    let res = unsafe{pthread_setaffinity_np(pid, cpusetsize, mask)};
    if res != 0 {
        return Err(res);
    }
    Ok(())
}

fn _sched_getaffinity(pid: pthread_t, cpusetsize: usize, mask: *mut cpuset_t) -> std::result::Result<(), i32> {
    let res = unsafe{pthread_getaffinity_np(pid, cpusetsize, mask)};
    if res != 0 {
        return Err(res);
    }
    Ok(())
}

fn _pthread_self() -> pthread_t {
    unsafe{pthread_self()}
}

pub const _BITSET_BITS: usize = size_of::<c_ulong>() * 8;

#[inline(always)]
const fn __howmany(x: usize, y: usize) -> usize
{
    (x + (y - 1)) / y
}

#[inline(always)]
const fn __bitset_words(_s: usize) -> usize
{
    __howmany(_s, _BITSET_BITS)
}

macro_rules! BITSET_DEFINE
{
    ($t: tt, $_s: ident) =>
    {
        pub(crate) type $t = [c_ulong; __bitset_words($_s)];
    }
}

#[inline(always)]
fn __bitset_mask(_s: usize, n: size_t) -> c_ulong
{
    let relative_bit = if __bitset_words(_s) == 1
    {
        n
    }
    else
    {
        n % _BITSET_BITS
    };
    const bit: c_ulong = 1;
    bit << (relative_bit as c_ulong)
}

#[inline(always)]
fn __bitset_word(_s: usize, n: size_t) -> usize
{
    if __bitset_words(_s) == 1
    {
        0
    }
    else
    {
        n / _BITSET_BITS
    }
}

#[inline(always)]
pub fn BIT_SET(_s: usize, n: size_t, p: &mut _cpuset)
{
    p[__bitset_word(_s, n)] |= __bitset_mask(_s, n)
}

#[inline(always)]
pub fn BIT_ISSET(_s: usize, n: size_t, p: &_cpuset) -> bool
{
    (p[__bitset_word(_s, n)] & __bitset_mask(_s, n)) != 0
}

// sys/_cpuset.h
pub const CPU_MAXSIZE: size_t = 256;
pub const CPU_SETSIZE: size_t = CPU_MAXSIZE;

BITSET_DEFINE!(_cpuset, CPU_SETSIZE);
pub type cpuset_t = _cpuset;

#[inline(always)]
pub fn CPU_SET(n: usize, p: &mut _cpuset)
{
    BIT_SET(CPU_SETSIZE, n, p);
}
#[inline(always)]
pub fn CPU_ISSET(n: usize, p: &_cpuset) -> bool
{
    BIT_ISSET(CPU_SETSIZE, n, p)
}
#[inline(always)]
pub fn CPU_ZERO(p: &mut _cpuset)
{
    for idx in 0..(CPU_SETSIZE / _BITSET_BITS) {
        p[idx] = 0;
    }
}

extern "C" {
    pub fn pthread_getaffinity_np(tid: pthread_t, cpusetsize: size_t, cpusetp: *mut cpuset_t) -> c_int;
    pub fn pthread_setaffinity_np(tid: pthread_t, cpusetsize: size_t, cpusetp: *const cpuset_t) -> c_int;
}
