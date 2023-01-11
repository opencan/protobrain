use std::fmt::Display;

use anyhow::{Context, Result};
use clap::Parser;
use indoc::formatdoc;
use opencan_core::{CANMessage, CANNetwork, CANSignal};
use textwrap::indent;

#[derive(Parser)]
pub struct Args {
    pub node: String,
    pub in_file: String,
}

pub struct Codegen {
    args: Args,
    net: CANNetwork,
    time: chrono::DateTime<chrono::Utc>,
}

pub enum CSignalTy {
    U8,
    U16,
    U32,
    U64,
    Float,
}

impl Display for CSignalTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::U8 => "uint8_t",
                Self::U16 => "uint16_t",
                Self::U32 => "uint32_t",
                Self::U64 => "uint64_t",
                Self::Float => "float", // todo: use a typedef?
            }
        )
    }
}

trait Indent {
    fn indent(&mut self, n: usize) -> String;
}

impl Indent for String {
    fn indent(&mut self, n: usize) -> String {
        indent(self, &" ".repeat(n))
    }
}

impl Indent for &str {
    fn indent(&mut self, n: usize) -> String {
        indent(self, &" ".repeat(n))
    }
}

trait MessageCodegen {
    fn struct_ty(&self) -> String;
    fn struct_def(&self) -> String;
    fn global_struct_ident(&self) -> String;

    fn raw_struct_ty(&self) -> String;
    fn raw_struct_def(&self) -> String;
    fn global_raw_struct_ident(&self) -> String;

    fn decode_fn_name(&self) -> String;
    fn decode_fn_def(&self) -> String;

    fn getter_fn_defs(&self) -> String;
}

impl MessageCodegen for CANMessage {
    fn struct_ty(&self) -> String {
        format!("struct CAN_Message_{}", self.name)
    }

    fn struct_def(&self) -> String {
        let mut top = String::new();
        let mut inner = String::new(); // struct contents

        top += &format!("{} {{", self.struct_ty());

        for sigbit in &self.signals {
            inner += "\n";
            inner += &formatdoc! {"
                /**
                 * -- Signal: {name}
                 *
                 * ----> Description: {desc}
                 * ----> Start bit: {start}
                 * ----> Width: {width}
                 */
                {sigty} {name};
                ",
                name = sigbit.sig.name,
                desc = sigbit.sig.description.as_ref().unwrap_or(&"(None)".into()),
                start = sigbit.bit,
                width = sigbit.sig.width,
                sigty = sigbit.sig.c_ty_decoded(),
            };
        }

        top += &inner.indent(4);
        top += "};";

        top
    }

    fn global_struct_ident(&self) -> String {
        format!("CANRX_Message_{}", self.name)
    }

    fn raw_struct_ty(&self) -> String {
        format!("struct CAN_MessageRaw_{}", self.name)
    }

    fn raw_struct_def(&self) -> String {
        let mut top = String::new();
        let mut inner = String::new(); // struct contents

        top += &format!("{} {{", self.raw_struct_ty());

        for sigbit in &self.signals {
            inner += "\n";
            inner += &formatdoc! {"
                /**
                 * -- Raw signal: {name}
                 *
                 * ----> Description: {desc}
                 * ----> Start bit: {start}
                 * ----> Width: {width}
                 */
                {sigty} {name};
                ",
                name = sigbit.sig.name,
                desc = sigbit.sig.description.as_ref().unwrap_or(&"(None)".into()),
                start = sigbit.bit,
                width = sigbit.sig.width,
                sigty = sigbit.sig.c_ty_raw(),
            };
        }

        top += &inner.indent(4);
        top += "};";

        top
    }

    fn global_raw_struct_ident(&self) -> String {
        format!("CANRX_MessageRaw_{}", self.name)
    }

    fn decode_fn_name(&self) -> String {
        format!("CANRX_decode_{}", self.name)
    }

