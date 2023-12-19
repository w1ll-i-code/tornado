use maplit::hashset;
use std::collections::{HashMap, HashSet};

pub type HostName = Box<str>;
pub type ServiceName = Box<str>;

#[derive(Default, Debug)]
pub struct IcingaObjectStore {
    store: HashMap<HostName, HashSet<ServiceName>>,
}

impl IcingaObjectStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn exists(&self, name: &str) -> bool {
        let (host, service) = split_once_on(name, '!');

        match (self.store.get(host), service) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(services), Some(service)) => services.contains(service),
        }
    }

    pub fn exists_service(&self, host: &str, service: &str) -> bool {
        self.store
            .get(host)
            .map(|service_set| service_set.contains(service))
            .is_some()
    }

    pub fn insert(&mut self, name: String) {
        let (host, service) = split_once_on(&name, '!');

        let service_set = match self.store.get_mut(host) {
            None => self
                .store
                .entry(host.to_owned().into())
                .or_insert(hashset! {}),
            Some(service_set) => service_set,
        };

        if let Some(service) = service {
            if !service_set.contains(service) {
                service_set.insert(service.to_owned().into());
            }
        }
    }

    pub fn delete(&mut self, name: &str) {
        let (host, service) = split_once_on(name, '!');

        match service {
            None => {
                self.store.remove(host);
            }
            Some(service) => {
                if let Some(services) = self.store.get_mut(host) {
                    services.remove(service);
                }
            }
        }
    }
}

fn split_once_on(string: &str, split: char) -> (&str, Option<&str>) {
    match string.find(split) {
        Some(pos) => (&string[..pos], Some(&string[pos + 1..])),
        None => (string, None),
    }
}
