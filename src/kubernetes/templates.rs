use std::collections::HashMap;
use std::env;
use kube::api::{DeleteParams, PostParams};
use kube::Error;
use serde_json::{json, Value};
use crate::Kubernetes;

impl Kubernetes {
    pub async fn create_pod(&self, kind: &str, image: &str, name: &str, properties: HashMap<String, String>) -> Result<(), Error> {
        let adress = env::var("SKYNET_EXTERNAL_ADDRESS").unwrap_or("http://skynet.skynet:8080".to_string());
        let amqp_adress = env::var("AMQP_ADDRESS").unwrap();
        let mut value = json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": name,
                "labels": {
                    "managed_by":"skynet",
                    "skynet/kind": kind,
                }
            },
            "spec": {
                "imagePullSecrets":[
                    {
                        "name": "aspaku-registry"
                    }
                ],
                "containers": [
                    {
                        "name": "minecraft",
                        "image": image,

                        "env": [
                            {
                              "name":"SKYNET_URL",
                              "value": adress
                            },
                            {
                              "name":"AMQP_ADDRESS",
                              "value": amqp_adress
                            },
                        ],
                        "ports": [
                            {
                                "containerPort": 25665
                            }
                        ]
                    }
                ]
            }
        });
        for (k, v) in properties {
            value["metadata"]["labels"][format!("skynet-prop/{}", k)] = Value::String(v);
        }

        let pod = serde_json::from_value(value).unwrap();

        self.pod_api.create(&PostParams::default(), &pod).await?;
        Ok(())
    }

    pub async fn delete_pod(&self, name: &str) -> Result<(), Error> {
        self.pod_api.delete(name, &DeleteParams::default()).await?;
        Ok(())
    }
}