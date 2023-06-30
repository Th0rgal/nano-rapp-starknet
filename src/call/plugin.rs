use starknet_sdk::types::{
    Call, FieldElement, AbstractCallData, AbstractCall
};

use nanos_sdk::testing::debug_print;
use nanos_sdk::plugin::{
    PluginResult,
    PluginInteractionType,
    PluginParam,
    plugin_call_v2
};
use nanos_sdk::string;
use nanos_ui::ui;

use crate::context::Ctx;

struct PluginItem<'a> {
    addr: FieldElement,
    name: &'a str
}

pub fn plugin_find(ctx: &mut Ctx) -> Option<&'static str> {

    let plugins: [PluginItem; 2] = [
        PluginItem {
            addr: FieldElement { value: [
                0x04, 0x9d, 0x36, 0x57, 0x0d, 0x4e, 0x46, 0xf4, 0x8e, 0x99, 0x67, 0x4b, 0xd3, 0xfc, 0xc8, 0x46,
                0x44, 0xdd, 0xd6, 0xb9, 0x6f, 0x7c, 0x74, 0x1b, 0x15, 0x62, 0xb8, 0x2f, 0x9e, 0x00, 0x4d, 0xc7] 
            },
            name: "plugin-erc20\0"
        },
        PluginItem {
            addr: FieldElement { value: [
                0x06, 0x8f, 0x5c, 0x6a, 0x61, 0x78, 0x07, 0x68, 0x45, 0x5d, 0xe6, 0x90, 0x77, 0xe0, 0x7e, 0x89, 
                0x78, 0x78, 0x39, 0xbf, 0x81, 0x66, 0xde, 0xcf, 0xbf, 0x92, 0xb6, 0x45, 0x20, 0x9c, 0x0f, 0xb8] 
            },
            name: "plugin-erc20\0"
        }
    ];

    let mut plugin_name = "";
    for item in plugins {
        if ctx.call.to == item.addr {
            plugin_name = item.name;
            break;
        }
    }

    /* CHECK */
    let mut params = PluginParam {
        plugin_internal_ctx: core::ptr::null_mut(),
        plugin_internal_ctx_len: 0,
        data_in: core::ptr::null(),
        data_out: core::ptr::null_mut(),
        result: PluginResult::Err
    };
    plugin_call_v2(plugin_name, &mut params, PluginInteractionType::Check);
    Some(plugin_name)
}