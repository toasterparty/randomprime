use structs::{
    Area, AreaLayerFlags, Dependency, MemoryRelayConn, Mlvl, Mrea, SclyLayer, SclyObject, Resource,
    ResourceListCursor
};
use reader_writer::{CStr, CStrConversionExtension, FourCC, LazyArray};


use std::collections::HashMap;

pub struct MlvlEditor<'r>
{
    pub mlvl: Mlvl<'r>,
}

pub struct MlvlArea<'r, 'mlvl, 'cursor, 'list>
{
    pub mrea_index: usize,
    pub mrea_cursor: &'cursor mut ResourceListCursor<'r, 'list>,
    pub mlvl_area: &'mlvl mut Area<'r>,
    pub layer_flags: &'mlvl mut AreaLayerFlags,
    pub layer_names: &'mlvl mut Vec<CStr<'r>>,
    pub memory_relay_conns: &'mlvl mut LazyArray<'r, MemoryRelayConn>,
    last_assigned_object_id: u32,
}

impl<'r> MlvlEditor<'r>
{
    pub fn new(mlvl: Mlvl<'r>) -> MlvlEditor<'r>
    {
        MlvlEditor { mlvl }
    }

    pub fn get_area<'s, 'cursor, 'list: 'cursor>(
        &'s mut self,
        mrea_cursor: &'cursor mut ResourceListCursor<'r, 'list>
    )
        -> MlvlArea<'r, 's, 'cursor, 'list>
    {
        assert_eq!(mrea_cursor.peek().unwrap().fourcc(), b"MREA".into());
        let file_id = mrea_cursor.peek().unwrap().file_id;
        let (i, area) = self.mlvl.areas.iter_mut()
            .enumerate()
            .find(|&(_, ref a)| a.mrea == file_id)
            .unwrap();
        MlvlArea {
            mrea_index: i,
            mrea_cursor,
            mlvl_area: area,
            layer_flags: self.mlvl.area_layer_flags.as_mut_vec().get_mut(i).unwrap(),
            layer_names: self.mlvl.area_layer_names.mut_names_for_area(i).unwrap(),
            memory_relay_conns: &mut self.mlvl.memory_relay_conns,
            last_assigned_object_id: 0,
        }
    }
}

impl<'r, 'mlvl, 'cursor, 'list> MlvlArea<'r, 'mlvl, 'cursor, 'list>
{
    pub fn mrea_file_id(&mut self) -> u32
    {
        self.mrea_cursor.peek().unwrap().file_id
    }

    pub fn mrea(&mut self) -> &mut Mrea<'r>
    {
        let x = self.mrea_cursor.value().unwrap();
        x.kind.as_mrea_mut().unwrap()
    }

    pub fn get_layer_id_from_name(&mut self, layer_name: &str) -> usize
    {
        let layer_name_nul = format!("{}\0", layer_name);
        let c_layer_name = (*(&layer_name_nul[..])).as_bytes().as_cstr();
        let mut i: i32 = 0;
        let mut layer_id: i32 = -1;
        for l_name in self.layer_names.iter_mut() {
            if (*l_name).eq(&(c_layer_name.to_owned())) {
                layer_id = i;
                break;
            }
            i += 1;
        }

        if layer_id < 0 {
            panic!("Layer {} doesn't exist", layer_name);
        }

        layer_id as usize
    }

    pub fn new_object_id_from_layer_name(&mut self, layer_name: &str) -> u32
    {
        let layer_id = self.get_layer_id_from_name(layer_name) as u32;
        return self.new_object_id_from_layer_id(layer_id);
    }

