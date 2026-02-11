use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;

use browsy_core::fetch::{InputPurpose, Session, SessionConfig};
use browsy_core::output::{self, SpatialDom, SpatialElement as CoreElement};

fn convert_err(e: browsy_core::fetch::FetchError) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

fn json_to_py(py: Python<'_>, val: serde_json::Value) -> PyObject {
    match val {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py).unwrap().into_any().unbind()
            } else {
                n.as_f64().unwrap_or(0.0).into_pyobject(py).unwrap().into_any().unbind()
            }
        }
        serde_json::Value::String(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        serde_json::Value::Array(arr) => {
            let items: Vec<PyObject> = arr.into_iter().map(|v| json_to_py(py, v)).collect();
            pyo3::types::PyList::new(py, items).unwrap().into_any().unbind()
        }
        serde_json::Value::Object(map) => {
            let dict = pyo3::types::PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_py(py, v)).unwrap();
            }
            dict.into_any().unbind()
        }
    }
}

// --- Element ---

#[pyclass(frozen)]
#[derive(Clone)]
struct Element {
    inner: CoreElement,
}

#[pymethods]
impl Element {
    #[getter]
    fn id(&self) -> u32 {
        self.inner.id
    }

    #[getter]
    fn tag(&self) -> &str {
        &self.inner.tag
    }

    #[getter]
    fn role(&self) -> Option<&str> {
        self.inner.role.as_deref()
    }

    #[getter]
    fn text(&self) -> Option<&str> {
        self.inner.text.as_deref()
    }

    #[getter]
    fn href(&self) -> Option<&str> {
        self.inner.href.as_deref()
    }

    #[getter]
    fn placeholder(&self) -> Option<&str> {
        self.inner.ph.as_deref()
    }

    #[getter]
    fn value(&self) -> Option<&str> {
        self.inner.val.as_deref()
    }

    #[getter]
    fn input_type(&self) -> Option<&str> {
        self.inner.input_type.as_deref()
    }

    #[getter]
    fn name(&self) -> Option<&str> {
        self.inner.name.as_deref()
    }

    #[getter]
    fn label(&self) -> Option<&str> {
        self.inner.label.as_deref()
    }

    #[getter]
    fn alert_type(&self) -> Option<&str> {
        self.inner.alert_type.as_deref()
    }

    #[getter]
    fn disabled(&self) -> Option<bool> {
        self.inner.disabled
    }

    #[getter]
    fn checked(&self) -> Option<bool> {
        self.inner.checked
    }

    #[getter]
    fn expanded(&self) -> Option<bool> {
        self.inner.expanded
    }

    #[getter]
    fn selected(&self) -> Option<bool> {
        self.inner.selected
    }

    #[getter]
    fn required(&self) -> Option<bool> {
        self.inner.required
    }

    #[getter]
    fn hidden(&self) -> Option<bool> {
        self.inner.hidden
    }

    #[getter]
    fn bounds(&self) -> (i32, i32, i32, i32) {
        (self.inner.b[0], self.inner.b[1], self.inner.b[2], self.inner.b[3])
    }

    fn __repr__(&self) -> String {
        let text = self.inner.text.as_deref().unwrap_or("");
        format!("<Element id={} tag={} text={:?}>", self.inner.id, self.inner.tag, text)
    }
}

// --- Page ---

#[pyclass]
#[derive(Clone)]
struct Page {
    inner: SpatialDom,
}

#[pymethods]
impl Page {
    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }

    #[getter]
    fn url(&self) -> &str {
        &self.inner.url
    }

    #[getter]
    fn elements(&self) -> Vec<Element> {
        self.inner.els.iter().map(|e| Element { inner: e.clone() }).collect()
    }

    fn visible(&self) -> Vec<Element> {
        self.inner.visible().into_iter().map(|e| Element { inner: e.clone() }).collect()
    }

    fn above_fold(&self) -> Vec<Element> {
        self.inner.above_fold().into_iter().map(|e| Element { inner: e.clone() }).collect()
    }

    fn get(&self, id: u32) -> Option<Element> {
        self.inner.get(id).map(|e| Element { inner: e.clone() })
    }

    fn tables(&self) -> Vec<PyObject> {
        Python::with_gil(|py| {
            self.inner.tables().into_iter().map(|t| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("headers", &t.headers).unwrap();
                let rows: Vec<Vec<String>> = t.rows;
                dict.set_item("rows", rows).unwrap();
                dict.into_any().unbind()
            }).collect()
        })
    }

    fn page_type(&self) -> String {
        format!("{:?}", self.inner.page_type)
    }

    fn pagination(&self) -> Option<PyObject> {
        let p = self.inner.pagination()?;
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("next", &p.next).unwrap();
            dict.set_item("prev", &p.prev).unwrap();
            let pages: Vec<(String, String)> = p.pages;
            dict.set_item("pages", pages).unwrap();
            Some(dict.into_any().unbind())
        })
    }

    fn alerts(&self) -> Vec<Element> {
        self.inner.alerts().into_iter().map(|e| Element { inner: e.clone() }).collect()
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn to_compact(&self) -> String {
        output::to_compact_string(&self.inner)
    }

    fn suggested_actions(&self) -> Vec<PyObject> {
        Python::with_gil(|py| {
            self.inner.suggested_actions.iter().map(|a| {
                let val = serde_json::to_value(a).unwrap();
                json_to_py(py, val)
            }).collect()
        })
    }

    fn __len__(&self) -> usize {
        self.inner.els.len()
    }

    fn __repr__(&self) -> String {
        format!("<Page title={:?} url={:?} els={}>", self.inner.title, self.inner.url, self.inner.els.len())
    }
}

