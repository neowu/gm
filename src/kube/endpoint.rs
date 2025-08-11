pub struct Endpoint<'a> {
    pub name: &'a str,
    pub ns: &'a str,
    pub ip: &'a str,
}

impl Endpoint<'_> {
    pub fn to_kube_config(&self) -> String {
        let name = self.name;
        let ns = self.ns;
        let ip = self.ip;
        format!(
            r#"apiVersion: v1
kind: Service
metadata:
  name: {name}
  namespace: {ns}
spec:
  clusterIP: None
---
apiVersion: discovery.k8s.io/v1
kind: EndpointSlice
metadata:
  name: {name}
  namespace: {ns}
  labels:
    kubernetes.io/service-name: {name}
addressType: IPv4
endpoints:
- addresses:
  - {ip}"#
        )
    }
}
