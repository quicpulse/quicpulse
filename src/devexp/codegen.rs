//! Code snippet generation for various languages
//!
//! Generates equivalent code in Python, Node.js, Go, Java, PHP, Rust, and Ruby.

use crate::cli::Args;
use crate::cli::parser::ProcessedArgs;
use crate::input::InputItem;

/// Supported languages for code generation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Language {
    Python,
    Node,
    Go,
    Java,
    Php,
    Rust,
    Ruby,
    Csharp,
}

impl Language {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Some(Language::Python),
            "node" | "nodejs" | "js" | "javascript" => Some(Language::Node),
            "go" | "golang" => Some(Language::Go),
            "java" => Some(Language::Java),
            "php" => Some(Language::Php),
            "rust" | "rs" => Some(Language::Rust),
            "ruby" | "rb" => Some(Language::Ruby),
            "csharp" | "c#" | "cs" => Some(Language::Csharp),
            _ => None,
        }
    }
}

/// Generate code snippet for the specified language
pub fn generate_code(language: &str, args: &Args, processed: &ProcessedArgs) -> Result<String, String> {
    let lang = Language::from_str(language)
        .ok_or_else(|| format!(
            "Unknown language '{}'. Supported: python, node, go, java, php, rust, ruby, csharp",
            language
        ))?;

    Ok(match lang {
        Language::Python => generate_python(args, processed),
        Language::Node => generate_node(args, processed),
        Language::Go => generate_go(args, processed),
        Language::Java => generate_java(args, processed),
        Language::Php => generate_php(args, processed),
        Language::Rust => generate_rust(args, processed),
        Language::Ruby => generate_ruby(args, processed),
        Language::Csharp => generate_csharp(args, processed),
    })
}

/// Extract headers from processed args
fn get_headers(processed: &ProcessedArgs) -> Vec<(String, String)> {
    processed.items.iter()
        .filter_map(|item| {
            match item {
                InputItem::Header { name, value } => Some((name.clone(), value.clone())),
                InputItem::HeaderFile { name, path } => {
                    std::fs::read_to_string(path).ok().map(|v| (name.clone(), v.trim().to_string()))
                }
                _ => None,
            }
        })
        .collect()
}

/// Build request body from processed args
fn build_body(args: &Args, processed: &ProcessedArgs) -> Option<String> {
    if let Some(ref raw) = args.raw {
        return Some(raw.clone());
    }

    let data_items: Vec<_> = processed.items.iter()
        .filter(|i| i.is_data())
        .collect();

    if data_items.is_empty() {
        return None;
    }

    if args.form {
        let pairs: Vec<String> = data_items.iter()
            .filter_map(|item| {
                match item {
                    InputItem::DataField { key, value } => Some(format!("{}={}", key, value)),
                    _ => None,
                }
            })
            .collect();
        Some(pairs.join("&"))
    } else {
        // JSON body
        let mut obj = serde_json::Map::new();
        for item in data_items {
            let (key, value) = match item {
                InputItem::DataField { key, value } => {
                    (key.clone(), serde_json::Value::String(value.clone()))
                }
                InputItem::JsonField { key, value } => {
                    (key.clone(), value.clone())
                }
                _ => continue,
            };
            obj.insert(key, value);
        }
        Some(serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap_or_default())
    }
}

