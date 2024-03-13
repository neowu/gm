pub struct Endpoint<'a> {
    pub name: &'a str,
    pub ns: &'a str,
    pub ip: &'a str,
}

impl Endpoint<'_> {
    pub fn to_resource(&self) -> String {
        let name = &self.name;
        let ns = &self.ns;
        let ip = &self.ip;
        format!(
            r#"
apiVersion: v1
kind: Service
metadata:
    name: {name}
    namespace: {ns}
spec:
    clusterIP: None
---
apiVersion: v1
kind: Endpoints
metadata:
    name: {name}
    namespace: {ns}
subsets:
- addresses:
    - ip: {ip}        
"#
        )
    }
}
