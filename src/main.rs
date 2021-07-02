#![no_std]
#![no_main]
#![feature(asm)]
#[allow(dead_code)]
#[allow(unused_mut)]
#[macro_use]

extern crate toppers_asp;

mod kernel_cfg;
mod sample1_helper;

use toppers_asp::kernel::kernel_api::*;
use toppers_asp::kernel::stddef::*;
use toppers_asp::syssvc::serial::*;
use toppers_asp::syssvc::syslog::*;

use sample1_helper::*;

#[allow(unused_imports)]
use kernel_cfg::*;

use core::ffi::c_void;
use core::panic::PanicInfo;
use core::ptr::{read_volatile, write_volatile};
use core::usize;

pub fn svc_error_output(prio: u32, file: &'static str, line: u32, expr: &'static str, ercd: Er) {
    if ercd < 0 {
        toppers_syssvc_syslog!(
            prio,
            "%s (%d) reported by `%s' in line %d of `%s'.",
            ercd.to_u8ptr() as u32,
            sercd(ercd) as u32,
            expr.as_bytes().as_ptr() as u32,
            line,
            file.as_bytes().as_ptr() as u32
        );
    }
}

macro_rules! svc_perror {
    ($expr : expr) => {
        svc_error_output(
            LOG_ERROR,
            concat!(file!(), '\0'),
            line!(),
            concat!(stringify!($expr), '\0'),
            $expr,
        );
    };
    ($expr : expr, $ercd : expr, $info : expr) => {
        let mut cls = || -> Er{
            let (e, x) = $expr;
            $info = x;
            $ercd = e;
            e
        };
        svc_error_output(
            LOG_ERROR,
            concat!(file!(), '\0'),
            line!(),
            concat!(stringify!($expr), '\0'),
            cls()
        );
    };
}

fn raise_cpu_exception() {
    #[cfg(feature = "target_support_cpu_exception")]
    {
        sample1_raise_cpu_exception();
    }
}

fn empty_loop(count: u32) {
    let mut _i: u32 = 0;
    unsafe {
        loop {
            write_volatile(&mut _i, _i + 1);
            if read_volatile(&_i) > count {
                break;
            }
        }
    }
}

/*
 * タスク優先度
 */
const HIGH_PRIORITY: Pri = 9;
const MID_PRIORITY: Pri = 10;
const LOW_PRIORITY: Pri = 11;

/*
 *  並行実行されるタスクへのメッセージ領域
 */
static mut MESSAGE: [char; 3] = [0 as char; 3];

/*
 *  ループ回数
 */
const LOOP_REF: u32 = 1000000; /* 速度計測用のループ回数 */
static mut TASK_LOOP: u32 = 0; /* タスク内でのループ回数 */
static mut TEX_LOOP: u32 = 0; /* 例外処理ルーチン内でのループ回数 */

/* シリアルポートID */
const TASK_PORTID: Id = 1;

