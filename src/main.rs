#![no_std]
#![no_main]


mod crypto;
mod utils;
mod context;
mod display;
mod transaction;

use crypto::{
    sign_hash, 
    pedersen, 
    get_pubkey, 
    set_derivation_path
};

use context::{Ctx, RequestType, FieldElement};
use transaction::{
    set_tx_fields,
    set_tx_calldata_lengths,
    set_tx_callarray,
    set_tx_calldata
};

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::io;
use nanos_ui::ui;

use nanos_sdk::bindings::{
    os_lib_call
};

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

#[no_mangle]
extern "C" fn sample_pending() {
    let mut comm = io::Comm::new();

    ui::SingleMessage::new("Pending").show();

    loop {
        match comm.next_event::<Ins>() {
            io::Event::Button(ButtonEvent::RightButtonRelease) => break,
            _ => (),
        }
    }
    ui::SingleMessage::new("Ledger review").show();
    loop {
        match comm.next_event::<Ins>() {
            io::Event::Button(ButtonEvent::BothButtonsRelease) => break,
            _ => (),
        }
    }
}

#[no_mangle]
extern "C" fn sample_main(arg0: u32) {

    let mut comm = io::Comm::new();

    // Draw some 'welcome' screen
    ui::SingleMessage::new(display::WELCOME_SCREEN).show();

    let mut ctx: Ctx = Ctx::new();

    loop {        
        // Wait for either a specific button push to exit the app
        // or an APDU command
        //printf("loop\n");
        match comm.next_event() {
            io::Event::Button(ButtonEvent::RightButtonRelease) => nanos_sdk::exit_app(0),        
            io::Event::Command(ins) => {
                match handle_apdu(&mut comm, ins, &mut ctx) {
                    Ok(()) => {
                        comm.reply_ok();
                    }
                    Err(sw) => comm.reply(sw),
                }
                ui::clear_screen();
                ui::SingleMessage::new(display::WELCOME_SCREEN).show();
            },
            _ => (),
        }
    }
}

#[repr(u8)]
enum Ins {
    GetVersion,
    GetPubkey,
    SignHash,
    PedersenHash,
    SignTx,
    TestPlugin
}

impl TryFrom<io::ApduHeader> for Ins {
    type Error = ();
    fn try_from(header: io::ApduHeader) -> Result<Self, Self::Error> {
        match header.ins {
            0 => Ok(Ins::GetVersion),
            1 => Ok(Ins::GetPubkey),
            2 => Ok(Ins::SignHash),
            3 => Ok(Ins::SignTx),
            4 => Ok(Ins::PedersenHash),
            5 => Ok(Ins::TestPlugin),
            _ => Err(())
        }
    }
}

use nanos_sdk::io::Reply;
use nanos_sdk::plugin::{
    PluginInitParams,
    PluginFeedParams,
    PluginInteractionType
};

