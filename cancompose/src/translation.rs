use anyhow::{Context, Result};
use can::*;

use crate::ymlfmt::*;

impl YDesc {
    pub fn into_network(self) -> Result<CANNetwork> {
        let mut net = CANNetwork::new();

        for (msg_name, msg) in self.messages {
            let mut sigs = Vec::new();

            for (sig_name, sdesc) in msg.signals {
                let new_sig = CANSignal::builder()
                    .name(sig_name.clone())
                    .start_bit(0)
                    .width(sdesc.width)
                    .description(sdesc.description)
                    .scale(sdesc.scale)
                    .build()
                    .context(format!(
                        "Could not create signal `{sig_name}` while building message `{msg_name}`"
                    ))?;

                sigs.push(new_sig);
            }

            let can_msg = CANMessage::builder()
                .name(msg_name.clone())
                .id(msg.id)
                .cycletime_ms(msg.cycletime_ms)
                .signals(sigs)
                .build()
                .context(format!("Could not create message `{msg_name}`"))?;

            net.insert_msg(can_msg)
                .context(format!("Could not insert message `{msg_name}`"))?;
        }
        Ok(net)
    }
}
