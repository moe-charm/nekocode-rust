use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

// Data processor struct
#[derive(Debug)]
pub struct DataProcessor {
    name: String,
    data: Arc<Mutex<Vec<i32>>>,
}

impl DataProcessor {
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn create_default() -> Self {
        Self::new("default".to_string())
    }

    pub async fn process_data(&self, items: Vec<i32>) -> Result<(), Box<dyn std::error::Error>> {
        for item in items {
            self.process_item(item).await?;
        }
        Ok(())
    }

    async fn process_item(&self, item: i32) -> Result<(), Box<dyn std::error::Error>> {
        sleep(Duration::from_millis(1)).await; // Simulate async work
        
        let mut data = self.data.lock().unwrap();
        data.push(item * 2);
        Ok(())
    }

    pub fn get_data(&self) -> Vec<i32> {
        let data = self.data.lock().unwrap();
        data.clone()
    }

    pub fn get_stats(&self) -> HashMap<String, i32> {
        let data = self.data.lock().unwrap();
        let mut stats = HashMap::new();
        stats.insert("total_items".to_string(), data.len() as i32);
        stats.insert("name_length".to_string(), self.name.len() as i32);
        stats
    }
}

// Processor trait
pub trait Processor {
    async fn process_data(&self, items: Vec<i32>) -> Result<(), Box<dyn std::error::Error>>;
    fn get_data(&self) -> Vec<i32>;
}

impl Processor for DataProcessor {
    async fn process_data(&self, items: Vec<i32>) -> Result<(), Box<dyn std::error::Error>> {
        self.process_data(items).await
    }

    fn get_data(&self) -> Vec<i32> {
        self.get_data()
    }
}

// Simple data struct
#[derive(Debug, Clone)]
pub struct SimpleData {
    pub id: i32,
    pub name: String,
}

impl SimpleData {
    pub fn new(id: i32, name: String) -> Self {
        Self { id, name }
    }
}

// Enum for processor types
#[derive(Debug, Clone)]
pub enum ProcessorType {
    Basic,
    Advanced,
    Custom(String),
}

// Top-level functions
pub fn create_sample_data() -> Vec<SimpleData> {
    vec![
        SimpleData::new(1, "item1".to_string()),
        SimpleData::new(2, "item2".to_string()),
        SimpleData::new(3, "item3".to_string()),
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let processor = DataProcessor::create_default();
    let data = vec![1, 2, 3, 4, 5];

    processor.process_data(data).await?;

    let result = processor.get_data();
    println!("Processed data: {:?}", result);

    let stats = processor.get_stats();
    for (key, value) in stats {
        println!("{}: {}", key, value);
    }

    let sample_data = create_sample_data();
    println!("Sample data: {:?}", sample_data);

    Ok(())
}