fn handle_apdu(comm: &mut io::Comm, ins: Ins, ctx: &mut Ctx) -> Result<(), Reply> {
    
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }
    
    let apdu_header = comm.get_apdu_metadata();
    if apdu_header.cla != 0x80 {
        return Err(io::StatusWords::BadCla.into());
    }

    match ins {
        Ins::GetVersion => {
            let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u8>().unwrap();
            let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u8>().unwrap();
            let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u8>().unwrap();
            comm.append([version_major, version_minor, version_patch].as_slice());
        }
        Ins::GetPubkey => {

            ctx.clear();
            ctx.req_type = RequestType::GetPubkey;

            let mut data = comm.get_data()?;

            match set_derivation_path(&mut data, ctx) {
                Ok(()) => {
                    match get_pubkey(ctx) {
                        Ok(k) => {
                            comm.append(k.as_ref());
                        }
                        Err(e) => {
                            return Err(Reply::from(e));
                        } 
                    }
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        Ins::SignHash => {

            let p1 = apdu_header.p1;
            let p2 = apdu_header.p2;

            let mut data = comm.get_data()?;

            match p1 {
                0 => {
                    ctx.clear();
                    ctx.req_type = RequestType::SignHash;

                    set_derivation_path(&mut data, ctx)?;
                }
                _ => {
                    ctx.hash_info.m_hash = data.into();
                    if p2 > 0 {
                        match display::sign_ui(data) {
                            Ok(v) => {
                                if v {
                                    sign_hash(ctx).unwrap();
                                }
                                else {
                                    return Err(io::StatusWords::UserCancelled.into());
                                }
                            }
                            Err(_e) => {
                                return Err(io::SyscallError::Unspecified.into());
                            }
                        }
                    }
                    else {
                        sign_hash(ctx).unwrap();
                    }
                    comm.append([0x41].as_slice());
                    comm.append(ctx.hash_info.r.as_ref());
                    comm.append(ctx.hash_info.s.as_ref());
                    comm.append([ctx.hash_info.v].as_slice());
                }
            }
        }  
        Ins::PedersenHash => {
            ctx.clear();
            ctx.req_type = RequestType::ComputePedersen;
            let data = comm.get_data()?;
            let (a_s, b_s) = data.split_at(32);
            let mut a: FieldElement = a_s.into();
            let b: FieldElement = b_s.into();
            pedersen::pedersen_hash(&mut a, &b);
            comm.append(&a.value[..]);
        }
        Ins::SignTx => {
            
            let p1 = apdu_header.p1;
            let p2 = apdu_header.p2;
            let mut data = comm.get_data()?;

            match p1 {
                0 => {
                    ctx.clear();
                    ctx.req_type = RequestType::SignTransaction;
                    set_derivation_path(&mut data, ctx)?;
                }
                1 => {
                    set_tx_fields(&mut data, ctx);
                }
                2 => {
                    set_tx_calldata_lengths(&mut data, ctx);
                }
                3 => {
                    set_tx_callarray(&mut data, ctx, p2 as usize);
                }
                4 => {

                    match set_tx_calldata(data, ctx, p2 as usize) {
                        Ok(flag) => {
                            if !flag {
                                return Err(io::StatusWords::UserCancelled.into());
                            }
                        }
                        _ => ()
                    }

                    if p2 + 1 == ctx.tx_info.calldata.call_array_len.into() {
                        sign_hash(ctx).unwrap();
                        comm.append([65u8].as_slice());
                        comm.append(ctx.hash_info.r.as_ref());
                        comm.append(ctx.hash_info.s.as_ref());
                        comm.append([ctx.hash_info.v].as_slice());
                    }
                }
                _ => return Err(io::StatusWords::BadP1P2.into()),
            }
        }
        Ins::TestPlugin => {

            let p1 = apdu_header.p1;

            let plugin_name: &[u8] = "plugin-erc20\0".as_bytes();
            let mut arg: [u32; 3] = [0x00; 3];
            arg[0] = plugin_name.as_ptr() as u32;

            match p1 {
                0 => {

                    ctx.clear();
                    ctx.req_type = RequestType::TestPlugin;

                    let operation: u16 = PluginInteractionType::Check.into();
                    arg[1] = operation as u32;
                    nanos_sdk::testing::debug_print("=========================> Plugin call\n");
                    unsafe {
                        os_lib_call(arg.as_mut_ptr());
                    }
                    nanos_sdk::testing::debug_print("=========================> Plugin has been call\n");
                }
                1 => {
                    let mut plugin_ctx = PluginInitParams {
                        operation: 69,
                        name: [0x00; 100],
                        plugin_internal_ctx: &mut ctx.plugin_internal_ctx as *mut u8,
                        plugin_internal_ctx_len: ctx.plugin_internal_ctx_len
                    };

                    for (idx, b) in "Initialization".bytes().enumerate() {
                        plugin_ctx.name[idx] = b;
                    }

                    let operation: u16 = PluginInteractionType::Init.into();
                    arg[1] = operation as u32;
                    arg[2] = &mut plugin_ctx as *mut PluginInitParams as u32;
                    nanos_sdk::testing::debug_print("=========================> Plugin call\n");
                    unsafe {
                        os_lib_call(arg.as_mut_ptr());
                    }
                    nanos_sdk::testing::debug_print("=========================> Plugin has been call\n");
                }
                2 => {
                    let mut plugin_ctx = PluginFeedParams {
                        plugin_internal_ctx: &mut ctx.plugin_internal_ctx as *mut u8,
                        plugin_internal_ctx_len: ctx.plugin_internal_ctx_len
                    };
                    let operation: u16 = PluginInteractionType::Feed.into();
                    arg[1] = operation as u32;
                    arg[2] = &mut plugin_ctx as *mut PluginFeedParams as u32;
                    nanos_sdk::testing::debug_print("=========================> Plugin call\n");
                    unsafe {
                        os_lib_call(arg.as_mut_ptr());
                    }
                    nanos_sdk::testing::debug_print("=========================> Plugin has been call\n");

                }
                _ => return Err(io::StatusWords::BadP1P2.into()),
            }
            comm.append([0u8].as_slice());
        }
    }
    Ok(())
}