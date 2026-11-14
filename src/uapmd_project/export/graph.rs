//! Preserve the playback order: layer gain, sum, track effects, track fader.
use super::{Layer, Part, PluginState};
use serde_json::{json, Value};

type Add<'a> = dyn FnMut(String, Vec<u8>) -> Result<(), String> + 'a;

#[derive(Default)]
struct Graph {
    plugins: Vec<Value>,
    nodes: Vec<Value>,
    connections: Vec<Value>,
}

impl Graph {
    fn connect(&mut self, bus: &str, source: &str, target: &str) {
        let endpoint = |id: &str| {
            let kind = match id {
                "graph:input" => "graph_input",
                "graph:output" => "graph_output",
                _ => "plugin",
            };
            json!({"type":kind,"node_id":id,"bus_index":0})
        };
        self.connections
            .push(json!({"id":self.connections.len() + 1,"bus_type":bus,
            "source":endpoint(source),"target":endpoint(target)}));
    }

    fn plugin(
        &mut self,
        id: &str,
        slot: &PluginState,
        file: String,
        group: usize,
        add: &mut Add<'_>,
    ) -> Result<(), String> {
        add(file.clone(), slot.state.clone())?;
        self.plugins
            .push(json!({"node_id":id,"plugin_id":slot.plugin.id,
            "format":slot.plugin.format,"display_name":slot.plugin.name,
            "state_file":file,"group_index":group}));
        Ok(())
    }

    fn layer(
        &mut self,
        index: usize,
        group: usize,
        layer: &Layer,
        add: &mut Add<'_>,
    ) -> Result<String, String> {
        let id = format!("plugin:{group}");
        if let Some(slot) = &layer.state {
            let file = if group == 0 {
                format!("plugin_states/{index}.state")
            } else {
                format!("plugin_states/{index}-{group}.state")
            };
            self.plugin(&id, slot, file, group, add)?;
        } else {
            self.plugins
                .push(json!({"node_id":id,"plugin_id":layer.plugin.id,
                "format":layer.plugin.format,"display_name":layer.plugin.name,
                "state_file":"","group_index":group}));
        }
        self.connect("event", "graph:input", &id);
        let gain = format!("layer:{group}:gain");
        self.nodes
            .push(json!({"node_id":gain,"type":"webaudio:GainNode",
            "parameters":{"gain":if layer.muted {0.0} else {layer.volume}}}));
        self.connect("audio", &id, &gain);
        Ok(gain)
    }

    fn effects(
        &mut self,
        prefix: &str,
        effects: &[PluginState],
        mut sources: Vec<String>,
        add: &mut Add<'_>,
    ) -> Result<(), String> {
        for (index, slot) in effects.iter().enumerate() {
            let id = format!("effect:{index}");
            self.plugin(
                &id,
                slot,
                format!("plugin_states/{prefix}-fx-{index}.state"),
                0,
                add,
            )?;
            // UAPMD's project schema does not persist plugin bypass. Retain the
            // state but route around a bypassed effect, as playback does.
            if slot.bypass {
                continue;
            }
            for source in sources.drain(..) {
                self.connect("audio", &source, &id);
            }
            sources.push(id);
        }
        for source in sources {
            self.connect("audio", &source, "graph:output");
        }
        Ok(())
    }

    fn value(self) -> Value {
        json!({"graph_type":"urn:uapmd-graph:common/graph/dag/v1",
            "plugins":self.plugins,"nodes":self.nodes,"connections":self.connections})
    }
}

pub(super) fn track(index: usize, part: &Part, add: &mut Add<'_>) -> Result<Value, String> {
    let mut graph = Graph::default();
    let sources = part
        .layers
        .iter()
        .enumerate()
        .map(|(group, layer)| graph.layer(index, group, layer, add))
        .collect::<Result<_, _>>()?;
    graph.effects(&index.to_string(), &part.effects, sources, add)?;
    Ok(graph.value())
}

pub(super) fn master(effects: &[PluginState], add: &mut Add<'_>) -> Result<Value, String> {
    let mut graph = Graph::default();
    graph.effects("master", effects, vec!["graph:input".into()], add)?;
    Ok(graph.value())
}
