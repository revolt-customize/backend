use std::collections::HashMap;

auto_derived!(
    /// uuap response
    pub struct UUAPResponse {
        pub code: u32,
        pub msg: String,
        pub data: UUAPResponseData,
    }

    #[serde(untagged)]
    pub enum UUAPResponseData {
        Redirect(String),
        Success {
            cookie: HashMap<String, String>,
            username: String,
            user: UUAPUserInfo,
        },
        Forbidden {
            username: String,
        },
    }

    pub struct UUAPUserInfo {
        pub username: String,
        pub department_name: String,
        pub work_team: String,
    }
);

#[cfg(test)]
#[cfg(feature = "validator")]
mod tests {
    use crate::v0;
    use serde_json::json;

    #[test]
    fn test_uuap() {
        let data1 = json!({
            "code": 302,
            "msg": "redirect login",
            "data": "https://itebeta.baidu.com/login?service=https://botworld-sandbox.now.baidu-int.com/&appKey=uuapclient-872431902769029121-15lch-beta&version=v2"
        });

        let data2 = json!({
            "code": 403,
            "msg": "Forbidden",
            "data": {
                "username": "lixxxx"
            }
        });

        let data3 = json!({
            "code": 200,
            "msg": "",
            "data": {
                "username": "zhangshiju01",
                "cookie": {
                    "UUAP_TRACE_TOKEN": "0338350554b65d8590d68bbf721f8fd0",
                    "jsdk-uuid": "09851bc9-9d75-41d3-aacc-1ec0fa5ac083",
                    "ssousername": "zhangshiju01",
                },
                "user": {
                    "username": "zhangshiju01",
                    "department_name": "企业云平台服务部",
                    "work_team": "混合云平台组"
                }
            }
        });

        let response1: v0::UUAPResponse = serde_json::from_value(data1).unwrap();
        println!("{response1:?}");

        let response2: v0::UUAPResponse = serde_json::from_value(data2).unwrap();
        println!("{response2:?}");

        let response3: v0::UUAPResponse = serde_json::from_value(data3).unwrap();
        println!("{response3:?}");
    }
}