/// Generate Python code (using requests library)
fn generate_python(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("import requests\n\n");

    // URL
    code.push_str(&format!("url = \"{}\"\n", processed.url));

    // Headers
    if !headers.is_empty() {
        code.push_str("headers = {\n");
        for (name, value) in &headers {
            code.push_str(&format!("    \"{}\": \"{}\",\n", name, escape_string(value)));
        }
        code.push_str("}\n");
    }

    // Body
    if let Some(ref body_str) = body {
        if args.form {
            code.push_str(&format!("data = \"{}\"\n", escape_string(body_str)));
        } else {
            code.push_str(&format!("json_data = {}\n", body_str));
        }
    }

    // Request
    code.push_str("\nresponse = requests.");
    code.push_str(&processed.method.to_lowercase());
    code.push_str("(\n    url");

    if !headers.is_empty() {
        code.push_str(",\n    headers=headers");
    }

    if body.is_some() {
        if args.form {
            code.push_str(",\n    data=data");
        } else {
            code.push_str(",\n    json=json_data");
        }
    }

    if let Some(ref auth) = args.auth {
        if matches!(args.auth_type, Some(crate::cli::args::AuthType::Bearer)) {
            // Already handled in headers
        } else {
            let parts: Vec<&str> = auth.splitn(2, ':').collect();
            if parts.len() == 2 {
                code.push_str(&format!(",\n    auth=(\"{}\", \"{}\")", parts[0], parts[1]));
            }
        }
    }

    if let Some(timeout) = args.timeout {
        code.push_str(&format!(",\n    timeout={}", timeout));
    }

    code.push_str("\n)\n\n");
    code.push_str("print(response.status_code)\n");
    code.push_str("print(response.json())\n");

    code
}

/// Generate Node.js code (using fetch)
fn generate_node(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("// Node.js (using fetch)\n\n");

    code.push_str(&format!("const url = '{}';\n\n", processed.url));

    code.push_str("const options = {\n");
    code.push_str(&format!("  method: '{}',\n", processed.method));

    code.push_str("  headers: {\n");
    if !args.form {
        code.push_str("    'Content-Type': 'application/json',\n");
    }
    for (name, value) in &headers {
        code.push_str(&format!("    '{}': '{}',\n", name, escape_string(value)));
    }
    code.push_str("  },\n");

    if let Some(ref body_str) = body {
        if args.form {
            code.push_str(&format!("  body: '{}',\n", escape_string(body_str)));
        } else {
            code.push_str(&format!("  body: JSON.stringify({}),\n", body_str));
        }
    }

    code.push_str("};\n\n");

    code.push_str("fetch(url, options)\n");
    code.push_str("  .then(response => response.json())\n");
    code.push_str("  .then(data => console.log(data))\n");
    code.push_str("  .catch(error => console.error('Error:', error));\n");

    code
}

/// Generate Go code
fn generate_go(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("package main\n\n");
    code.push_str("import (\n");
    code.push_str("\t\"bytes\"\n");
    code.push_str("\t\"fmt\"\n");
    code.push_str("\t\"io\"\n");
    code.push_str("\t\"net/http\"\n");
    code.push_str(")\n\n");

    code.push_str("func main() {\n");
    code.push_str(&format!("\turl := \"{}\"\n\n", processed.url));

    if let Some(ref body_str) = body {
        code.push_str(&format!("\tpayload := bytes.NewBufferString(`{}`)\n\n", body_str));
        code.push_str(&format!("\treq, err := http.NewRequest(\"{}\", url, payload)\n", processed.method));
    } else {
        code.push_str(&format!("\treq, err := http.NewRequest(\"{}\", url, nil)\n", processed.method));
    }

    code.push_str("\tif err != nil {\n");
    code.push_str("\t\tpanic(err)\n");
    code.push_str("\t}\n\n");

    if !args.form {
        code.push_str("\treq.Header.Set(\"Content-Type\", \"application/json\")\n");
    }
    for (name, value) in &headers {
        code.push_str(&format!("\treq.Header.Set(\"{}\", \"{}\")\n", name, escape_string(value)));
    }

    code.push_str("\n\tclient := &http.Client{}\n");
    code.push_str("\tresp, err := client.Do(req)\n");
    code.push_str("\tif err != nil {\n");
    code.push_str("\t\tpanic(err)\n");
    code.push_str("\t}\n");
    code.push_str("\tdefer resp.Body.Close()\n\n");

    code.push_str("\tbody, _ := io.ReadAll(resp.Body)\n");
    code.push_str("\tfmt.Println(string(body))\n");
    code.push_str("}\n");

    code
}

