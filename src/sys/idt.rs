/* SPDX-License-Identifier: MIT */
/*
 * Copyright (C) 2022 Advanced Micro Devices, Inc.
 *
 * Authors: Carlos Bilbao <carlos.bilbao@amd.com> and
 *          Tom Lendacky <thomas.lendacky@amd.com>
 */

use lazy_static::lazy_static;
use log::info;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::idt::PageFaultErrorCode;

use crate::error::VmplError;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt: InterruptDescriptorTable = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(bp_handler);
        idt.double_fault.set_handler_fn(df_handler);
        idt.general_protection_fault.set_handler_fn(gp_handler);
        idt.page_fault.set_handler_fn(pf_handler);

        idt
    };
}

fn do_panic(stack_frame: InterruptStackFrame, name: &str, error_code: u64) -> ! {
    let rip: u64 = stack_frame.instruction_pointer.as_u64();
    let msg: String = format!(
        "#{} at RIP {:#0x} with error code {:#0x}",
        name,
        rip,
        error_code
    );

    panic!("{}", msg);
}

/// Breakpoint handler
/// This handler is used for debugging purposes
/// It will print a message and continue execution
extern "x86-interrupt" fn bp_handler(stack_frame: InterruptStackFrame) {
    let rip: u64 = stack_frame.instruction_pointer.as_u64();
    let msg: String = format!("#BP at RIP {:#0x}", rip);

    println!("{}", msg);
}

/// Double fault handler
/// Every interruption except for #PF, #VC and #GP will end up here
extern "x86-interrupt" fn df_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    do_panic(stack_frame, "DF", 0)
}

/// General protection fault handler
extern "x86-interrupt" fn gp_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    do_panic(stack_frame, "GP", error_code)
}

/// Page fault handler
extern "x86-interrupt" fn pf_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    do_panic(stack_frame, "PF", error_code.bits())
}

/// Load IDT with function handlers for each exception
pub fn idt_init() -> Result<(), VmplError> {
    info!("Loading IDT");
    IDT.load();
    Ok(())
}
