window.BENCHMARK_DATA = {
  "lastUpdate": 1755755477346,
  "repoUrl": "https://github.com/allthingsgenome/rust-fastq-parser",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "cguzman@allthingsgenome.com",
            "name": "Carlos Guzman",
            "username": "allthingsgenome"
          },
          "committer": {
            "email": "cguzman@allthingsgenome.com",
            "name": "Carlos Guzman",
            "username": "allthingsgenome"
          },
          "distinct": true,
          "id": "047d7674e14924d643a743a2f378c16c55277eff",
          "message": "update: added write permissions to github pages",
          "timestamp": "2025-08-21T05:48:06Z",
          "tree_id": "55ae3678e33588f04102b8b2ee6e9f083a902a09",
          "url": "https://github.com/allthingsgenome/rust-fastq-parser/commit/047d7674e14924d643a743a2f378c16c55277eff"
        },
        "date": 1755755476896,
        "tool": "cargo",
        "benches": [
          {
            "name": "basic_parser/parse_10k_records",
            "value": 1348726,
            "range": "± 30434",
            "unit": "ns/iter"
          },
          {
            "name": "parallel_parser/parallel_parse_10k",
            "value": 2620454,
            "range": "± 22161",
            "unit": "ns/iter"
          },
          {
            "name": "mmap_reader/mmap_10k_records",
            "value": 2308342,
            "range": "± 71419",
            "unit": "ns/iter"
          },
          {
            "name": "simd/find_newlines",
            "value": 49650,
            "range": "± 2542",
            "unit": "ns/iter"
          },
          {
            "name": "simd/validate_ascii",
            "value": 12,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "simd/count_chars",
            "value": 17314,
            "range": "± 44",
            "unit": "ns/iter"
          },
          {
            "name": "memory_scaling/parse_1000_records",
            "value": 133808,
            "range": "± 640",
            "unit": "ns/iter"
          },
          {
            "name": "memory_scaling/parse_10000_records",
            "value": 1344902,
            "range": "± 38756",
            "unit": "ns/iter"
          },
          {
            "name": "memory_scaling/parse_100000_records",
            "value": 13507432,
            "range": "± 236230",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}