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
  - {ip}
---
apiVersion: v1
kind: Endpoints
metadata:
  name: {name}
  namespace: {ns}
subsets:
- addresses:
  - ip: {ip}"#
        )
    }
}

mod test {
    use crate::kube::endpoint::Endpoint;

    #[test]
    fn to_kube_config() {
        let endpoint = Endpoint {
            name: "db",
            ns: "app",
            ip: "10.10.0.1",
        };

        assert_eq!(
            endpoint.to_kube_config(),
            r#"apiVersion: v1
kind: Service
metadata:
  name: db
  namespace: app
spec:
  clusterIP: None
---
apiVersion: discovery.k8s.io/v1
kind: EndpointSlice
metadata:
  name: db
  namespace: app
  labels:
    kubernetes.io/service-name: db
addressType: IPv4
endpoints:
- addresses:
  - 10.10.0.1
---
apiVersion: v1
kind: Endpoints
metadata:
  name: db
  namespace: app
subsets:
- addresses:
  - ip: 10.10.0.1"#
        );
    }
}
