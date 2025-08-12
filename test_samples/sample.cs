using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace TestSample
{
    public class DataProcessor
    {
        private readonly string _name;
        private readonly List<int> _data;

        public DataProcessor(string name)
        {
            _name = name;
            _data = new List<int>();
        }

        public static DataProcessor CreateDefault()
        {
            return new DataProcessor("default");
        }

        public async Task ProcessDataAsync(IEnumerable<int> items)
        {
            foreach (var item in items)
            {
                await ProcessItemAsync(item);
            }
        }

        private async Task ProcessItemAsync(int item)
        {
            await Task.Delay(1); // Simulate async work
            _data.Add(item * 2);
        }

        public IReadOnlyList<int> GetData()
        {
            return _data.AsReadOnly();
        }

        public Dictionary<string, object> GetStats()
        {
            return new Dictionary<string, object>
            {
                ["name"] = _name,
                ["total_items"] = _data.Count,
                ["last_update"] = DateTime.Now
            };
        }
    }

    public interface IProcessor
    {
        Task ProcessDataAsync(IEnumerable<int> items);
        IReadOnlyList<int> GetData();
    }

    public class AdvancedProcessor : DataProcessor, IProcessor
    {
        public AdvancedProcessor() : base("advanced")
        {
        }

        public new async Task ProcessDataAsync(IEnumerable<int> items)
        {
            Console.WriteLine("Advanced processing...");
            await base.ProcessDataAsync(items);
        }
    }

    public struct SimpleData
    {
        public int Id { get; set; }
        public string Name { get; set; }

        public SimpleData(int id, string name)
        {
            Id = id;
            Name = name;
        }
    }

    public enum ProcessorType
    {
        Basic,
        Advanced,
        Custom
    }

    // Top-level program
    public class Program
    {
        public static async Task Main(string[] args)
        {
            var processor = DataProcessor.CreateDefault();
            var data = new[] { 1, 2, 3, 4, 5 };

            await processor.ProcessDataAsync(data);

            var result = processor.GetData();
            Console.WriteLine($"Processed data: [{string.Join(", ", result)}]");

            var stats = processor.GetStats();
            foreach (var kvp in stats)
            {
                Console.WriteLine($"{kvp.Key}: {kvp.Value}");
            }
        }
    }
}