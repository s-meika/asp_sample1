pub fn sample1_raise_cpu_exception(){
    unsafe {
        asm!("mcr p15, 0, r1, c2, c0, 0");
    }
}