    fn decode_fn_def(&self) -> String {
        /* function comment */
        let comment = formatdoc! {"
            /**
             * Unpacks and decodes message `{}` from raw data.
             *
             * @param data - Input raw data array
             * @param len  - Length of data (must be {} for this function),
             * @param out  - Pointer to output struct
             *
             * @return     - boolean indicating whether decoding was done (len was correct)
             */",
            self.name,
            self.length
        };

        /* arguments */
        let args = formatdoc! {"
            const uint8_t * const data,
            const uint_fast8_t len"
        };

        /* length condition check */
        let length_cond = formatdoc! {"
            /*  Check that data length is correct  */
            if (len != {}U) {{
                return false;
            }}",
            self.length
        };

        /* unpacking */
        let unpack_start = formatdoc! {"
            /*  Unpack signals  */
            {rawty} raw = {{0}};",
            rawty = self.raw_struct_ty()
        };

        let mut unpack = String::new();

        for sigbit in &self.signals {
            let sig = &sigbit.sig;
            let bit = sigbit.bit;

            unpack += &format!(
                "// Unpack `{}`, start bit {}, width {}\n",
                sig.name, bit, sig.width
            );
        }

        unpack = unpack.trim().into();

        /* set global variables */
        let set_global = formatdoc! {"
            /* Set global data. */
            {global_raw} = raw;",
            global_raw = self.global_raw_struct_ident()
        };

        /* stitch it all together */
        let body = formatdoc! {"
            {length_cond}

            {unpack_start}

            {unpack}

            {set_global}"
        }
        .indent(4);

        formatdoc! {"
            {comment}
            bool {}(\n{args})\n{{
            {body}
            }}",
            self.decode_fn_name(),
        }
    }

    fn getter_fn_defs(&self) -> String {
        let mut getters = String::new();

        for sigbit in &self.signals {
            let sig = &sigbit.sig;
            getters += &formatdoc! {"\n
                {sigty_dec} {fn_name}(void) {{
                    return {global_decoded}.{name};
                }}

                {sigty_raw} {fn_name_raw}(void) {{
                    return {global_raw}.{name};
                }}",
                name = sig.name,
                sigty_dec = sig.c_ty_decoded(),
                sigty_raw = sig.c_ty_raw(),
                global_decoded = self.global_struct_ident(),
                global_raw = self.global_raw_struct_ident(),
                fn_name = sig.getter_fn_name(),
                fn_name_raw = sig.raw_getter_fn_name(),
            }
        }

        getters.trim().into()
    }
}

trait SignalCodegen {
    fn c_ty_raw(&self) -> CSignalTy;
    fn c_ty_decoded(&self) -> CSignalTy;

    fn getter_fn_name(&self) -> String;
    fn raw_getter_fn_name(&self) -> String;
}

impl SignalCodegen for CANSignal {
    fn c_ty_raw(&self) -> CSignalTy {
        match self.width {
            1..=8 => CSignalTy::U8,
            9..=16 => CSignalTy::U16,
            17..=32 => CSignalTy::U32,
            33..=64 => CSignalTy::U64,
            w => panic!(
                "Unexpectedly wide signal: `{}` is `{}` bits wide",
                self.name, w
            ),
        }
    }

    /// Get the C type for the decoded signal.
    ///
    /// This does not take into account minimum/maximum capping - that is, this
    /// gives the type for the entire _representable_ decoded range, not just
    /// what's within the minimum/maximum additional bounds.
    fn c_ty_decoded(&self) -> CSignalTy {
        // todo: complete integer signal bounds support
        // should we make this implicit or explicit... hmmm...
        // making it implicit (i.e. say 1 instead of 1.0) might be obtuse / ambiguous
        // -> otoh, saying force_integer: yes or force_float: yes all the time is annnoying

        // I think I lean implicit. The problem is then it becomes a nightmare in Rust code....

        // for now, if the signal has no offset or scale, then return its raw type, else float.
        if self.scale.is_none() && self.offset.is_none() {
            self.c_ty_raw()
        } else {
            CSignalTy::Float
        }
    }

