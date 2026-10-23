//! Original MVC initialization failures that occur before an HTTP listener exists.
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Cause {
    class: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct Failure {
    exception_class: &'static str,
    message: String,
    bad_request: bool,
    causes: [Cause; 3],
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.exception_class, self.message)
    }
}

pub(crate) fn check() -> Result<(), Failure> {
    teaql_forge_codegen::xml_factory::check_server_configuration().map_err(|error| {
        let root = error.to_string();
        let bean = format!("Failed to instantiate [org.springframework.web.servlet.mvc.method.annotation.RequestMappingHandlerAdapter]: Factory method 'requestMappingHandlerAdapter' threw exception; nested exception is java.lang.IllegalArgumentException: {root}");
        let message = format!("Error creating bean with name 'requestMappingHandlerAdapter' defined in class path resource [org/springframework/boot/autoconfigure/web/servlet/WebMvcAutoConfiguration$EnableWebMvcConfiguration.class]: Bean instantiation via factory method failed; nested exception is org.springframework.beans.BeanInstantiationException: {bean}");
        Failure {
            exception_class: "org.springframework.beans.factory.BeanCreationException",
            message: message.clone(),
            bad_request: false,
            causes: [
                Cause { class: "org.springframework.beans.factory.BeanCreationException", message },
                Cause { class: "org.springframework.beans.BeanInstantiationException", message: bean },
                Cause { class: "java.lang.IllegalArgumentException", message: root },
            ],
        }
    })
}
