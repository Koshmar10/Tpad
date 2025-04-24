use std::fs::metadata;
use std::os::unix::fs::MetadataExt;
use std::os::unix::raw::off_t;
use std::{collections::btree_map::Range, fs};



pub mod session;
pub mod tpad_error;
pub mod data_models;
pub mod ui;
pub mod doc;
pub mod app;
pub use data_models::*;









/*
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn word_found_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test.txt"),
            content: vec![String::from("This is a test document, made for testing purposes only")],
        });
        let result = app.documents[0].find("test");
        assert_eq!(vec!["Word found"], result);
    }
    
    #[test]
    fn word_not_found_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test.txt"),
            content: vec![String::from("This is a test document, made for testing purposes only")],
        });
        let result = app.documents[0].find("mama");
        assert_eq!(vec!["Word not found"], result);
    }
    
    #[test]
    fn open_failed_test() {
        let mut app = App::new(vec![]);
        let result = app.open("nonexistent.txt");
        assert!(result.is_err(), "Expected an error when opening a non-existent file");
    }
    
    #[test]
    fn open_success_test() {
        let mut app = App::new(vec![]);
        // Replace "document.txt" with an actual file path that exists for testing
        let result = app.open("document.txt");
        assert!(result.is_ok(), "File opened successfully");
    }
    
    #[test]
    fn positive_word_count() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test.txt"),
            content: vec![String::from("This is a test document, made for testing purposes only")],
        });
        let result = app.documents[0].word_count("test");
        assert_eq!(vec!["Found 2 matches"], result);
    }
    
    
    #[test]
    fn list_docs_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test1.txt"),
            content: vec![String::from("Content of test1")],
        });
        app.documents.push(Document {
            file_path: String::from("test2.txt"),
            content: vec![String::from("Content of test2")],
        });
        let result = app.list_docs();
        assert_eq!(vec!["test1.txt", "test2.txt"], result);
    }
    
    #[test]
    fn close_document_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test1.txt"),
            content: vec![String::from("Content of test1")],
        });
        app.documents.push(Document {
            file_path: String::from("test2.txt"),
            content: vec![String::from("Content of test2")],
        });
        app.close();
        assert_eq!(app.documents.len(), 1);
        assert_eq!(app.documents[0].file_path, "test1.txt");
    }
    
    #[test]
    fn change_active_document_test() {
        let mut app = App::new(vec![]);
        app.documents.push(Document {
            file_path: String::from("test1.txt"),
            content: vec![String::from("Content of test1")],
        });
        app.documents.push(Document {
            file_path: String::from("test2.txt"),
            content: vec![String::from("Content of test2")],
        });
        app.change(1);
        assert_eq!(app.active, 1);
    }
}
    */