// --- Browser ---

#[pyclass]
struct Browser {
    session: Session,
}

#[pymethods]
impl Browser {
    #[new]
    #[pyo3(signature = (viewport_width=1920, viewport_height=1080))]
    fn new(viewport_width: u32, viewport_height: u32) -> PyResult<Self> {
        let config = SessionConfig {
            viewport_width: viewport_width as f32,
            viewport_height: viewport_height as f32,
            ..SessionConfig::default()
        };
        let session = Session::with_config(config).map_err(convert_err)?;
        Ok(Browser { session })
    }

    fn goto(&mut self, url: &str) -> PyResult<Page> {
        let dom = self.session.goto(url).map_err(convert_err)?;
        Ok(Page { inner: dom })
    }

    fn click(&mut self, id: u32) -> PyResult<Page> {
        let dom = self.session.click(id).map_err(convert_err)?;
        Ok(Page { inner: dom })
    }

    fn type_text(&mut self, id: u32, text: &str) -> PyResult<()> {
        self.session.type_text(id, text).map_err(convert_err)
    }

    fn check(&mut self, id: u32) -> PyResult<()> {
        self.session.check(id).map_err(convert_err)
    }

    fn uncheck(&mut self, id: u32) -> PyResult<()> {
        self.session.uncheck(id).map_err(convert_err)
    }

    fn select(&mut self, id: u32, value: &str) -> PyResult<()> {
        self.session.select(id, value).map_err(convert_err)
    }

    fn back(&mut self) -> PyResult<Page> {
        let dom = self.session.back().map_err(convert_err)?;
        Ok(Page { inner: dom })
    }

    fn dom(&self) -> Option<Page> {
        self.session.dom().map(|d| Page { inner: d })
    }

    fn search(&mut self, query: &str) -> PyResult<Vec<PyObject>> {
        let results = self.session.search(query).map_err(convert_err)?;
        Python::with_gil(|py| {
            Ok(results.into_iter().map(|r| {
                let dict = pyo3::types::PyDict::new(py);
                dict.set_item("title", &r.title).unwrap();
                dict.set_item("url", &r.url).unwrap();
                dict.set_item("snippet", &r.snippet).unwrap();
                dict.into_any().unbind()
            }).collect())
        })
    }

    fn find_by_text(&self, text: &str) -> Vec<Element> {
        self.session.find_by_text(text)
            .into_iter()
            .map(|e| Element { inner: e.clone() })
            .collect()
    }

    fn find_by_role(&self, role: &str) -> Vec<Element> {
        self.session.find_by_role(role)
            .into_iter()
            .map(|e| Element { inner: e.clone() })
            .collect()
    }

    fn login(&mut self, username: &str, password: &str) -> PyResult<Page> {
        let dom = self.session.login(username, password).map_err(convert_err)?;
        Ok(Page { inner: dom })
    }

    fn enter_code(&mut self, code: &str) -> PyResult<Page> {
        let dom = self.session.enter_code(code).map_err(convert_err)?;
        Ok(Page { inner: dom })
    }

    fn find_by_text_fuzzy(&self, text: &str) -> Vec<Element> {
        self.session.find_by_text_fuzzy(text)
            .into_iter()
            .map(|e| Element { inner: e.clone() })
            .collect()
    }

    fn find_input_by_purpose(&self, purpose: &str) -> Option<Element> {
        let p = match purpose.to_lowercase().as_str() {
            "password" => InputPurpose::Password,
            "email" => InputPurpose::Email,
            "username" => InputPurpose::Username,
            "verification_code" | "code" | "otp" => InputPurpose::VerificationCode,
            "search" => InputPurpose::Search,
            "phone" | "tel" => InputPurpose::Phone,
            _ => return None,
        };
        self.session.find_input_by_purpose(p).map(|e| Element { inner: e.clone() })
    }

    fn find_verification_code(&self) -> Option<String> {
        self.session.find_verification_code()
    }

    fn load_html(&mut self, html: &str, url: &str) -> PyResult<Page> {
        let dom = self.session.load_html(html, url).map_err(convert_err)?;
        Ok(Page { inner: dom })
    }
}

// --- Module ---

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Browser>()?;
    m.add_class::<Page>()?;
    m.add_class::<Element>()?;
    Ok(())
}
