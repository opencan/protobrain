use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use crate::{CANMessage, CANNetwork, CANSignal, TranslationToOpencan};

pub struct DbcImporter {
    dbc: can_dbc::DBC,
}

impl TranslationToOpencan for DbcImporter {
    fn import_network(input: String) -> crate::CANNetwork {
        let import = Self {
            dbc: can_dbc::DBC::try_from(input.as_str()).unwrap(),
        };

        dbg!(&import.dbc);

        let mut net = CANNetwork::new();

        // wtf here
        // Add all the nodes to the network
        for node in &import.dbc.nodes().iter().next().unwrap().0 {
            net.add_node(&node).unwrap();
        }

        // Add all the messages in each node to the network
        for dbc_msg in import.dbc.messages() {
            let mut msg = CANMessage::builder()
                .name(dbc_msg.message_name())
                .id(dbc_msg.message_id().0);

            match dbc_msg.transmitter() {
                can_dbc::Transmitter::NodeName(node) => msg = msg.tx_node(node),
                can_dbc::Transmitter::VectorXXX => todo!("support for anonymous tx node"),
            }

            let mut opencan_signals: Vec<_> = dbc_msg
                .signals()
                .iter()
                .map(|dbc_signal| {
                    (
                        dbc_signal.start_bit as u32,
                        import.translate_signal(dbc_msg, dbc_signal),
                    )
                })
                .collect();

            opencan_signals.sort_by_key(|s| s.0);

            msg = msg.add_signals_fixed(opencan_signals).unwrap();

            net.insert_msg(msg.build().unwrap()).unwrap();
        }

        dbg!(&net);

        net
    }
}

impl DbcImporter {
    fn translate_signal(
        &self,
        dbc_msg: &can_dbc::Message,
        dbc_signal: &can_dbc::Signal,
    ) -> CANSignal {
        let mut sig = CANSignal::builder()
            .name(dbc_signal.name())
            .width(dbc_signal.signal_size as _);

        // twos complement?
        if matches!(dbc_signal.value_type(), can_dbc::ValueType::Signed) {
            sig = sig.twos_complement(true);
        }

        // emumerated values
        if let Some(d) = self
            .dbc
            .value_descriptions_for_signal(*dbc_msg.message_id(), dbc_signal.name())
        {
            let mut enumerated_values: Vec<(String, u64)> = Vec::new();

            for val_desc in d {
                // sig = sig
                //     .add_enumerated_value(val_desc.b(), val_desc.a().try_into().unwrap())
                //     .unwrap();

                // unfortunately some people do insane things with their value descriptions.
                // we are going to normalize these names and prevent collisions.
                let name = val_desc.b();

                // map naughty characters to _
                let normalized_name: String = name
                    .to_ascii_uppercase()
                    .chars()
                    .map(|c| match c {
                        'A'..='Z' | '0'..='9' => c,
                        _ => '_',
                    })
                    .collect();

                // trim trailing/leading '_'
                let normalized_name = normalized_name.trim_matches('_');

                // get the value
                let value = val_desc.a();
                if value.fract() != 0.0 {
                    panic!("Expected integer value description!");
                }

                // push
                enumerated_values.push((normalized_name.into(), *value as _));
            }

            // find duplicate names
            let mut occurences: HashMap<String, u64> = HashMap::new();
            for val in &enumerated_values {
                *occurences.entry(val.0.clone()).or_insert(0) += 1;
            }

            // actually add the enumerated values
            for val in enumerated_values {
                let name = if occurences[&val.0] > 1 {
                    // making unique names if there was more than one occurrence
                    format!("{}_{}", val.1, &val.0)
                } else {
                    val.0
                };

                let val = val.1;

                sig = sig.add_enumerated_value(&name, val).unwrap();
            }
        }

        sig.build().unwrap()
    }
}