    fn getter_fn_name(&self) -> String {
        format!("CANRX_get_{}", self.name)
    }

    fn raw_getter_fn_name(&self) -> String {
        format!("CANRX_getRaw_{}", self.name)
    }
}

impl Codegen {
    const DECODE_FN_PTR_TYPEDEF: &str = "decode_fn_ptr";
    const ID_TO_DECODE_FN: &str = "CANRX_id_to_decode_fn";

    pub fn new(args: Args, net: CANNetwork) -> Self {
        Self {
            args,
            net,
            time: chrono::Utc::now(),
        }
    }

    pub fn network_to_c(&self) -> Result<String> {
        let mut output = String::new();

        self.net
            .node_by_name(&self.args.node)
            .context(format!("Node `{}` not found in network.", self.args.node))?;

        output += &formatdoc! {"
            {greet}

            {pre_defs}
            ",
            greet = self.internal_prelude_greeting(),
            pre_defs = Self::internal_prelude_defs(),
        };

        for msg in self.sorted_messages() {
            output += "\n";
            output += &formatdoc! {"
                /*********************************************************/
                /* Message: {name} */
                /*********************************************************/

                /*** Message Structs ***/

                {mstruct_raw}
                static {mstruct_raw_name} {global_ident_raw};

                {mstruct}
                static {mstruct_name} {global_ident};

                /*** Signal Getters ***/

                {getters}

                /*** Decode Function ***/

                {decode_fn}
                ",
                name = msg.name,
                mstruct_raw = msg.raw_struct_def(),
                mstruct_raw_name = msg.raw_struct_ty(),
                global_ident_raw = msg.global_raw_struct_ident(),
                mstruct = msg.struct_def(),
                mstruct_name = msg.struct_ty(),
                global_ident = msg.global_struct_ident(),
                getters = msg.getter_fn_defs(),
                decode_fn = msg.decode_fn_def(),
            }
        }

        output += "\n";
        output += &self.rx_id_to_decode_fn();

        Ok(output)
    }

    fn internal_prelude_defs() -> String {
        formatdoc! {"
            #include <stdbool.h>
            #include <stddef.h>
            #include <stdint.h>

            typedef bool (*{})(const uint8_t * const data, const uint_fast8_t len);
            ",
            Self::DECODE_FN_PTR_TYPEDEF,
        }
    }

    fn internal_prelude_greeting(&self) -> String {
        formatdoc! {"
            /**
             * OpenCAN CAN C Codegen - opencan_generated.c
             *
             * Node: {}
             *
             * spdx-license-identifier: MPL-2.0
             *
             * Generated by {} v{} at {}
             */
            ",
            self.args.node,
            clap::crate_name!(),
            clap::crate_version!(),
            self.time.format("%a %b %d, %T %Y %Z")
        }
    }

    fn rx_id_to_decode_fn(&self) -> String {
        let mut cases = String::new();

        for msg in self.sorted_messages() {
            cases += &formatdoc! {"
                case 0x{:X}: return {};
                ",
                msg.id,
                msg.decode_fn_name()
            };
        }

        formatdoc! {"
            {dec_ptr} {name}(const uint32_t id)
            {{
                switch (id) {{
            {cases}
                    default:
                        return NULL;
                }}
            }}
            ",
            dec_ptr = Self::DECODE_FN_PTR_TYPEDEF,
            name = Self::ID_TO_DECODE_FN,
            cases = cases.trim().indent(8),
        }
    }

    /// Get messages for our node sorted by ID
    fn sorted_messages(&self) -> Vec<&CANMessage> {
        let mut messages = self.net.messages_by_node(&self.args.node).unwrap();

        messages.sort_by(|m1, m2| m1.id.cmp(&m2.id));

        messages
    }
}