#[no_mangle]
#[allow(unreachable_code)]
extern "C" fn task(exinf: i32) {
    let mut n: u32 = 0;
    let tskno: i32 = exinf;
    let graph: [*const u8; 3] = [
        "|\0".as_bytes().as_ptr(),
        "  +\0".as_bytes().as_ptr(),
        "    *\0".as_bytes().as_ptr(),
    ];
    let mut c: char;

    svc_perror!(Asp::ena_tex());

    loop {
        toppers_syssvc_syslog!(
            LOG_NOTICE,
            "task%d is running (%03d).   %s",
            tskno as u32,
            n,
            graph[(tskno - 1) as usize] as u32
        );
        n += 1;

        unsafe {
            empty_loop(TASK_LOOP);
            c = MESSAGE[(tskno - 1) as usize];
            MESSAGE[(tskno - 1) as usize] = 0 as char;
        }

        match c {
            'e' => {
                toppers_syssvc_syslog!(LOG_INFO, "#%d#ext_tsk()", tskno as u32);
                svc_perror!(Asp::ext_tsk());
            }
            's' => {
                toppers_syssvc_syslog!(LOG_INFO, "#%d#slp_tsk()", tskno as u32);
                svc_perror!(Asp::slp_tsk());
            }
            'S' => {
                toppers_syssvc_syslog!(LOG_INFO, "#%d#tslp_tsk(10000)", tskno as u32);
                svc_perror!(Asp::tslp_tsk(10000));
            }
            'd' => {
                toppers_syssvc_syslog!(LOG_INFO, "#%d#dly_tsk(10000)", tskno as u32);
                svc_perror!(Asp::dly_tsk(10000));
            }
            'y' => {
                toppers_syssvc_syslog!(LOG_INFO, "#%d#dis_tex()", tskno as u32);
                svc_perror!(Asp::dis_tex());
            }
            'Y' => {
                toppers_syssvc_syslog!(LOG_INFO, "#%d#ena_tex()", tskno as u32);
                svc_perror!(Asp::ena_tex());
            }
            'z' => {
                toppers_syssvc_syslog!(LOG_NOTICE, "#%d#raise CPU exception", tskno as u32);
                raise_cpu_exception();
            }
            'Z' => {
                svc_perror!(Asp::loc_cpu());
                toppers_syssvc_syslog!(LOG_NOTICE, "#%d#raise CPU exception", tskno as u32);
                raise_cpu_exception();
                svc_perror!(Asp::unl_cpu());
            }
            _ => (),
        }
    }

    let _ = Asp::ext_tsk();
}

#[no_mangle]
extern "C" fn tex_routine(texptn: TexPtn, exinf: i32) {
    let tskno: i32 = exinf;

    toppers_syssvc_syslog!(
        LOG_NOTICE,
        "task%d receives exception 0x%04x.",
        tskno as u32,
        texptn
    );

    unsafe { for _i in 0..TEX_LOOP {} }

    if (texptn & 0x8000u32) != 0 {
        toppers_syssvc_syslog!(LOG_INFO, "#%d#ext_tsk()", tskno as u32);
        svc_perror!(Asp::ext_tsk());
        toppers_assert!(false);
    }
}

#[no_mangle]
extern "C" fn cpuexc_handler(p_excinf: &c_void) {
    toppers_syssvc_syslog!(
        LOG_NOTICE,
        "CPU exception handler (p_excinf = 0x%08x).",
        p_excinf as *const c_void as u32
    );

    if Asp::sns_ctx() != true {
        toppers_syssvc_syslog!(
            LOG_WARNING,
            "sns_ctx() is not true in CPU exception handler.",
        );
    }
    if Asp::sns_dpn() != true {
        toppers_syssvc_syslog!(
            LOG_WARNING,
            "sns_dpn() is not true in CPU exception handler.",
        );
    }
    toppers_syssvc_syslog!(
        LOG_INFO,
        "sns_loc = %d sns_dsp = %d sns_tex = %d",
        Asp::sns_loc() as u32,
        Asp::sns_dsp() as u32,
        Asp::sns_tex() as u32
    );
    toppers_syssvc_syslog!(
        LOG_INFO,
        "xsns_dpn = %d xsns_xpn = %d",
        Asp::xsns_dpn(p_excinf) as u32,
        Asp::xsns_xpn(p_excinf) as u32
    );

    if Asp::xsns_xpn(p_excinf) == true {
        toppers_syssvc_syslog!(LOG_NOTICE, "Sample program ends with exception.",);
        svc_perror!(Asp::ext_ker());
        toppers_assert!(false);
    }

    let mut _ercd : Er = E_OK;
    let mut tskid: Id = 0;
    svc_perror!(Asp::iget_tid(), _ercd, tskid);
    svc_perror!(Asp::iras_tex(tskid, 0x8000u32));
}

#[no_mangle]
extern "C" fn cyclic_handler(_exinf: u32) {
    svc_perror!(Asp::irot_rdq(HIGH_PRIORITY));
    svc_perror!(Asp::irot_rdq(MID_PRIORITY));
    svc_perror!(Asp::irot_rdq(LOW_PRIORITY));
}