    pub fn new_object_id_from_layer_id(&mut self, layer_id: u32) -> u32
    {
        let mut new_obj_id: u32 = self.last_assigned_object_id;
        if new_obj_id == 0 {
            // search for the last object id
            for layer in self.mrea().scly_section_mut().layers.iter_mut() {
                for obj in layer.objects.iter_mut() {
                    if obj.instance_id & 0xffff > new_obj_id {
                        new_obj_id = obj.instance_id & 0xffff
                    }
                }
            }
        }

        // add one to the last object id so it's using a free slot
        new_obj_id += 1;

        // add the area id to the object id
        new_obj_id |= (self.mrea_index as u32) << 16;

        // add the layer id to the object id
        new_obj_id |= layer_id << 26;

        self.last_assigned_object_id = new_obj_id & 0xffffff;

        new_obj_id
    }

    pub fn set_memory_relay_active(&mut self, mem_relay_id: u32, active: u8)
    {
        let layer_id = ((mem_relay_id >> 26) & 0x1f) as usize;

        let layers = self.mrea()
                         .scly_section_mut()
                         .layers
                         .as_mut_vec();

        if layers[layer_id].objects.iter_mut().find(|obj| obj.instance_id == mem_relay_id).is_none() {
            panic!("[set_memory_relay_active] mem_relay doesn't exist! (ID : {:X})", mem_relay_id);
        }

        layers[layer_id].objects
                        .iter_mut()
                        .find(|obj| obj.instance_id == mem_relay_id)
                        .and_then(|obj| obj.property_data.as_memory_relay_mut())
                        .unwrap()
                        .active = active;

        for mem_relay in self.memory_relay_conns.iter_mut() {
            if mem_relay.sender_id == mem_relay_id {
                mem_relay.active = active;
            }
        }
    }

    pub fn add_memory_relay(&mut self, mem_relay: SclyObject<'r>)
    {
        let layer_id = ((mem_relay.instance_id >> 26) & 0x1f) as usize;
        let active = mem_relay.property_data.as_memory_relay().unwrap().active;

        if !mem_relay.property_data.is_memory_relay() {
            panic!("[add_memory_relay] mem_relay is not a memory relay object! (ID : {:X})", mem_relay.instance_id);
        }

        let layers = self.mrea()
                         .scly_section_mut()
                         .layers
                         .as_mut_vec();

        layers[layer_id].objects
                        .as_mut_vec()
                        .push(mem_relay.clone());

        for conn in mem_relay.connections.iter() {
            self.memory_relay_conns
                .as_mut_vec()
                .push(MemoryRelayConn {
                    sender_id: mem_relay.instance_id,
                    target_id: conn.target_object_id,
                    message: conn.message.0 as u16,
                    active: active
                });
        }
    }

    pub fn add_layer(&mut self, name: CStr<'r>)
    {
        // Mark this layer as active
        self.layer_flags.flags |= 1 << self.layer_flags.layer_count;
        self.layer_flags.layer_count += 1;
        self.layer_names.push(name);

        {
            let deps = self.mlvl_area.dependencies.deps.as_mut_vec();
            let index = deps.len() - 1;
            deps.insert(index, vec![].into());
        }

        self.mrea().scly_section_mut().layers.as_mut_vec().push(SclyLayer::new());
    }

    pub fn add_dependencies<I>(&mut self, pickup_resources: &HashMap<(u32, FourCC), Resource<'r>>,
                               layer_num: usize, deps: I)
        where I: Iterator<Item=Dependency>,
    {
        let layers = self.mlvl_area.dependencies.deps.as_mut_vec();
        let iter = deps.filter_map(|dep| {
                if layers.iter().all(|layer| layer.iter().all(|i| *i != dep)) {
                    if !pickup_resources.contains_key(&&(dep.asset_id, dep.asset_type)) {
                        panic!("Failed to find dependency in pickup_resources - 0x{:X} ({:?})", dep.asset_id, dep.asset_type);
                    }
                    let res = pickup_resources[&(dep.asset_id, dep.asset_type)].clone();
                    layers[layer_num].as_mut_vec().push(dep);
                    Some(res)
                }  else {
                    None
                }
            });
        self.mrea_cursor.insert_after(iter);
    }
}