/// Generate Java code
fn generate_java(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("import java.net.http.*;\n");
    code.push_str("import java.net.URI;\n\n");

    code.push_str("public class HttpRequest {\n");
    code.push_str("    public static void main(String[] args) throws Exception {\n");
    code.push_str("        HttpClient client = HttpClient.newHttpClient();\n\n");

    code.push_str("        HttpRequest.Builder requestBuilder = HttpRequest.newBuilder()\n");
    code.push_str(&format!("            .uri(URI.create(\"{}\"))\n", processed.url));

    if let Some(ref body_str) = body {
        code.push_str(&format!(
            "            .method(\"{}\", HttpRequest.BodyPublishers.ofString(\"{}\"))\n",
            processed.method,
            escape_java_string(body_str)
        ));
    } else {
        code.push_str(&format!(
            "            .method(\"{}\", HttpRequest.BodyPublishers.noBody())\n",
            processed.method
        ));
    }

    if !args.form {
        code.push_str("            .header(\"Content-Type\", \"application/json\")\n");
    }
    for (name, value) in &headers {
        code.push_str(&format!("            .header(\"{}\", \"{}\")\n", name, escape_java_string(value)));
    }

    code.push_str("            .build();\n\n");

    code.push_str("        HttpResponse<String> response = client.send(requestBuilder,\n");
    code.push_str("            HttpResponse.BodyHandlers.ofString());\n\n");

    code.push_str("        System.out.println(response.statusCode());\n");
    code.push_str("        System.out.println(response.body());\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    code
}

/// Generate PHP code
fn generate_php(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("<?php\n\n");

    code.push_str("$curl = curl_init();\n\n");

    code.push_str("curl_setopt_array($curl, [\n");
    code.push_str(&format!("    CURLOPT_URL => \"{}\",\n", processed.url));
    code.push_str("    CURLOPT_RETURNTRANSFER => true,\n");
    code.push_str(&format!("    CURLOPT_CUSTOMREQUEST => \"{}\",\n", processed.method));

    let mut header_array = Vec::new();
    if !args.form {
        header_array.push("\"Content-Type: application/json\"".to_string());
    }
    for (name, value) in &headers {
        header_array.push(format!("\"{}: {}\"", name, escape_string(value)));
    }

    if !header_array.is_empty() {
        code.push_str(&format!("    CURLOPT_HTTPHEADER => [{}],\n", header_array.join(", ")));
    }

    if let Some(ref body_str) = body {
        code.push_str(&format!("    CURLOPT_POSTFIELDS => '{}',\n", escape_string(body_str)));
    }

    code.push_str("]);\n\n");

    code.push_str("$response = curl_exec($curl);\n");
    code.push_str("$httpCode = curl_getinfo($curl, CURLINFO_HTTP_CODE);\n");
    code.push_str("curl_close($curl);\n\n");

    code.push_str("echo \"Status: $httpCode\\n\";\n");
    code.push_str("echo $response;\n");

    code
}

/// Generate Rust code (using reqwest)
fn generate_rust(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("use reqwest;\n\n");
    code.push_str("#[tokio::main]\n");
    code.push_str("async fn main() -> Result<(), Box<dyn std::error::Error>> {\n");
    code.push_str("    let client = reqwest::Client::new();\n\n");

    code.push_str(&format!(
        "    let response = client.{}(\"{}\")\n",
        processed.method.to_lowercase(),
        processed.url
    ));

    for (name, value) in &headers {
        code.push_str(&format!("        .header(\"{}\", \"{}\")\n", name, escape_string(value)));
    }

    if let Some(ref body_str) = body {
        if args.form {
            code.push_str(&format!("        .body(\"{}\")\n", escape_string(body_str)));
        } else {
            code.push_str(&format!("        .json(&serde_json::json!({}))\n", body_str));
        }
    }

    code.push_str("        .send()\n");
    code.push_str("        .await?;\n\n");

    code.push_str("    println!(\"Status: {}\", response.status());\n");
    code.push_str("    println!(\"{}\", response.text().await?);\n\n");
    code.push_str("    Ok(())\n");
    code.push_str("}\n");

    code
}

/// Generate Ruby code
fn generate_ruby(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("require 'net/http'\n");
    code.push_str("require 'uri'\n");
    code.push_str("require 'json'\n\n");

    code.push_str(&format!("uri = URI.parse('{}')\n", processed.url));
    code.push_str("http = Net::HTTP.new(uri.host, uri.port)\n");
    code.push_str("http.use_ssl = uri.scheme == 'https'\n\n");

    let method_class = match processed.method.to_uppercase().as_str() {
        "GET" => "Net::HTTP::Get",
        "POST" => "Net::HTTP::Post",
        "PUT" => "Net::HTTP::Put",
        "DELETE" => "Net::HTTP::Delete",
        "PATCH" => "Net::HTTP::Patch",
        _ => "Net::HTTP::Get",
    };

    code.push_str(&format!("request = {}.new(uri.request_uri)\n", method_class));

    if !args.form {
        code.push_str("request['Content-Type'] = 'application/json'\n");
    }
    for (name, value) in &headers {
        code.push_str(&format!("request['{}'] = '{}'\n", name, escape_string(value)));
    }

    if let Some(ref body_str) = body {
        code.push_str(&format!("request.body = '{}'\n", escape_string(body_str)));
    }

    code.push_str("\nresponse = http.request(request)\n");
    code.push_str("puts \"Status: #{response.code}\"\n");
    code.push_str("puts response.body\n");

    code
}

/// Generate C# code
fn generate_csharp(args: &Args, processed: &ProcessedArgs) -> String {
    let headers = get_headers(processed);
    let body = build_body(args, processed);

    let mut code = String::from("using System;\n");
    code.push_str("using System.Net.Http;\n");
    code.push_str("using System.Text;\n");
    code.push_str("using System.Threading.Tasks;\n\n");

    code.push_str("class Program\n{\n");
    code.push_str("    static async Task Main(string[] args)\n    {\n");
    code.push_str("        using var client = new HttpClient();\n\n");

    for (name, value) in &headers {
        code.push_str(&format!(
            "        client.DefaultRequestHeaders.Add(\"{}\", \"{}\");\n",
            name, escape_string(value)
        ));
    }

    if let Some(ref body_str) = body {
        code.push_str(&format!(
            "\n        var content = new StringContent(\"{}\", Encoding.UTF8, \"{}\");\n",
            escape_csharp_string(body_str),
            if args.form { "application/x-www-form-urlencoded" } else { "application/json" }
        ));
    }

    let method = match processed.method.to_uppercase().as_str() {
        "GET" => "GetAsync",
        "POST" => "PostAsync",
        "PUT" => "PutAsync",
        "DELETE" => "DeleteAsync",
        _ => "SendAsync",
    };

    if body.is_some() && method != "GetAsync" && method != "DeleteAsync" {
        code.push_str(&format!(
            "\n        var response = await client.{}(\"{}\", content);\n",
            method, processed.url
        ));
    } else {
        code.push_str(&format!(
            "\n        var response = await client.{}(\"{}\");\n",
            method, processed.url
        ));
    }

    code.push_str("\n        Console.WriteLine($\"Status: {response.StatusCode}\");\n");
    code.push_str("        var body = await response.Content.ReadAsStringAsync();\n");
    code.push_str("        Console.WriteLine(body);\n");
    code.push_str("    }\n}\n");

    code
}

/// Escape special characters for strings
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape for Java strings
fn escape_java_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape for C# strings
fn escape_csharp_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("python"), Some(Language::Python));
        assert_eq!(Language::from_str("py"), Some(Language::Python));
        assert_eq!(Language::from_str("node"), Some(Language::Node));
        assert_eq!(Language::from_str("js"), Some(Language::Node));
        assert_eq!(Language::from_str("go"), Some(Language::Go));
        assert_eq!(Language::from_str("rust"), Some(Language::Rust));
        assert_eq!(Language::from_str("unknown"), None);
    }
}