#[no_mangle]
extern "C" fn alarm_handler(_exinf: u32) {
    svc_perror!(Asp::irot_rdq(HIGH_PRIORITY));
    svc_perror!(Asp::irot_rdq(MID_PRIORITY));
    svc_perror!(Asp::irot_rdq(LOW_PRIORITY));
}

#[no_mangle]
#[allow(unreachable_code)]
extern "C" fn main_task(exinf: i32) {
    let mut ercd: Er;

    svc_perror!(Syslog::syslog_msk_log(
        Syslog::syslog_log_upto(LOG_INFO),
        Syslog::syslog_log_upto(LOG_EMERG)
    ));

    toppers_syssvc_syslog!(
        LOG_NOTICE,
        "Sample program starts (exinf = %d).",
        exinf as u32
    );
    ercd = Serial::serial_opn_por(TASK_PORTID);

    if ercd < 0 && mercd(ercd) != E_OBJ {
        toppers_syssvc_syslog!(
            LOG_ERROR,
            "%s (%d) reported by `serial_opn_por'.",
            ercd.to_u8ptr() as u32,
            sercd(ercd) as u32
        );
    }

    svc_perror!(Serial::serial_ctl_por(
        TASK_PORTID,
        IOCTL_CRLF | IOCTL_FCSND | IOCTL_FCRCV
    ));

    /* 0.4秒経過させるためのループ回数の算出 */
    let mut time1: Systim = 0;
    let mut time2: Systim = 0;

    svc_perror!(Asp::get_tim(), ercd, time1);
    empty_loop(LOOP_REF);
    svc_perror!(Asp::get_tim(),  ercd,time2);

    unsafe {
        TASK_LOOP = LOOP_REF * 400 / (time2 - time1);
        TEX_LOOP = TASK_LOOP / 4;
    }

    /*
     *  タスクの起動
     */
    svc_perror!(Asp::act_tsk(TASK1));
    svc_perror!(Asp::act_tsk(TASK2));
    svc_perror!(Asp::act_tsk(TASK3));

    /* メインループ */
    let mut rbuf: u8 = 0;
    let mut tskno: usize = 1 as usize;
    let mut tskid: Id = TASK1;

    'main_loop: loop {
        svc_perror!(Serial::serial_rea_dat(TASK_PORTID, &mut rbuf, 1));
        let c = rbuf as char;

        match c {
            'e' | 's' | 'S' | 'd' | 'y' | 'Y' | 'z' | 'Z' => unsafe {
                MESSAGE[tskno - 1] = c;
            },
            '1' => {
                tskno = 1;
                tskid = TASK1;
            }
            '2' => {
                tskno = 2;
                tskid = TASK2;
            }
            '3' => {
                tskno = 3;
                tskid = TASK3;
            }
            'a' => {
                toppers_syssvc_syslog!(LOG_INFO, "#act_tsk(%d)", tskno as u32);
                svc_perror!(Asp::act_tsk(tskid));
            }
            'A' => {
                toppers_syssvc_syslog!(LOG_INFO, "#can_act(%d)", tskno as u32);
                ercd = Asp::can_act(tskid);
                if ercd >= 0 {
                    toppers_syssvc_syslog!(
                        LOG_NOTICE,
                        "can_act(%d) returns %d",
                        tskno as u32,
                        ercd as u32
                    );
                } else {
                    toppers_syssvc_syslog!(
                        LOG_ERROR,
                        "can_act(%d) returns %d",
                        tskno as u32,
                        ercd as u32
                    );
                }
            }
            't' => {
                toppers_syssvc_syslog!(LOG_INFO, "#ter_tsk(%d)", tskno as u32);
                svc_perror!(Asp::ter_tsk(tskid));
            }
            '>' => {
                toppers_syssvc_syslog!(LOG_INFO, "chg_pri(%d, HIGH_PRIORITY)", tskno as u32);
                svc_perror!(Asp::chg_pri(tskid, HIGH_PRIORITY));
            }
            '=' => {
                toppers_syssvc_syslog!(LOG_INFO, "#chg_pri(%d, MID_PRIORITY)", tskno as u32);
                svc_perror!(Asp::chg_pri(tskid, MID_PRIORITY));
            }
            '<' => {
                toppers_syssvc_syslog!(LOG_INFO, "chg_pri(%d, LOW_PRIORITY)", tskno as u32);
                svc_perror!(Asp::chg_pri(tskid, LOW_PRIORITY));
            }
            'G' => {
                toppers_syssvc_syslog!(LOG_INFO, "#get_pri(%d, &tskpri)", tskno as u32);
                let mut tskpri: Pri = 0;
                svc_perror!(Asp::get_pri(tskid), ercd, tskpri);
                if ercd >= 0 {
                    toppers_syssvc_syslog!(
                        LOG_NOTICE,
                        "priority of task %d is %d",
                        tskno as u32,
                        tskpri as u32
                    );
                }
            }
            'w' => {
                toppers_syssvc_syslog!(LOG_INFO, "#wup_tsk(%d)", tskno as u32);
                svc_perror!(Asp::wup_tsk(tskid));
            }
            'l' => {
                toppers_syssvc_syslog!(LOG_INFO, "#rel_wai(%d)", tskno as u32);
                svc_perror!(Asp::rel_wai(tskid));
            }
            'u' => {
                toppers_syssvc_syslog!(LOG_INFO, "#sus_tsk(%d)", tskno as u32);
                svc_perror!(Asp::sus_tsk(tskid));
            }
            'm' => {
                toppers_syssvc_syslog!(LOG_INFO, "#rsm_tsk(%d)", tskno as u32);
                svc_perror!(Asp::rsm_tsk(tskid));
            }
            'x' => {
                toppers_syssvc_syslog!(LOG_INFO, "#ras_tex(%d, 0x0001U)", tskno as u32);
                svc_perror!(Asp::ras_tex(tskid, 0x0001 as TexPtn));
            }
            'X' => {
                toppers_syssvc_syslog!(LOG_INFO, "#ras_tex(%d, 0x0002U)", tskno as u32);
                svc_perror!(Asp::ras_tex(tskid, 0x0002 as TexPtn));
            }
            'r' => {
                toppers_syssvc_syslog!(LOG_INFO, "#rot_rdq(three priorities)",);
                svc_perror!(Asp::rot_rdq(HIGH_PRIORITY));
                svc_perror!(Asp::rot_rdq(MID_PRIORITY));
                svc_perror!(Asp::rot_rdq(LOW_PRIORITY));
            }
            'c' => {
                toppers_syssvc_syslog!(LOG_INFO, "#sta_cyc(1)",);
                svc_perror!(Asp::sta_cyc(CYCHDR1));
            }
            'C' => {
                toppers_syssvc_syslog!(LOG_INFO, "#stp_cyc(1)",);
                svc_perror!(Asp::stp_cyc(CYCHDR1));
            }
            'b' => {
                toppers_syssvc_syslog!(LOG_INFO, "#sta_alm(1, 5000)",);
                svc_perror!(Asp::sta_alm(ALMHDR1, 5000));
            }
            'B' => {
                toppers_syssvc_syslog!(LOG_INFO, "#stp_alm(1, 5000)",);
                svc_perror!(Asp::stp_alm(ALMHDR1));
            }
            'V' => {
                #[cfg(feature = "toppers_support_get_utm")]
                {
                    let mut utime1 = 0;
                    let mut utime2 = 0;

                    svc_perror!(Asp::get_utm(), ercd, utime1);
                    svc_perror!(Asp::get_utm(), ercd, utime2);

                    toppers_syssvc_syslog!(LOG_INFO, "utime1 = %ld, utime2 = %ld", utime1, utime2);
                }
                #[cfg(not(feature = "toppers_support_get_utm"))]
                {
                    toppers_syssvc_syslog!(LOG_NOTICE, "get_utm is not supported.",);
                }
            }
            'Q' | '\u{003}' => break 'main_loop,
            _ => (),
        }
    }

    toppers_syssvc_syslog!(LOG_NOTICE, "Sample program ends.",);
    svc_perror!(Asp::ext_ker());
    toppers_assert!(false);
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    toppers_syssvc_syslog!(LOG_EMERG, "Panic!.",);
    loop {}
}
